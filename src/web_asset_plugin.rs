use bevy::{prelude::*, tasks::IoTaskPool};

use super::WebAssetIo;

/// Add this plugin to bevy to support loading http and https urls.
///
/// # Example
///
/// ```
/// # use bevy::prelude::*;
/// let mut app = App::build();
/// app.add_plugins_with(DefaultPlugins, |group| {
///    // The web asset plugin must be inserted in-between the
///    // `CorePlugin' and `AssetPlugin`. It needs to be after the
///    // CorePlugin, so that the IO task pool has already been constructed.
///    // And it must be before the `AssetPlugin` so that the asset plugin
///    // doesn't create another instance of an assert server. In general,
///    // the AssetPlugin should still run so that other aspects of the
///    // asset system are initialized correctly.
///    group.add_before::<bevy::asset::AssetPlugin, _>(bevy_web_asset::WebAssetPlugin)
/// });
/// ```
///});
pub struct WebAssetPlugin;

impl Plugin for WebAssetPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let task_pool = app
            .world()
            .get_resource::<IoTaskPool>()
            .expect("IoTaskPool resource not found")
            .0
            .clone();

        let asset_io = {
            let default_io = bevy::asset::create_platform_default_asset_io(app);
            WebAssetIo { default_io }
        };

        app.insert_resource(AssetServer::new(asset_io, task_pool));
    }
}
