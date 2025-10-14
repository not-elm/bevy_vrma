use crate::prelude::ChildSearcher;
use crate::vrm::humanoid_bone::RequestInitializeHumanoidBones;
use crate::vrm::node_constraint::registry::{Constraint, NodeConstraintRegistry};
use crate::vrm::node_constraint::{RotationConstraintDest, RotationConstraintDestinations};
use crate::vrm::spring_bone::registry::SpringJointPropsRegistry;
use bevy::prelude::*;

#[derive(Event)]
pub(crate) struct RequestInitializeNodeConstraints;

pub struct NodeConstraintInitializePlugin;

impl Plugin for NodeConstraintInitializePlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.add_observer(apply_initialize_node_constraints);
    }
}

fn apply_initialize_node_constraints(
    trigger: Trigger<RequestInitializeNodeConstraints>,
    mut commands: Commands,
    mut rotation_constraints: Query<Option<&mut RotationConstraintDestinations>>,
    child_searcher: ChildSearcher,
    models: Query<(Entity, &NodeConstraintRegistry)>,
) {
    let root = trigger.target();
    let Ok((vrm, nodes)) = models.get(root) else {
        return;
    };
    for (name, constraints) in nodes.iter() {
        let Some(dest) = child_searcher.find_from_name(root, name.as_str()) else {
            continue;
        };
        for constraint in constraints {
            match constraint {
                Constraint::Rotation { source, weight } => {
                    register_rotation_constraint(
                        vrm,
                        &mut commands,
                        &mut rotation_constraints,
                        dest,
                        source,
                        *weight,
                        &child_searcher,
                    );
                }
            }
        }
    }
}

fn register_rotation_constraint(
    vrm: Entity,
    commands: &mut Commands,
    rotation_constraints: &mut Query<Option<&mut RotationConstraintDestinations>>,
    dest: Entity,
    source_name: &str,
    weight: f32,
    child_searcher: &ChildSearcher,
) {
    if let Some(source) = child_searcher.find_from_name(vrm, source_name) {
        if let Ok(Some(mut existing)) = rotation_constraints.get_mut(source) {
            existing.0.push(RotationConstraintDest { dest, weight });
        } else {
            commands
                .entity(source)
                .insert(RotationConstraintDestinations(vec![
                    RotationConstraintDest { dest, weight },
                ]));
        }
    }
}
