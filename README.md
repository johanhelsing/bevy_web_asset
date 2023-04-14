# Bevy Web Asset

[![crates.io](https://img.shields.io/crates/v/bevy_web_asset.svg)](https://crates.io/crates/bevy_web_asset)
![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)
[![crates.io](https://img.shields.io/crates/d/bevy_web_asset.svg)](https://crates.io/crates/bevy_web_asset)
[![docs.rs](https://img.shields.io/docsrs/bevy_web_asset)](https://docs.rs/bevy_web_asset)

This is a tiny crate that that wraps the standard bevy asset loader, and adds
the ability to load assets from http and https urls.

Supports both wasm (web-sys) and native.

If asset paths start with http:// or https://, then we try to do a web request
to load the asset, otherwise, we just call the normal asset io.

This is nice if you want to keep your content on a server, even when developing
native games. Use cases can be:

- Tuning game balance post-launch
- Seasonal events (halloween theme etc.)
- Downloading dynamic content from 3rd party services (lospec, gltf repositories etc.)
- Sharing user-created assets/mods over some service (level editor etc.)
- Keeping initial download size small
- Testing with different online assets during development

## Usage

NOTE: You need to add the plugin instead of `AssetPlugin` and before `DefaultPlugins`:

Bevy 0.7 users, see [the 0.3.0 readme](https://github.com/johanhelsing/bevy_web_asset/tree/v0.3.0) for how to add the plugin.

Bevy 0.8 users, see [the 0.4.0 readme](https://github.com/johanhelsing/bevy_web_asset/tree/v0.4.0) for how to add the plugin.

```rust no_run
use bevy::prelude::*;
use bevy_web_asset::WebAssetPlugin;

fn main() {
    App::new()
        // The `WebAssetPlugin` must be inserted instead of `AssetPlugin` and before `DefaultPlugins`
        .add_plugin(WebAssetPlugin::default())
        .add_plugins(DefaultPlugins.build().disable::<AssetPlugin>())
        // ...
        .run();
}
```

But using it is quite simple, just use http urls instead of regular asset paths.

```rust ignore
let font: Handle<Font> = asset_server.load("https://example.com/fonts/quicksand-light.ttf");
```

Or:

```rust ignore
commands.spawn(SpriteBundle {
    // Simply use a url where you would normally use an asset folder relative path
    texture: asset_server.load("https://johanhelsing.studio/assets/favicon.png"),
    ..default()
});
```

# Bevy version support

I intend to support the latest bevy release in the `main` branch.

Fixes against the Bevy `main` branch goes in the the `bevy-main` branch and are
merged back into `main` whenever bevy is released.

|bevy|bevy_web_asset|
|---|---|
|main|bevy-main|
|0.9|0.5, main|
|0.8|0.4|
|0.7|0.3|
|0.6|0.2|
|0.5|0.1|

## License

`bevy_web_asset` is dual-licensed under either

- MIT License (./LICENSE-MIT or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 (./LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)

at your option.

## Contributions

PRs welcome!
