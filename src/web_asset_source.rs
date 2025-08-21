use bevy::{asset::io::PathStream, tasks::ConditionalSendFuture};
use std::path::{Path, PathBuf};

use bevy::asset::io::{AssetReader, AssetReaderError, Reader};

/// Treats paths as urls to load assets from.
pub struct WebAssetReader {
    /// Option to cache resource.
    pub cache_resource: bool,
    /// Option to disable meta request (some server returns 500 if too many invalid
    /// requests are sent, to prevent bot).
    pub reject_meta_request: bool,
    /// Connection type.
    pub connection: WebAssetReaderConnection,
}

impl Default for WebAssetReader {
    fn default() -> Self {
        Self {
            cache_resource: false,
            reject_meta_request: false,
            connection: WebAssetReaderConnection::Https,
        }
    }
}

impl WebAssetReader {
    #[cfg(feature = "cache_asset")]
    fn get_cache_path(&self, path: &Path) -> Option<PathBuf> {
        use slug::slugify;

        if self.cache_resource {
            return directories::ProjectDirs::from("", "", "bevy_web_asset").map(|user_dirs| {
                // extract the directory part of the path, and if it doesn't exist, use the path itself
                let url_dir = path.parent().unwrap_or(path).to_string_lossy();

                // Extract the last component of the path, to use as the filename
                let url_filename = path
                    .file_name()
                    .map(|filename| filename.to_string_lossy())
                    .unwrap_or(std::borrow::Cow::Borrowed("filename"))
                    .to_string();

                // Build the final path by combining cache directory, slug, and filename
                user_dirs
                    .cache_dir()
                    .join(slugify(url_dir))
                    .join(url_filename)
            });
        }
        None
    }

    #[cfg(not(feature = "cache_asset"))]
    fn get_cache_path(&self, _: &Path) -> Option<PathBuf> {
        None
    }
}

/// Treats paths as urls to load assets from.
pub enum WebAssetReaderConnection {
    /// Unencrypted connections.
    Http,
    /// Use TLS for setting up connections.
    Https,
}

impl WebAssetReaderConnection {
    fn make_uri(&self, path: &Path) -> PathBuf {
        PathBuf::from(match self {
            WebAssetReaderConnection::Http => "http://",
            WebAssetReaderConnection::Https => "https://",
        })
        .join(path)
    }

    /// See [bevy::asset::io::get_meta_path]
    fn make_meta_uri(&self, path: &Path) -> Option<PathBuf> {
        let mut uri = self.make_uri(path);
        let mut extension = path.extension()?.to_os_string();
        extension.push(".meta");
        uri.set_extension(extension);
        Some(uri)
    }
}

#[cfg(target_arch = "wasm32")]
async fn get(path: PathBuf, _: Option<PathBuf>) -> Result<Box<dyn Reader>, AssetReaderError> {
    use bevy::asset::io::VecReader;
    use js_sys::Uint8Array;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::Response;

    fn js_value_to_err<'a>(
        context: &'a str,
    ) -> impl FnOnce(wasm_bindgen::JsValue) -> std::io::Error + 'a {
        move |value| {
            let message = match js_sys::JSON::stringify(&value) {
                Ok(js_str) => format!("Failed to {context}: {js_str}"),
                Err(_) => {
                    format!(
                        "Failed to {context} and also failed to stringify the JSValue of the error"
                    )
                }
            };

            std::io::Error::new(std::io::ErrorKind::Other, message)
        }
    }

    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_str(path.to_str().unwrap()))
        .await
        .map_err(js_value_to_err("fetch path"))?;
    let resp = resp_value
        .dyn_into::<Response>()
        .map_err(js_value_to_err("convert fetch to Response"))?;
    match resp.status() {
        200 => {
            let data = JsFuture::from(resp.array_buffer().unwrap()).await.unwrap();
            let bytes = Uint8Array::new(&data).to_vec();
            let reader: Box<dyn Reader> = Box::new(VecReader::new(bytes));
            Ok(reader)
        }
        404 => Err(AssetReaderError::NotFound(path)),
        status => Err(AssetReaderError::Io(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Encountered unexpected HTTP status {status}"),
            )
            .into(),
        )),
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn get(
    path: PathBuf,
    cache_path: Option<PathBuf>,
) -> Result<Box<dyn Reader>, AssetReaderError> {
    use std::fs;
    use std::future::Future;
    use std::io;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    if let Some(cache_path) = cache_path.as_ref() {
        if cache_path.exists() {
            // TODO: fallback to deleting cache if it fails to read, and re-download the file?
            // Currently user can delete the cache manually to trigger a re-download.
            return Ok(Box::new(VecReader::new(fs::read(cache_path)?)));
        }
    }

    use bevy::asset::io::VecReader;
    use surf::StatusCode;

    #[pin_project::pin_project]
    struct ContinuousPoll<T>(#[pin] T);

    impl<T: Future> Future for ContinuousPoll<T> {
        type Output = T::Output;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            // Always wake - blocks on single threaded executor.
            cx.waker().wake_by_ref();

            self.project().0.poll(cx)
        }
    }

    let str_path = path.to_str().ok_or_else(|| {
        AssetReaderError::Io(
            io::Error::new(
                io::ErrorKind::Other,
                format!("non-utf8 path: {}", path.display()),
            )
            .into(),
        )
    })?;

    #[cfg(not(feature = "redirect"))]
    let client = surf::Client::new();

    #[cfg(feature = "redirect")]
    let client = surf::Client::new().with(surf::middleware::Redirect::default());

    let mut response = ContinuousPoll(client.get(str_path)).await.map_err(|err| {
        AssetReaderError::Io(
            io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "unexpected status code {} while loading {}: {}",
                    err.status(),
                    path.display(),
                    err.into_inner(),
                ),
            )
            .into(),
        )
    })?;

    match response.status() {
        StatusCode::Ok => {
            let buf = ContinuousPoll(response.body_bytes())
                .await
                .map_err(|_| AssetReaderError::NotFound(path.to_path_buf()))?;

            #[cfg(feature = "cache_asset")]
            if let Some(cache_path) = cache_path {
                use std::io::Write;

                if let Some(parent_dirs) = cache_path.parent() {
                    fs::create_dir_all(parent_dirs)?;
                }
                let mut file = fs::OpenOptions::new()
                    .create(true)
                    .truncate(true)
                    .write(true)
                    .open(&cache_path)?;
                // write result to disk, then return the result as a file reader
                file.write_all(buf.as_slice())?;
                return Ok(Box::new(VecReader::new(fs::read(&cache_path)?)));
            }
            Ok(Box::new(VecReader::new(buf)) as _)
        }
        StatusCode::NotFound => Err(AssetReaderError::NotFound(path)),
        code => Err(AssetReaderError::Io(
            io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "unexpected status code {} while loading {}",
                    code,
                    path.display()
                ),
            )
            .into(),
        )),
    }
}

impl AssetReader for WebAssetReader {
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl ConditionalSendFuture<Output = Result<Box<dyn Reader>, AssetReaderError>> {
        let uri = self.connection.make_uri(path);

        let cache_path = self.get_cache_path(&uri);
        get(uri, cache_path)
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<Box<dyn Reader>, AssetReaderError> {
        if self.reject_meta_request {
            // see https://github.com/johanhelsing/bevy_web_asset/issues/28
            // too many request made to .meta can cause issue
            return Err(AssetReaderError::NotFound("meta request rejected".into()));
        }

        match self.connection.make_meta_uri(path) {
            Some(uri) => {
                let cache_path = self.get_cache_path(&uri);
                match get(uri, cache_path).await {
                    Ok(reader) => Ok(reader),
                    Err(err) => Err(AssetReaderError::NotFound(
                        format!("Error loading meta: {err}").into(),
                    )),
                }
            }
            None => Err(AssetReaderError::NotFound(
                "source path has no extension".into(),
            )),
        }
    }

    async fn is_directory<'a>(&'a self, _path: &'a Path) -> Result<bool, AssetReaderError> {
        Ok(false)
    }

    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        Err(AssetReaderError::NotFound(self.connection.make_uri(path)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_http_uri() {
        assert_eq!(
            WebAssetReaderConnection::Http
                .make_uri(Path::new("s3.johanhelsing.studio/dump/favicon.png"))
                .to_str()
                .unwrap(),
            "http://s3.johanhelsing.studio/dump/favicon.png"
        );
    }

    #[test]
    fn make_https_uri() {
        assert_eq!(
            WebAssetReaderConnection::Https
                .make_uri(Path::new("s3.johanhelsing.studio/dump/favicon.png"))
                .to_str()
                .unwrap(),
            "https://s3.johanhelsing.studio/dump/favicon.png"
        );
    }

    #[test]
    fn make_http_meta_uri() {
        assert_eq!(
            WebAssetReaderConnection::Http
                .make_meta_uri(Path::new("s3.johanhelsing.studio/dump/favicon.png"))
                .expect("cannot create meta uri")
                .to_str()
                .unwrap(),
            "http://s3.johanhelsing.studio/dump/favicon.png.meta"
        );
    }

    #[test]
    fn make_https_meta_uri() {
        assert_eq!(
            WebAssetReaderConnection::Https
                .make_meta_uri(Path::new("s3.johanhelsing.studio/dump/favicon.png"))
                .expect("cannot create meta uri")
                .to_str()
                .unwrap(),
            "https://s3.johanhelsing.studio/dump/favicon.png.meta"
        );
    }

    #[test]
    fn make_https_without_extension_meta_uri() {
        assert_eq!(
            WebAssetReaderConnection::Https
                .make_meta_uri(Path::new("s3.johanhelsing.studio/dump/favicon")),
            None
        );
    }
}
