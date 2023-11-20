use bevy::prelude::*;
use bevy_web_asset::WebAssetPlugin;

fn main() {
    App::new()
        // The web asset plugin must be inserted before the `AssetPlugin` so
        // that the AssetPlugin recognizes the new sources.
        .add_plugins(WebAssetPlugin::default())
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn(SpriteBundle {
        // Simply use a url where you would normally use an asset folder relative path
        texture: asset_server.load("https://s3.johanhelsing.studio/dump/favicon.png"),
        ..default()
    });
}
