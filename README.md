# Bevy Web Asset

This is a tiny crate that that wraps the standard bevy asset loader, and adds
the ability to load assets from http and https urls.

If asset paths start with http:// or https://, then we try to do a web request
to load the asset, otherwise, we just call the normal asset io.

This is nice if you want to keep your content on a server, even when developing
native games. Use cases can be:

- Tuning game balance post-launch
- Seasonal events (halloween theme etc.)
- Downloading dynamic content from 3rd party services (lospec, gltf repositories etc.)
- Sharing user-created assets/mods over some service (level editor etc.)
- Keeping initial download size small

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