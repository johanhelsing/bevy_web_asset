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
#[derive(Default)]
pub struct WebAssetPlugin {
    /// User agent to tell the server about. Some servers require this to be set.
    /// Note: This flag is entirely ignored on wasm, as only firefox reliably supports this.
    pub user_agent: Option<String>,
}

impl Plugin for WebAssetPlugin {
    fn build(&self, app: &mut App) {
        for (id, kind) in [
            ("http", WebAssetReaderKind::Http),
            ("https", WebAssetReaderKind::Https),
        ] {
            let user_agent = self.user_agent.clone();
            app.register_asset_source(
                id,
                AssetSource::build().with_reader(move || {
                    Box::new(WebAssetReader {
                        user_agent: user_agent.clone(),
                        kind,
                    })
                }),
            );
        }
    }
}
