use crate::vrm::node_constraint::rotation::bind::NodeConstraintBindPlugin;
use crate::vrm::node_constraint::rotation::initialize::NodeConstraintInitializePlugin;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub mod registry;
pub mod rotation;

#[derive(Debug, Clone, Reflect, Serialize, Deserialize, Component)]
#[reflect(Component, Serialize, Deserialize, Clone)]
pub struct RotationConstraintDestinations(pub Vec<RotationConstraintDest>);

#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize, Clone)]
pub struct RotationConstraintDest {
    pub dest: Entity,
    pub weight: f32,
}

pub struct VrmNodeConstraintPlugin;

impl Plugin for VrmNodeConstraintPlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.register_type::<RotationConstraintDestinations>()
            .register_type::<RotationConstraintDest>()
            .add_plugins((NodeConstraintInitializePlugin, NodeConstraintBindPlugin));
    }
}
