use bevy::{
    asset::io::PathStream,
    utils::{ConditionalSendFuture, HashMap},
};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use bevy::asset::io::{AssetReader, AssetReaderError, Reader};

/// Which protocol to use
pub enum Protocol {
    /// Unencrypted connections.
    Http,
    /// Use TLS for setting up connections.
    Https,
}

/// Treats paths as urls to load assets from.
pub struct WebAssetReader {
    /// The protocol whith which the request is sent
    pub protocol: Protocol,
    /// Headers will be passed along with each request
    pub headers: HashMap<String, Vec<String>>,
    /// Query parameters will be passed along with each request
    pub query: HashMap<String, String>,
    /// Fake extensions are those with 2 dots. They will be removed before sending the request.
    pub fake_extensions: bool,
}

fn strip_double_extension(path: &mut PathBuf) -> Option<()> {
    let fname = path.file_name()?.to_str()?;
    let ext_start = fname.len() - path.extension()?.len();

    if &fname[ext_start - 2..ext_start] == ".." {
        path.set_extension("");
        path.set_extension("");
        Some(())
    } else {
        Some(())
    }
}

impl WebAssetReader {
    fn make_header_iter(&self) -> impl Iterator<Item = (&str, &[String])> {
        self.headers.iter().map(|(k, v)| (k.as_str(), v.as_slice()))
    }

    fn make_uri(&self, path: &Path) -> PathBuf {
        let mut buf = PathBuf::from(match self.protocol {
            Protocol::Http => "http://",
            Protocol::Https => "https://",
        })
        .join(path);
        if self.fake_extensions {
            strip_double_extension(&mut buf);
        }
        buf
    }

    fn make_uri_query(&self, path: &Path) -> PathBuf {
        let mut buf = self.make_uri(path);
        let mut query = self.query.iter();
        let mut query_string = String::new();
        if let Some((query_k, val)) = query.next() {
            query_string += &format!("?{query_k}={val}");
        }

        for (query_k, val) in query {
            query_string += &format!("&{query_k}={val}");
        }
        buf.push(query_string);
        buf
    }

    /// See [bevy::asset::io::get_meta_path]
    fn make_meta_uri(&self, path: &Path) -> Option<PathBuf> {
        path.extension()?;
        let mut uri = self.make_uri(path);
        let mut fname = uri.file_name()?.to_os_string();
        fname.push(".meta");
        uri.set_file_name(fname);
        Some(uri)
    }
}

#[cfg(target_arch = "wasm32")]
async fn get<'a>(
    path: PathBuf,
    headers: impl Iterator<Item = (&str, &[String])>,
) -> Result<Box<Reader<'a>>, AssetReaderError> {
    use bevy::asset::io::VecReader;
    use js_sys::Uint8Array;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, RequestMode, Response};

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
    let mut init = RequestInit::new();
    init.set_mode(RequestMode::Cors);
    let request = Request::new_with_str_and_init(path.to_str().unwrap(), &init).unwrap();
    let request_headers = request.headers();
    for (header_name, header_values) in headers {
        for header_value in header_values {
            request_headers
                .append(header_name, header_value.as_str())
                .map_err(js_value_to_err("append header"))?;
        }
    }
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
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
async fn get<'a>(
    path: PathBuf,
    headers: impl Iterator<Item = (&str, &[String])>,
) -> Result<Box<Reader<'a>>, AssetReaderError> {
    use std::future::Future;
    use std::io;
    use std::pin::Pin;
    use std::str::FromStr;
    use std::task::{Context, Poll};

    use bevy::asset::io::VecReader;
    use surf::http::headers::{HeaderValue, HeaderValues};
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

    let str_path = path
        .to_str()
        .ok_or_else(|| {
            AssetReaderError::Io(
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("non-utf8 path: {}", path.display()),
                )
                .into(),
            )
        })?
        .to_string();

    let mut request = surf::get(str_path);

    // From headers iter to surf headers
    for (header_name, header_values) in headers {
        let hvs: Result<HeaderValues, _> = header_values
            .iter()
            .map(|f| {
                HeaderValue::from_str(f).map_err(|_| {
                    AssetReaderError::Io(
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Header values for {} should be ASCII", header_name),
                        )
                        .into(),
                    )
                })
            })
            .collect();
        request = request.header(header_name, &hvs?);
    }

    let mut response = ContinuousPoll(request).await.map_err(|err| {
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

impl AssetReader for WebAssetReader {
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl ConditionalSendFuture<Output = Result<Box<Reader<'a>>, AssetReaderError>> {
        get(self.make_uri_query(path), self.make_header_iter())
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<Box<Reader<'a>>, AssetReaderError> {
        match self.make_meta_uri(path) {
            Some(uri) => get(uri, self.make_header_iter()).await,
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
        Err(AssetReaderError::NotFound(self.make_uri(path)))
    }
}

#[cfg(test)]
mod tests {
    use bevy::utils::default;

    use super::*;

    #[test]
    fn make_http_uri() {
        assert_eq!(
            WebAssetReader {
                protocol: Protocol::Http,
                headers: default(),
                query: default(),
                fake_extensions: true
            }
            .make_uri(Path::new("s3.johanhelsing.studio/dump/favicon.png"))
            .to_str()
            .unwrap(),
            "http://s3.johanhelsing.studio/dump/favicon.png"
        );
    }

    #[test]
    fn make_http_uri_strip_fake() {
        assert_eq!(
            WebAssetReader {
                protocol: Protocol::Http,
                headers: default(),
                query: default(),
                fake_extensions: true
            }
            .make_uri(Path::new("s3.johanhelsing.studio/dump/favicon..png"))
            .to_str()
            .unwrap(),
            "http://s3.johanhelsing.studio/dump/favicon"
        );
    }

    #[test]
    fn make_https_uri() {
        assert_eq!(
            WebAssetReader {
                protocol: Protocol::Https,
                headers: default(),
                query: default(),
                fake_extensions: true,
            }
            .make_uri(Path::new("s3.johanhelsing.studio/dump/favicon.png"))
            .to_str()
            .unwrap(),
            "https://s3.johanhelsing.studio/dump/favicon.png"
        );
    }

    #[test]
    fn make_http_meta_uri() {
        assert_eq!(
            WebAssetReader {
                protocol: Protocol::Http,
                headers: default(),
                query: default(),
                fake_extensions: true,
            }
            .make_meta_uri(Path::new("s3.johanhelsing.studio/dump/favicon.png"))
            .expect("cannot create meta uri")
            .to_str()
            .unwrap(),
            "http://s3.johanhelsing.studio/dump/favicon.png.meta"
        );
    }

    #[test]
    fn make_http_meta_uri_strip_fake() {
        assert_eq!(
            WebAssetReader {
                protocol: Protocol::Http,
                headers: default(),
                query: default(),
                fake_extensions: true,
            }
            .make_meta_uri(Path::new("s3.johanhelsing.studio/dump/favicon..png"))
            .expect("cannot create meta uri")
            .to_str()
            .unwrap(),
            "http://s3.johanhelsing.studio/dump/favicon.meta"
        );
    }

    #[test]
    fn make_https_meta_uri() {
        assert_eq!(
            WebAssetReader {
                protocol: Protocol::Https,
                headers: default(),
                query: default(),
                fake_extensions: true,
            }
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
            WebAssetReader {
                protocol: Protocol::Https,
                headers: default(),
                query: default(),
                fake_extensions: true,
            }
            .make_meta_uri(Path::new("s3.johanhelsing.studio/dump/favicon")),
            None
        );
    }
}
