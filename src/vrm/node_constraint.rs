use crate::vrm::node_constraint::bind::aim::AimConstraintBindPlugin;
use crate::vrm::node_constraint::bind::roll::RollConstraintBindPlugin;
use crate::vrm::node_constraint::bind::rotation::RotationConstraintBindPlugin;
use crate::vrm::node_constraint::initialize::NodeConstraintInitializePlugin;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub mod bind;
pub mod initialize;
pub mod registry;

#[derive(Debug, Clone, Reflect, Serialize, Deserialize, Component)]
#[reflect(Component, Serialize, Deserialize, Clone)]
pub struct RotationConstraintDestinations(pub Vec<RotationConstraintDest>);

#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize, Clone)]
pub struct RotationConstraintDest {
    pub dest: Entity,
    pub weight: f32,
}

#[derive(Debug, Clone, Reflect, Serialize, Deserialize, Component)]
#[reflect(Component, Serialize, Deserialize, Clone)]
pub struct RollConstraintDestinations(pub Vec<RollConstraintDest>);

#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize, Clone)]
pub struct RollConstraintDest {
    pub dest: Entity,
    pub weight: f32,
    pub roll_axis: Dir3,
}

#[derive(Debug, Clone, Reflect, Serialize, Deserialize, Component)]
#[reflect(Component, Serialize, Deserialize, Clone)]
pub struct AimConstraintDestinations(pub Vec<AimConstraintDest>);

#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize, Clone)]
pub struct AimConstraintDest {
    pub dest: Entity,
    pub weight: f32,
    pub aim_axis: Dir3,
}

pub struct VrmNodeConstraintPlugin;

impl Plugin for VrmNodeConstraintPlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.register_type::<RotationConstraintDestinations>()
            .register_type::<RotationConstraintDest>()
            .register_type::<RollConstraintDestinations>()
            .register_type::<RollConstraintDest>()
            .register_type::<AimConstraintDestinations>()
            .register_type::<AimConstraintDest>()
            .add_plugins((
                NodeConstraintInitializePlugin,
                RotationConstraintBindPlugin,
                RollConstraintBindPlugin,
                AimConstraintBindPlugin,
            ));
    }
}
