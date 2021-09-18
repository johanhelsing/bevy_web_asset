use bevy::asset::{AssetIo, AssetIoError, BoxedFuture};
use std::path::{Path, PathBuf};

/// Wraps the default bevy AssetIo and adds support for loading http urls
pub struct WebAssetIo {
    pub(crate) default_io: Box<dyn AssetIo>,
}

impl AssetIo for WebAssetIo {
    fn load_path<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<Vec<u8>, AssetIoError>> {
        if path.starts_with("http://") || path.starts_with("https://") {
            let uri = path.to_str().unwrap();

            #[cfg(target_arch = "wasm32")]
            let fut = Box::pin(async move {
                use wasm_bindgen::JsCast;
                use wasm_bindgen_futures::JsFuture;
                let window = web_sys::window().unwrap();
                let resp_value = JsFuture::from(window.fetch_with_str(uri)).await.unwrap();
                let resp: web_sys::Response = resp_value.dyn_into().unwrap();
                let data = JsFuture::from(resp.array_buffer().unwrap()).await.unwrap();
                let bytes = js_sys::Uint8Array::new(&data).to_vec();
                Ok(bytes)
            });

            #[cfg(not(target_arch = "wasm32"))]
            let fut = Box::pin(async move {
                let bytes = surf::get(uri)
                    .await
                    .map_err(|_| AssetIoError::NotFound(path.to_path_buf()))?
                    .body_bytes()
                    .await
                    .map_err(|_| AssetIoError::NotFound(path.to_path_buf()))?;

                Ok(bytes)
            });

            fut
        } else {
            self.default_io.load_path(path)
        }
    }

    fn read_directory(
        &self,
        path: &Path,
    ) -> Result<Box<dyn Iterator<Item = PathBuf>>, AssetIoError> {
        self.default_io.read_directory(path)
    }

    fn watch_path_for_changes(&self, path: &Path) -> Result<(), AssetIoError> {
        self.default_io.watch_path_for_changes(path)
    }

    fn watch_for_changes(&self) -> Result<(), AssetIoError> {
        self.default_io.watch_for_changes()
    }

    fn is_directory(&self, path: &Path) -> bool {
        self.default_io.is_directory(path)
    }
}
