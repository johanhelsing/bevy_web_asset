use bevy::{asset::io::PathStream, utils::BoxedFuture};
use std::path::{Path, PathBuf};

use bevy::asset::io::{AssetReader, AssetReaderError, Reader, VecReader};

// Note: Bevy does not retain the asset source identifier (http/https)
// so we need to pass the protocol manually.
pub(super) enum WebAssetReader {
    Http,
    Https,
}

impl WebAssetReader {
    fn make_uri(&self, path: &Path) -> PathBuf {
        PathBuf::from(match self {
            Self::Http => "http://",
            Self::Https => "https://",
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
}

#[cfg(target_arch = "wasm32")]
async fn get<'a>(path: PathBuf) -> Result<Box<Reader<'a>>, AssetReaderError> {
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
        status => Err(AssetReaderError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Encountered unexpected HTTP status {status}"),
        ))),
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn get<'a>(uri: PathBuf) -> Result<Box<Reader<'a>>, AssetReaderError> {
    use ehttp::{fetch, Request};
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::mpsc::{channel, Receiver};
    use std::task::{Context, Poll};

    let uri_str = uri.to_string_lossy();
    bevy::prelude::info!("fetching {uri_str}");
    let (sender, receiver) = channel();
    fetch(Request::get(uri_str), move |result| {
        bevy::prelude::info!("callback");
        use std::io::{Error, ErrorKind};
        sender
            .send(
                result
                    .map_err(|e| AssetReaderError::Io(Error::new(ErrorKind::Other, e)))
                    .and_then(|response| match response.status {
                        200 => Ok(response.bytes),
                        404 => Err(AssetReaderError::NotFound(uri)),
                        _ => Err(AssetReaderError::Io(Error::from(ErrorKind::Other))),
                    }),
            )
            .unwrap();
    });

    struct AsyncReceiver<T>(Receiver<T>);
    impl<T> Future for AsyncReceiver<T> {
        type Output = T;
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            match self.0.try_recv() {
                Err(_) => {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
                Ok(t) => {
                    bevy::prelude::info!("something");
                    Poll::Ready(t)
                }
            }
        }
    }

    let bytes = AsyncReceiver(receiver).await?;
    let reader: Box<Reader> = Box::new(VecReader::new(bytes));
    Ok(reader)
}

impl AssetReader for WebAssetReader {
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        Box::pin(get(self.make_uri(path)))
    }

    fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        Box::pin(get(self.make_meta_uri(path)))
    }

    fn is_directory<'a>(
        &'a self,
        _path: &'a Path,
    ) -> BoxedFuture<'a, Result<bool, AssetReaderError>> {
        Box::pin(async move { Ok(false) })
    }

    fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<PathStream>, AssetReaderError>> {
        Box::pin(async move { Err(AssetReaderError::NotFound(self.make_uri(path))) })
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
