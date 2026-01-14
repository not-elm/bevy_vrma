//! This module handles humanoid bones.
//! Refer to [here](https://docs.unity3d.com/ja/2019.4/ScriptReference/HumanBodyBones.html) for the list of humanoid bones.
//!
//! After the VRM(A) is loaded, marker components are inserted for each bone.
//! For example, the entity of the hips bone will have [`Hips`] inserted.
//! Additionally, a component that holds the entity will be inserted into the VRM(A) entity.
//!
//! The setup of these is done after all bones have been spawned, so there may be a slight delay.

mod bones;

use crate::prelude::*;
use crate::vrm::gltf::extensions::VrmNode;
use crate::vrm::humanoid_bone::bones::BonesPlugin;
use crate::vrm::{RestGlobalTransform, RestTransform, VrmBone};
use crate::vrma::RetargetSource;
use bevy::animation::{AnimatedBy, AnimationTargetId};
use bevy::app::{App, Plugin};
use bevy::asset::{Assets, Handle};
use bevy::gltf::GltfNode;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

pub mod prelude {
    pub use crate::vrm::humanoid_bone::bones::*;
}

#[derive(EntityEvent)]
pub(crate) struct RequestInitializeHumanoidBones(pub(crate) Entity);

#[derive(Component, Deref, Reflect, Default)]
pub(crate) struct HumanoidBoneRegistry(HashMap<VrmBone, Name>);

impl HumanoidBoneRegistry {
    pub fn new(
        bones: &HashMap<String, VrmNode>,
        node_assets: &Assets<GltfNode>,
        nodes: &[Handle<GltfNode>],
    ) -> Self {
        Self(
            bones
                .iter()
                .filter_map(|(name, target_node)| {
                    let node_handle = nodes.get(target_node.node)?;
                    let node = node_assets.get(node_handle)?;
                    Some((VrmBone(name.clone()), Name::new(node.name.clone())))
                })
                .collect(),
        )
    }
}

pub(super) struct VrmHumanoidBonePlugin;

impl Plugin for VrmHumanoidBonePlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.register_type::<HumanoidBoneRegistry>()
            .add_plugins(BonesPlugin)
            .add_observer(apply_insert_rest_transforms)
            .add_observer(apply_initialize_humanoid_bones);
    }
}

macro_rules! insert_bone {
    (
        $commands: expr,
        $vrm_entity: expr,
        $bone_entity: expr,
        $bone_name: expr,
        $($bone: ident),+$(,)?
    ) => {

        match $bone_name.0.to_uppercase(){
            $(
                x if x == stringify!($bone).to_uppercase() => {
                    paste::paste!{
                        $commands.entity($vrm_entity).insert([<$bone BoneEntity>]($bone_entity));
                    }
                    $commands.entity($bone_entity).insert($bone);
                }
            )+
            _ => {

            }
        }
    };
}

fn apply_insert_rest_transforms(
    trigger: On<RequestInitializeHumanoidBones>,
    mut commands: Commands,
    childrens: Query<&Children>,
    transforms: Query<(&Transform, &GlobalTransform)>,
) {
    let vrm = trigger.event_target();
    insert_rest_transforms_recursive(&mut commands, vrm, &childrens, &transforms);
}

fn insert_rest_transforms_recursive(
    commands: &mut Commands,
    entity: Entity,
    childrens: &Query<&Children>,
    transforms: &Query<(&Transform, &GlobalTransform)>,
) {
    let Ok(children) = childrens.get(entity) else {
        return;
    };
    for child in children {
        let Ok((tf, gtf)) = transforms.get(entity) else {
            continue;
        };
        commands
            .entity(entity)
            .insert((RestTransform(*tf), RestGlobalTransform(*gtf)));
        insert_rest_transforms_recursive(commands, *child, childrens, transforms);
    }
}

fn apply_initialize_humanoid_bones(
    trigger: On<RequestInitializeHumanoidBones>,
    mut commands: Commands,
    searcher: ChildSearcher,
    models: Query<&HumanoidBoneRegistry>,
    parents: Query<&ChildOf>,
    transforms: Query<(&Transform, &GlobalTransform)>,
    has_vrm: Query<Has<Vrm>>,
) {
    let model_entity = trigger.event_target();
    let Ok(registry) = models.get(model_entity) else {
        return;
    };
    let Some(hips) =
        searcher.find_from_name(model_entity, registry.get(&VrmBone::from("hips")).unwrap())
    else {
        return;
    };
    let Ok(ChildOf(root_bone)) = parents.get(hips) else {
        return;
    };
    let has_vrm = has_vrm.get(model_entity).is_ok_and(|h| h);
    commands
        .entity(*root_bone)
        .insert((AnimationPlayer::default(), AnimationTransitions::default()));
    if has_vrm {
        commands.entity(*root_bone).insert((
            Name::new(Vrm::ROOT_BONE),
            RetargetSource,
            AnimationTargetId::from_name(&Name::new(Vrm::ROOT_BONE)),
            AnimatedBy(*root_bone),
        ));
    }

    for (bone, name) in registry.iter() {
        let Some(bone_entity) = searcher.find_from_name(model_entity, name.as_str()) else {
            continue;
        };
        let Ok((tf, gtf)) = transforms.get(bone_entity) else {
            continue;
        };
        commands.entity(bone_entity).insert((
            bone.clone(),
            RestTransform(*tf),
            RestGlobalTransform(*gtf),
            RetargetSource,
        ));
        if has_vrm {
            commands
                .entity(bone_entity)
                .insert((AnimationTargetId::from_name(name), AnimatedBy(*root_bone)));
        }
        insert_bone!(
            commands,
            model_entity,
            bone_entity,
            bone,
            Hips,
            RightRingProximal,
            RightThumbDistal,
            RightRingIntermediate,
            RightUpperArm,
            LeftIndexProximal,
            LeftUpperLeg,
            LeftFoot,
            LeftIndexDistal,
            LeftThumbMetacarpal,
            RightLowerArm,
            LeftMiddleDistal,
            RightUpperLeg,
            LeftToes,
            LeftThumbDistal,
            RightShoulder,
            RightThumbMetacarpal,
            Spine,
            LeftLowerLeg,
            LeftShoulder,
            LeftUpperArm,
            UpperChest,
            RightToes,
            RightIndexDistal,
            LeftMiddleProximal,
            LeftRingProximal,
            LeftRingDistal,
            LeftThumbProximal,
            LeftIndexIntermediate,
            LeftLittleProximal,
            LeftLittleDistal,
            RightHand,
            RightLittleProximal,
            LeftRingIntermediate,
            RightIndexIntermediate,
            Chest,
            LeftHand,
            RightLittleIntermediate,
            RightFoot,
            RightLowerLeg,
            LeftLittleIntermediate,
            LeftLowerArm,
            RightLittleDistal,
            RightMiddleIntermediate,
            RightMiddleProximal,
            RightThumbProximal,
            Neck,
            Jaw,
            Head,
            LeftEye,
            RightEye,
            LeftMiddleIntermediate,
            RightRingDistal,
            RightIndexProximal,
            RightMiddleDistal,
        );
    }
}
