use bevy::asset::io::{PathStream, VecReader};
use bevy::utils::BoxedFuture;
use std::path::{Path, PathBuf};

use bevy::asset::io::{AssetReader, AssetReaderError, Reader};

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

    #[cfg(target_arch = "wasm")]
    async fn get<'a>(&'a self, path: PathBuf) -> Result<Box<Reader<'a>>, AssetReaderError> {
        let uri_str = uri.to_str().unwrap();

        use wasm_bindgen::JsCast;
        use wasm_bindgen_futures::JsFuture;
        let window = web_sys::window().unwrap();
        let response = JsFuture::from(window.fetch_with_str(uri_str))
            .await
            .map(|r| r.dyn_into::<web_sys::Response>().unwrap())
            .map_err(|e| e.dyn_into::<js_sys::TypeError>().unwrap());

        if let Err(err) = &response {
            // warn!("Failed to fetch asset {uri_str}: {err:?}");
        }

        let response = response.map_err(|_| AssetIoError::NotFound(uri))?;

        let data = JsFuture::from(response.array_buffer().unwrap())
            .await
            .unwrap();

        let bytes = js_sys::Uint8Array::new(&data).to_vec();

        let reader: Box<Reader> = Box::new(VecReader::new(bytes));

        Ok(reader)
    }

    #[cfg(not(target_arch = "wasm"))]
    async fn get<'a>(&'a self, uri: PathBuf) -> Result<Box<Reader<'a>>, AssetReaderError> {
        let uri_str = uri.to_str().unwrap();

        let bytes = surf::get(uri_str)
            .recv_bytes()
            .await
            .map_err(|_| AssetReaderError::NotFound(uri))?;

        let reader: Box<Reader> = Box::new(VecReader::new(bytes));

        Ok(reader)
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
