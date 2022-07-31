use bevy::prelude::*;
use bevy_web_asset::WebAssetPlugin;

fn main() {
    App::new()
        .add_plugins_with(DefaultPlugins, |group| {
            // The web asset plugin must be inserted in-between the
            // `CorePlugin' and `AssetPlugin`. It needs to be after the
            // CorePlugin, so that the IO task pool has already been constructed.
            // And it must be before the `AssetPlugin` so that the asset plugin
            // doesn't create another instance of an assert server. In general,
            // the AssetPlugin should still run so that other aspects of the
            // asset system are initialized correctly.
            group.add_before::<bevy::asset::AssetPlugin, _>(WebAssetPlugin)
        })
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(Camera2dBundle::default());

    commands.spawn_bundle(SpriteBundle {
        // Simply use a url where you would normally use a asset-folder relative path
        texture: asset_server.load("https://johanhelsing.studio/assets/favicon.png"),
        ..default()
    });
}
