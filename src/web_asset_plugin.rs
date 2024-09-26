use bevy::{prelude::*, utils::HashMap};

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
    headers: HashMap<String, Vec<String>>,
    query: HashMap<String, String>,
    fake_extension: bool,
}

impl WebAssetPlugin {
    /// Headers will be passed along with each request
    pub fn new(headers: HashMap<String, Vec<String>>, query: HashMap<String, String>) -> Self {
        Self {
            headers,
            query,
            fake_extension: false,
        }
    }

    /// Enable "fake extension". This turns "test/example..png" into "test/example", but leaves single dots alone.
    pub fn enable_fake_extensions(mut self) -> Self {
        self.fake_extension = true;
        self
    }

    /// Push a new header to be sent along every asset load. The same key can be pushed multiple times.
    pub fn push_header(mut self, key: impl ToString, value: impl ToString) -> Self {
        self.headers
            .entry(key.to_string())
            .or_insert_with(Vec::new)
            .push(value.to_string());
        self
    }

    /// Push a query parameter, which will be appended to the reqeust before its sent
    pub fn push_query(mut self, key: impl ToString, value: impl ToString) -> Self {
        self.query.insert(key.to_string(), value.to_string());
        self
    }
}

impl Plugin for WebAssetPlugin {
    fn build(&self, app: &mut App) {
        let headers = self.headers.clone();
        let query = self.query.clone();
        let fake_extension = self.fake_extension;
        app.register_asset_source(
            "http",
            AssetSource::build().with_reader(move || {
                Box::new(WebAssetReader {
                    protocol: Protocol::Http,
                    headers: headers.clone(),
                    query: query.clone(),
                    fake_extensions: fake_extension,
                })
            }),
        );

        let query = self.query.clone();
        let headers = self.headers.clone();
        app.register_asset_source(
            "https",
            AssetSource::build().with_reader(move || {
                Box::new(WebAssetReader {
                    protocol: Protocol::Https,
                    headers: headers.clone(),
                    query: query.clone(),
                    fake_extensions: fake_extension,
                })
            }),
        );
    }
}
