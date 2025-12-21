use crate::prelude::VrmSystemSets;
use crate::vrm::RestTransform;
use crate::vrm::node_constraint::RotationConstraintDestinations;
use bevy::app::{AnimationSystems, Plugin};
use bevy::prelude::*;

pub(crate) struct RotationConstraintBindPlugin;

impl Plugin for RotationConstraintBindPlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.add_systems(
            PostUpdate,
            bind_rotation_constraints
                .in_set(VrmSystemSets::Constraints)
                .after(AnimationSystems),
        );
    }
}

fn bind_rotation_constraints(
    par_commands: ParallelCommands,
    sources: Query<
        (&Transform, &RestTransform, &RotationConstraintDestinations),
        Changed<Transform>,
    >,
    dests: Query<(&Transform, &RestTransform)>,
) {
    sources
        .par_iter()
        .for_each(|(src_tf, src_rest, destinations)| {
            for dest in &destinations.0 {
                if let Ok((dest_tf, dest_rest)) = dests.get(dest.dest) {
                    let src_delta_quat = src_rest.rotation.inverse() * src_tf.rotation;
                    let new_rot = dest_rest
                        .rotation
                        .slerp(dest_rest.rotation * src_delta_quat, dest.weight);
                    par_commands.command_scope(|mut commands| {
                        commands.entity(dest.dest).insert(Transform {
                            rotation: new_rot,
                            ..*dest_tf
                        });
                    });
                }
            }
        });
}
