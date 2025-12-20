use crate::prelude::ChildSearcher;
use crate::vrm::node_constraint::registry::{Constraint, NodeConstraintRegistry};
use crate::vrm::node_constraint::{
    AimConstraintDest, AimConstraintDestinations, RollConstraintDest, RollConstraintDestinations,
    RotationConstraintDest, RotationConstraintDestinations,
};
use bevy::prelude::*;

#[derive(EntityEvent)]
pub(crate) struct RequestInitializeNodeConstraints(pub(crate) Entity);

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
    trigger: On<RequestInitializeNodeConstraints>,
    mut commands: Commands,
    mut rotation_constraints: Query<Option<&mut RotationConstraintDestinations>>,
    mut roll_constraints: Query<Option<&mut RollConstraintDestinations>>,
    mut aim_constraints: Query<Option<&mut AimConstraintDestinations>>,
    child_searcher: ChildSearcher,
    models: Query<(Entity, &NodeConstraintRegistry)>,
) {
    let root = trigger.event_target();
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
                Constraint::Roll {
                    roll_axis,
                    source,
                    weight,
                } => {
                    register_roll_constraint(
                        vrm,
                        &mut commands,
                        &mut roll_constraints,
                        dest,
                        source,
                        *roll_axis,
                        *weight,
                        &child_searcher,
                    );
                }
                Constraint::Aim {
                    aim_axis,
                    source,
                    weight,
                } => {
                    register_aim_constraint(
                        vrm,
                        &mut commands,
                        &mut aim_constraints,
                        dest,
                        source,
                        *aim_axis,
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

fn register_roll_constraint(
    vrm: Entity,
    commands: &mut Commands,
    rotation_constraints: &mut Query<Option<&mut RollConstraintDestinations>>,
    dest: Entity,
    source_name: &str,
    roll_axis: Dir3,
    weight: f32,
    child_searcher: &ChildSearcher,
) {
    if let Some(source) = child_searcher.find_from_name(vrm, source_name) {
        let roll_constraint = RollConstraintDest {
            roll_axis,
            dest,
            weight,
        };
        if let Ok(Some(mut existing)) = rotation_constraints.get_mut(source) {
            existing.0.push(roll_constraint);
        } else {
            commands
                .entity(source)
                .insert(RollConstraintDestinations(vec![roll_constraint]));
        }
    }
}

fn register_aim_constraint(
    vrm: Entity,
    commands: &mut Commands,
    rotation_constraints: &mut Query<Option<&mut AimConstraintDestinations>>,
    dest: Entity,
    source_name: &str,
    roll_axis: Dir3,
    weight: f32,
    child_searcher: &ChildSearcher,
) {
    if let Some(source) = child_searcher.find_from_name(vrm, source_name) {
        let roll_constraint = AimConstraintDest {
            aim_axis: roll_axis,
            dest,
            weight,
        };
        if let Ok(Some(mut existing)) = rotation_constraints.get_mut(source) {
            existing.0.push(roll_constraint);
        } else {
            commands
                .entity(source)
                .insert(AimConstraintDestinations(vec![roll_constraint]));
        }
    }
}
