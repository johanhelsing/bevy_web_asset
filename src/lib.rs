#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

mod web_asset_io;
mod web_asset_plugin;
mod web_filesystem_watcher;

pub use web_asset_io::WebAssetIo;
pub use web_asset_plugin::WebAssetPlugin;
pub use web_filesystem_watcher::FilesystemWatcher;
