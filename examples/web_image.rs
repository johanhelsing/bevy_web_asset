use bevy::prelude::*;
use bevy_web_asset::WebAssetPlugin;

fn main() {
    App::new()
        // The web asset plugin must be inserted before the `AssetPlugin` so
        // that the asset plugin doesn't create another instance of an asset
        // server. WebAssetPlugin will handle initialization of AssetPlugin
        // so we remove it from the default plugins group.
        .add_plugin(WebAssetPlugin::default())
        .add_plugins(DefaultPlugins.build().disable::<AssetPlugin>())
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(Camera2dBundle::default());

    commands.spawn_bundle(SpriteBundle {
        // Simply use a url where you would normally use an asset folder relative path
        texture: asset_server.load("https://s3.johanhelsing.studio/dump/favicon.png"),
        ..default()
    });
}
