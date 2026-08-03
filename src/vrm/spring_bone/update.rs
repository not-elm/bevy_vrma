use crate::system_set::VrmSystemSets;
use crate::vrm::gltf::extensions::vrmc_spring_bone::ColliderShape;
use crate::vrm::spring_bone::{SpringJointProps, SpringJointState, SpringRoot};
use bevy::app::App;
use bevy::math::{Mat4, Vec3};
use bevy::prelude::*;
use bevy::time::Time;

pub struct SpringBoneUpdatePlugin;

impl Plugin for SpringBoneUpdatePlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.add_systems(
            PostUpdate,
            update_spring_bones
                .in_set(VrmSystemSets::SpringBone)
                .after(VrmSystemSets::PropagateAfterExpressions),
        );
    }
}

fn update_spring_bones(
    mut transforms: Query<(&mut Transform, &mut GlobalTransform)>,
    mut joints: Query<(&ChildOf, &mut SpringJointState, &SpringJointProps)>,
    spring_roots: Query<&SpringRoot>,
    time: Res<Time>,
) {
    let delta_time = time.delta_secs();
    for spring_root in spring_roots.iter() {
        let center_gtf = spring_root
            .center_node
            .and_then(|center| transforms.get(center).ok())
            .map(|(_, gtf)| gtf)
            .copied();
        for joint in spring_root.joints.iter().copied() {
            let Ok((child_of, mut state, props)) = joints.get_mut(joint) else {
                continue;
            };
            let parent_gtf = transforms
                .get(child_of.parent())
                .map(|(_, gtf)| *gtf)
                .unwrap_or_default();
            let parent_global_rotation = parent_gtf.to_scale_rotation_translation().1;
            let Ok(head_global_pos) = transforms.get(joint).map(|(_, gtf)| gtf.translation())
            else {
                continue;
            };

            let current_tail = center_local_to_global(state.current_tail, &center_gtf);
            let prev_tail = center_local_to_global(state.prev_tail, &center_gtf);
            let inertia = (current_tail - prev_tail) * (1. - props.drag_force);
            let stiffness = delta_time
                * (parent_global_rotation
                    * state.initial_local_rotation
                    * state.bone_axis
                    * props.stiffness);
            let external = delta_time * props.gravity_dir * props.gravity_power;

            let next_tail = current_tail + inertia + stiffness + external;
            let mut next_tail =
                head_global_pos + (next_tail - head_global_pos).normalize() * state.bone_length;

            apply_collision(
                &mut next_tail,
                spring_root.colliders.iter().copied(),
                props.hit_radius,
                head_global_pos,
                state.bone_length,
                &transforms,
            );

            state.prev_tail = state.current_tail;
            state.current_tail = global_to_center_local(next_tail, &center_gtf);

            let initial_global_matrix = parent_gtf.to_matrix() * state.initial_local_matrix;

            let Ok((mut tf, mut gtf)) = transforms.get_mut(joint) else {
                continue;
            };

            let delta_rotation =
                tail_rotation_or_identity(state.bone_axis, initial_global_matrix, next_tail);
            tf.rotation = state.initial_local_rotation * delta_rotation;
            *gtf = parent_gtf.mul_transform(*tf);
        }
    }
}

fn tail_rotation_or_identity(
    from: Vec3,
    initial_global_matrix: Mat4,
    tail_global_pos: Vec3,
) -> Quat {
    let inverse = initial_global_matrix.inverse();
    let to = inverse.transform_point3(tail_global_pos);
    if let Some(to) = to.try_normalize() {
        Quat::from_rotation_arc(from, to)
    } else {
        let initial_head_global_pos = initial_global_matrix.transform_point3(Vec3::ZERO);
        let to = inverse.transform_vector3(tail_global_pos - initial_head_global_pos);
        rotation_arc_or_identity(from, to)
    }
}

fn rotation_arc_or_identity(
    from: Vec3,
    to: Vec3,
) -> Quat {
    to.try_normalize()
        .map(|to| Quat::from_rotation_arc(from, to))
        .unwrap_or(Quat::IDENTITY)
}

fn center_local_to_global(
    tail_pos: Vec3,
    center_gtf: &Option<GlobalTransform>,
) -> Vec3 {
    if let Some(gtf) = center_gtf.as_ref() {
        gtf.transform_point(tail_pos)
    } else {
        tail_pos
    }
}

fn global_to_center_local(
    tail_pos: Vec3,
    center_gtf: &Option<GlobalTransform>,
) -> Vec3 {
    if let Some(gtf) = center_gtf.as_ref() {
        gtf.to_matrix().inverse().transform_point3(tail_pos)
    } else {
        tail_pos
    }
}

fn apply_collision(
    next_tail: &mut Vec3,
    collider_entities: impl Iterator<Item = (Entity, ColliderShape)>,
    joint_radius: f32,
    head_global_pos: Vec3,
    bone_length: f32,
    transforms: &Query<(&mut Transform, &mut GlobalTransform)>,
) {
    for (collider, collider_shape) in collider_entities {
        let Ok((_, collider_gtf)) = transforms.get(collider) else {
            continue;
        };
        collider_shape.apply_collision(
            next_tail,
            collider_gtf,
            head_global_pos,
            joint_radius,
            bone_length,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_rotation_matches_previous_behavior_for_finite_direction() {
        let from = Vec3::new(1.0, 2.0, 3.0).normalize();
        let matrix = Mat4::from_scale_rotation_translation(
            Vec3::new(0.75, 1.25, 0.5),
            Quat::from_euler(EulerRot::XYZ, 0.2, -0.3, 0.4),
            Vec3::new(-1.0, 2.0, 3.0),
        );
        let tail = matrix.transform_point3(Vec3::new(-4.0, 5.0, 6.0));

        let actual = tail_rotation_or_identity(from, matrix, tail);
        let previous =
            Quat::from_rotation_arc(from, matrix.inverse().transform_point3(tail).normalize());

        assert_eq!(actual, previous);
    }

    #[test]
    fn tail_rotation_falls_back_when_point_transform_cancels_out() {
        let matrix = Mat4::from_cols_array(&[
            0.35692674,
            -0.6559648,
            -0.06937952,
            0.0,
            1.0731882,
            0.60601205,
            -0.20860665,
            0.0,
            0.0954045,
            3.7252903e-9,
            0.49081358,
            0.0,
            -0.09683231,
            1.3288133,
            0.070472986,
            1.0,
        ]);
        let head = matrix.transform_point3(Vec3::ZERO);
        let tail = Vec3::new(-0.0968323, 1.3288133, 0.070473);
        let inverse = matrix.inverse();
        let via_point = inverse.transform_point3(tail);
        let via_vector = inverse.transform_vector3(tail - head);

        let actual = tail_rotation_or_identity(Vec3::Y, matrix, tail);
        let expected = Quat::from_rotation_arc(Vec3::Y, via_vector.normalize());

        assert_eq!(via_point, Vec3::ZERO);
        assert_ne!(via_vector, Vec3::ZERO);
        assert!(actual.is_finite());
        assert_eq!(actual, expected);
    }

    #[test]
    fn rotation_arc_is_identity_for_zero_direction() {
        assert_eq!(
            rotation_arc_or_identity(Vec3::Y, Vec3::ZERO),
            Quat::IDENTITY
        );
        assert_eq!(
            tail_rotation_or_identity(Vec3::Y, Mat4::IDENTITY, Vec3::ZERO),
            Quat::IDENTITY
        );
    }
}
