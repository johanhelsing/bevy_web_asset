use bevy::prelude::*;

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
#[derive(Default)]
pub struct WebAssetPlugin {
    /// Settings for the underlying (regular) AssetPlugin
    pub asset_plugin: AssetPlugin,
}

impl Plugin for WebAssetPlugin {
    fn build(&self, app: &mut App) {
        if self.asset_plugin.watch_for_changes {
            warn!("bevy_web_asset currently breaks regular filesystem hot reloading, see https://github.com/johanhelsing/bevy_web_asset/issues/1");
        }

        let asset_io = {
            let default_io = self.asset_plugin.create_platform_default_asset_io();
            WebAssetIo { default_io }
        };

        app.insert_resource(AssetServer::new(asset_io));

        // now that we've wrapped the AssetIo in WebAssetIo, we can initialize the normal asset plugin

        // AssetPlugin doesn't implement clone, so we need to do it manually
        let asset_plugin = AssetPlugin {
            asset_folder: self.asset_plugin.asset_folder.clone(),
            watch_for_changes: self.asset_plugin.watch_for_changes,
        };

        app.add_plugin(asset_plugin);
    }
}
