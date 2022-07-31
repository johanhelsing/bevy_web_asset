use bevy::prelude::*;
use bevy_web_asset::WebAssetPlugin;

fn main() {
    App::new()
        // The web asset plugin must be inserted before the `AssetPlugin` so
        // that the asset plugin doesn't create another instance of an asset
        // server. In general, the AssetPlugin should still run so that other
        // aspects of the asset system are initialized correctly.
        .add_plugin(WebAssetPlugin)
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(Camera2dBundle::default());

    commands.spawn_bundle(SpriteBundle {
        // Simply use a url where you would normally use an asset folder relative path
        texture: asset_server.load("https://johanhelsing.studio/assets/favicon.png"),
        ..default()
    });
}
