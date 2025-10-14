use crate::vrm::node_constraint::bind::NodeConstraintBindPlugin;
use crate::vrm::node_constraint::initialize::NodeConstraintInitializePlugin;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

mod bind;
pub(crate) mod initialize;
pub(crate) mod registry;

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
