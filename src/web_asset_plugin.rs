use bevy::prelude::*;

use crate::web_asset_source::*;
use bevy::asset::io::AssetSource;

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
///
/// app.add_plugins((
///     WebAssetPlugin::default(),
///     DefaultPlugins
/// ));
/// ```
///});
#[derive(Default)]
pub struct WebAssetPlugin;

impl Plugin for WebAssetPlugin {
    fn build(&self, app: &mut App) {
        app.register_asset_source(
            "http",
            AssetSource::build().with_reader(|| Box::new(WebAssetReader::Http)),
        );
        app.register_asset_source(
            "https",
            AssetSource::build().with_reader(|| Box::new(WebAssetReader::Https)),
        );
    }
}
