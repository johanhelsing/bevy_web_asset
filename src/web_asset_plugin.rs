use bevy::{asset::AssetServerSettings, prelude::*};

use super::WebAssetIo;

/// Add this plugin to bevy to support loading http and https urls.
///
/// Needs to be added before Bevy's `DefaultPlugins`.
///
/// # Example
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_web_asset::WebAssetPlugin;
///
/// let mut app = App::new();
///     // The web asset plugin must be inserted before the `AssetPlugin` so
///     // that the asset plugin doesn't create another instance of an asset
///     // server. In general, the AssetPlugin should still run so that other
///     // aspects of the asset system are initialized correctly.
/// app.add_plugin(WebAssetPlugin);
/// app.add_plugins(DefaultPlugins);
/// ```
///});
pub struct WebAssetPlugin;

impl Plugin for WebAssetPlugin {
    fn build(&self, app: &mut App) {
        // If configured by inserting the AssetServerSettingsResource
        let watch_for_changes_configured = app
            .world
            .get_resource::<AssetServerSettings>()
            .map(|s| s.watch_for_changes)
            .unwrap_or(false);

        if watch_for_changes_configured {
            warn!("bevy_web_asset currently breaks regular filesystem hot reloading, see https://github.com/johanhelsing/bevy_web_asset/issues/1");
        }

        let asset_io = {
            let default_io = bevy::asset::create_platform_default_asset_io(app);
            WebAssetIo { default_io }
        };

        app.insert_resource(AssetServer::new(asset_io));
    }
}
