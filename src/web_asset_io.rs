use super::FilesystemWatcher;

use bevy::asset::{AssetIo, AssetIoError, BoxedFuture};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

/// Wraps the default bevy AssetIo and adds support for loading http urls
pub struct WebAssetIo {
    pub(crate) root_path: PathBuf,
    pub(crate) default_io: Box<dyn AssetIo>,
    pub(crate) filesystem_watcher: Arc<RwLock<Option<FilesystemWatcher>>>,
}

fn is_http(path: &Path) -> bool {
    path.starts_with("http://") || path.starts_with("https://")
}

impl AssetIo for WebAssetIo {
    fn load_path<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<Vec<u8>, AssetIoError>> {
        if is_http(path) {
            let uri = path.to_str().unwrap();

            #[cfg(target_arch = "wasm32")]
            let fut = Box::pin(async move {
                use wasm_bindgen::JsCast;
                use wasm_bindgen_futures::JsFuture;
                let window = web_sys::window().unwrap();
                let response = JsFuture::from(window.fetch_with_str(uri))
                    .await
                    .map(|r| r.dyn_into::<web_sys::Response>().unwrap())
                    .map_err(|e| e.dyn_into::<js_sys::TypeError>().unwrap());

                if let Err(err) = &response {
                    warn!("Failed to fetch asset {uri}: {err:?}");
                }

                let response = response.map_err(|_| AssetIoError::NotFound(path.to_path_buf()))?;

                let data = JsFuture::from(response.array_buffer().unwrap())
                    .await
                    .unwrap();

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

    fn watch_path_for_changes(&self, _path: &Path) -> Result<(), AssetIoError> {
        if is_http(_path) {
            Ok(()) // Pretend everything is fine
        } else {
            // We can now simply use our own watcher

            let path = self.root_path.join(_path);

            let result = self.filesystem_watcher.write();

            if let Ok(mut option) = result {
                if let Some(ref mut watcher) = *option {
                    watcher
                        .watch(&path)
                        .map_err(|_error| AssetIoError::PathWatchError(path))?;
                }
            }

            Ok(())
        }
    }

    fn watch_for_changes(&self) -> Result<(), AssetIoError> {
        Ok(()) // This is done in web_asset_plugin
    }

    fn is_dir(&self, path: &Path) -> bool {
        if is_http(path) {
            false
        } else {
            self.default_io.is_dir(path)
        }
    }

    fn get_metadata(&self, path: &Path) -> Result<bevy::asset::Metadata, AssetIoError> {
        self.default_io.get_metadata(path)
    }
}
