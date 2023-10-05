use bevy::prelude::*;

use super::WebAssetIo;

/// Add this plugin to bevy to support loading http and https urls.
///
/// Needs to be added before Bevy's `DefaultPlugins`.
///
/// # Example
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use bevy_web_asset::WebAssetPlugin;
///
/// let mut app = App::new();
/// app.add_plugin(WebAssetPlugin);
/// app.add_plugins(DefaultPlugins);
/// ```
///});
#[derive(Default)]
pub struct WebAssetPlugin;

impl Plugin for WebAssetPlugin {
    fn build(&self, app: &mut App) {
        let asset_io = WebAssetIo {
            default_io: AssetPlugin::default().create_platform_default_asset_io(),
        };

        app.insert_resource(AssetServer::new(asset_io));

        let temp_asset_plugin = AssetPlugin::default();
        // AssetPlugin::build does a bunch of setup that is needed for asset loading to work,
        // but those things aren't all public. Fortunately calling build won't add a new AssetServer
        // if one already exists!
        temp_asset_plugin.build(app);
    }
}
