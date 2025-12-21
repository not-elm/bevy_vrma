use crate::prelude::{RestTransform, VrmSystemSets};
use crate::vrm::node_constraint::AimConstraintDestinations;
use bevy::app::AnimationSystems;
use bevy::prelude::*;

pub(crate) struct AimConstraintBindPlugin;

impl Plugin for AimConstraintBindPlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.add_systems(
            PostUpdate,
            bind_aim_constraints
                .in_set(VrmSystemSets::Constraints)
                .after(AnimationSystems),
        );
    }
}

fn bind_aim_constraints(
    par_commands: ParallelCommands,
    sources: Query<(&GlobalTransform, &AimConstraintDestinations), Changed<GlobalTransform>>,
    parents: Query<&GlobalTransform>,
    dests: Query<(&ChildOf, &Transform, &GlobalTransform, &RestTransform)>,
) {
    sources.par_iter().for_each(|(src_gtf, destinations)| {
        for dest in &destinations.0 {
            if let Ok((ChildOf(parent), dest_tf, dest_gtf, dest_rest)) = dests.get(dest.dest)
                && let Ok(parent_tf) = parents.get(*parent)
            {
                let dest_rest_q = dest_rest.rotation;
                let dest_parent_world_q = parent_tf.rotation();

                let from_vec = dest_parent_world_q * dest_rest_q * dest.aim_axis.as_vec3();
                let to_vec = (src_gtf.translation() - dest_gtf.translation()).normalize();
                let from_to_q = Quat::from_rotation_arc(from_vec, to_vec);
                let new_rot = dest_rest.rotation.slerp(
                    dest_parent_world_q.inverse() * from_to_q * dest_parent_world_q * dest_rest_q,
                    dest.weight,
                );
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
