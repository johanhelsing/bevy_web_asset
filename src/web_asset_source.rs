use bevy::asset::io::PathStream;
use bevy::utils::BoxedFuture;
use std::path::{Path, PathBuf};

use bevy::asset::io::{AssetReader, AssetReaderError, Reader};

/// Treats paths as urls to load assets from.
pub struct WebAssetReader {
    /// Whether to use https or http
    pub kind: WebAssetReaderKind,
    /// Optional user agent, some servers (e.g. openstreetmap) will reject connections without user agents
    pub user_agent: Option<String>,
}

#[derive(Copy, Clone)]
/// Whether to use https or http
pub enum WebAssetReaderKind {
    /// Unencrypted connections.
    Http,
    /// Use TLS for setting up connections.
    Https,
}

impl WebAssetReader {
    #[allow(non_upper_case_globals)]
    /// Unencrypted connections.
    pub const Http: Self = Self {
        kind: WebAssetReaderKind::Http,
        user_agent: None,
    };
    #[allow(non_upper_case_globals)]
    /// Use TLS for setting up connections.
    pub const Https: Self = Self {
        kind: WebAssetReaderKind::Https,
        user_agent: None,
    };

    fn make_uri(&self, path: &Path) -> PathBuf {
        PathBuf::from(match self.kind {
            WebAssetReaderKind::Http => "http://",
            WebAssetReaderKind::Https => "https://",
        })
        .join(path)
    }

    /// See [bevy::asset::io::get_meta_path]
    fn make_meta_uri(&self, path: &Path) -> PathBuf {
        let mut uri = self.make_uri(path);
        let mut extension = path
            .extension()
            .expect("asset paths must have extensions")
            .to_os_string();
        extension.push(".meta");
        uri.set_extension(extension);
        uri
    }

    #[cfg(target_arch = "wasm32")]
    async fn get(&self, path: PathBuf) -> Result<Box<Reader<'_>>, AssetReaderError> {
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
                let reader: Box<Reader> = Box::new(VecReader::new(bytes));
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
    async fn get(&self, path: PathBuf) -> Result<Box<Reader<'_>>, AssetReaderError> {
        use std::future::Future;
        use std::io;
        use std::pin::Pin;
        use std::task::{Context, Poll};

        use bevy::asset::io::VecReader;
        use surf::http::headers::USER_AGENT;
        use surf::{Client, Config, StatusCode};

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
        let mut config = Config::new();
        if let Some(user_agent) = &self.user_agent {
            config = config.add_header(USER_AGENT, user_agent).unwrap();
        }
        let client: Client = config.try_into().unwrap();
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
            StatusCode::Ok => Ok(Box::new(VecReader::new(
                ContinuousPoll(response.body_bytes())
                    .await
                    .map_err(|_| AssetReaderError::NotFound(path.to_path_buf()))?,
            )) as _),
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
}

impl AssetReader for WebAssetReader {
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        Box::pin(self.get(self.make_uri(path)))
    }

    fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        Box::pin(self.get(self.make_meta_uri(path)))
    }

    fn is_directory<'a>(
        &'a self,
        _path: &'a Path,
    ) -> BoxedFuture<'a, Result<bool, AssetReaderError>> {
        Box::pin(async { Ok(false) })
    }

    fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<PathStream>, AssetReaderError>> {
        Box::pin(async { Err(AssetReaderError::NotFound(self.make_uri(path))) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_http_uri() {
        assert_eq!(
            WebAssetReader::Http
                .make_uri(Path::new("s3.johanhelsing.studio/dump/favicon.png"))
                .to_str()
                .unwrap(),
            "http://s3.johanhelsing.studio/dump/favicon.png"
        );
    }

    #[test]
    fn make_https_uri() {
        assert_eq!(
            WebAssetReader::Https
                .make_uri(Path::new("s3.johanhelsing.studio/dump/favicon.png"))
                .to_str()
                .unwrap(),
            "https://s3.johanhelsing.studio/dump/favicon.png"
        );
    }

    #[test]
    fn make_http_meta_uri() {
        assert_eq!(
            WebAssetReader::Http
                .make_meta_uri(Path::new("s3.johanhelsing.studio/dump/favicon.png"))
                .to_str()
                .unwrap(),
            "http://s3.johanhelsing.studio/dump/favicon.png.meta"
        );
    }

    #[test]
    fn make_https_meta_uri() {
        assert_eq!(
            WebAssetReader::Https
                .make_meta_uri(Path::new("s3.johanhelsing.studio/dump/favicon.png"))
                .to_str()
                .unwrap(),
            "https://s3.johanhelsing.studio/dump/favicon.png.meta"
        );
    }
}
