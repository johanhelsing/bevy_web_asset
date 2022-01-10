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

Adding the plugin is little bit tricky:

```rust
.add_plugins_with(DefaultPlugins, |group| {
    // The web asset plugin must be inserted in-between the
    // `CorePlugin' and `AssetPlugin`. It needs to be after the
    // CorePlugin, so that the IO task pool has already been constructed.
    // And it must be before the `AssetPlugin` so that the asset plugin
    // doesn't create another instance of an assert server. In general,
    // the AssetPlugin should still run so that other aspects of the
    // asset system are initialized correctly.
    group.add_before::<bevy::asset::AssetPlugin, _>(WebAssetPlugin)
});
```

But using it is quite simple, just use http urls instead of regular asset paths.

```rust
let font: Handle<Font> = asset_server.load("https://example.com/fonts/quicksand-light.ttf");
```

# Bevy version support

I intend to support the latest bevy release in the `main` branch.

Fixes against the Bevy `main` branch goes in the the `bevy-main` branch and are
merged back into `main` whenever bevy is released.

|bevy|bevy_web_asset|
|---|---|
|main|bevy-main|
|0.6|0.2, main|
|0.5|0.1|