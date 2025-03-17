use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_vrma::vrm::loader::VrmHandle;
use bevy_vrma::vrm::VrmPlugin;
use bevy_vrma::vrma::animation::player::PlayAnimation;
use bevy_vrma::vrma::{VrmaHandle, VrmaPlugin};
use std::time::Duration;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            WorldInspectorPlugin::default(),
            VrmPlugin,
            VrmaPlugin,
        ))
        .init_resource::<Animations>()
        .add_systems(Startup, (spawn_camera, spawn_vrm))
        .add_systems(
            Update,
            change_animation.run_if(on_timer(Duration::from_secs(5))),
        )
        .run();
}

#[derive(Default, Resource)]
struct Animations {
    current_index: usize,
    animations: Vec<Entity>,
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera3d::default(), Transform::from_xyz(0., 1., 2.5)));
}

fn spawn_vrm(
    mut commands: Commands,
    mut animations: ResMut<Animations>,
    asset_server: Res<AssetServer>,
) {
    let rotate = commands
        .spawn(VrmaHandle(asset_server.load("vrma/rotate.vrma")))
        .id();
    commands
        .spawn(VrmHandle(asset_server.load("models/sample.vrm")))
        .add_child(rotate);
    animations.animations.push(rotate);
}

fn change_animation(mut commands: Commands, mut animations: ResMut<Animations>) {
    if animations.animations.is_empty() {
        return;
    }
    let next = (animations.current_index + 1) % animations.animations.len();
    commands
        .entity(animations.animations[next])
        .trigger(PlayAnimation { repeat: true });
    animations.current_index = next;
}
