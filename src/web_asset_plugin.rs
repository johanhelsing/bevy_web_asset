use bevy::asset::FileAssetIo;
use bevy::prelude::*;
use std::sync::{Arc, RwLock};

use crate::FilesystemWatcher;

use super::WebAssetIo;

/// Add this plugin to bevy to support loading http and https urls.
///
/// Needs to be added before Bevy's `DefaultPlugins`.
/// Also, make sure `AssetPlugin` is not loaded through `DefaultPlugins`.
///
/// # Example
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use bevy_web_asset::WebAssetPlugin;
///
/// let mut app = App::new();
/// // The web asset plugin should be added instead of the `AssetPlugin`
/// // Internally, WebAssetPlugin will create an AssetPlugin and hook into
/// // it in the right places
/// app.add_plugin(WebAssetPlugin::default());
/// app.add_plugins(DefaultPlugins.build().disable::<AssetPlugin>());
/// ```
///});
pub struct WebAssetPlugin {
    /// The asset folder, relative to the binary.
    pub asset_folder: String,
    /// Whether to use `FileAssetIo`-level hot reloading.
    pub watch_for_changes: bool,
}

impl Default for WebAssetPlugin {
    fn default() -> Self {
        let inner_default = AssetPlugin::default();

        Self {
            asset_folder: inner_default.asset_folder,
            watch_for_changes: inner_default.watch_for_changes,
        }
    }
}

impl Plugin for WebAssetPlugin {
    fn build(&self, app: &mut App) {
        // First, configure the underlying plugin
        // We use out own watcher, so `watch_for_changes` is always false
        let asset_plugin = AssetPlugin {
            asset_folder: self.asset_folder.clone(),
            watch_for_changes: false,
        };

        // Create the `FileAssetIo` wrapper
        let asset_io = {
            // This makes calling `WebAssetIo::watch_for_changes` redundant
            let filesystem_watcher = match self.watch_for_changes {
                true => Arc::new(RwLock::new(Some(FilesystemWatcher::default()))),
                false => Arc::new(RwLock::new(None)),
            };

            // Create the `FileAssetIo`
            let default_io = asset_plugin.create_platform_default_asset_io();

            // The method doesn't change, so we just use `FileAssetIo`'s
            let root_path = FileAssetIo::get_base_path().join(&self.asset_folder);

            WebAssetIo {
                default_io,
                root_path,
                filesystem_watcher,
            }
        };

        // Add the asset server with our `WebAssetIo` wrapping `FileAssetIo`
        app.insert_resource(AssetServer::new(asset_io));

        // Add the asset plugin
        app.add_plugin(asset_plugin);

        // Optionally add the watcher system
        if self.watch_for_changes {
            app.add_system_to_stage(
                bevy::asset::AssetStage::LoadAssets,
                super::web_filesystem_watcher::filesystem_watcher_system,
            );
        }
    }
}
