#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

mod web_asset_io;
mod web_asset_plugin;

pub use web_asset_io::WebAssetIo;
pub use web_asset_plugin::WebAssetPlugin;
