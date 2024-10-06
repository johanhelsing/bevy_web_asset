use std::sync::{Arc, RwLock};

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
    data: WebAssetReaderDataInner,
}

impl WebAssetPlugin {
    /// Headers will be passed along with each request
    pub fn new(data: WebAssetReaderDataInner) -> Self {
        Self { data }
    }

    /// Enable "fake extension". This turns "test/example..png" into "test/example", but leaves single dots alone.
    pub fn enable_fake_extensions(mut self) -> Self {
        self.data.fake_extensions = true;
        self
    }

    /// Push a new header to be sent along every asset load. The same key can be pushed multiple times.
    pub fn push_header(mut self, key: impl ToString, value: impl ToString) -> Self {
        self.data
            .headers
            .entry(key.to_string())
            .or_insert_with(Vec::new)
            .push(value.to_string());
        self
    }

    /// Push a query parameter, which will be appended to the reqeust before its sent
    pub fn push_query(mut self, key: impl ToString, value: impl ToString) -> Self {
        self.data.query.insert(key.to_string(), value.to_string());
        self
    }
}

impl Plugin for WebAssetPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WebAssetReaderData {
            data: Arc::new(RwLock::new(self.data.clone())),
        });

        // seems to be the best way to resolve borrow/move issues here with the closures
        let (weak_a, weak_b) = {
            let res = app.world().resource::<WebAssetReaderData>();
            (Arc::downgrade(&res.data), Arc::downgrade(&res.data))
        };

        app.register_asset_source(
            "http",
            AssetSource::build().with_reader(move || {
                Box::new(WebAssetReader {
                    protocol: Protocol::Http,
                    shared: weak_a.upgrade().unwrap(),
                })
            }),
        );

        app.register_asset_source(
            "https",
            AssetSource::build().with_reader(move || {
                Box::new(WebAssetReader {
                    protocol: Protocol::Https,
                    shared: weak_b.upgrade().unwrap(),
                })
            }),
        );
    }
}
