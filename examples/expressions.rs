//! This example shows how to directly control VRM expressions from code.
//!
//! Press number keys to trigger expressions:
//! - 1: happy
//! - 2: angry
//! - 3: sad
//! - 4: blink
//! - 0: clear all expressions (return to VRMA control)

use bevy::prelude::*;
use bevy_vrm1::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, VrmPlugin))
        .add_systems(Startup, (spawn_light, spawn_camera, spawn_vrm))
        .add_systems(Update, control_expressions)
        .run();
}

fn spawn_light(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(3.0, 3.0, 0.3).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera3d::default(), Transform::from_xyz(0., 0.8, 2.5)));
}

fn spawn_vrm(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(VrmHandle(asset_server.load("vrm/AliciaSolid.vrm")));
}

fn control_expressions(
    mut commands: Commands,
    vrms: Query<Entity, With<Vrm>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    for vrm in vrms.iter() {
        if input.just_pressed(KeyCode::Digit1) {
            commands.trigger(SetExpressions::single(vrm, "happy", 1.0));
        }
        if input.just_pressed(KeyCode::Digit2) {
            commands.trigger(SetExpressions::single(vrm, "angry", 1.0));
        }
        if input.just_pressed(KeyCode::Digit3) {
            commands.trigger(SetExpressions::single(vrm, "sad", 1.0));
        }
        if input.just_pressed(KeyCode::Digit4) {
            commands.trigger(SetExpressions::single(vrm, "blink", 1.0));
        }
        if input.just_pressed(KeyCode::Digit0) {
            commands.trigger(ClearExpressions { entity: vrm });
        }
    }
}
