use crate::system_param::child_searcher::ChildSearcher;
use crate::vrm::extensions::VrmNode;
use crate::vrm::{BoneRestGlobalTransform, BoneRestTransform, VrmBone, VrmHipsBoneTo};
use bevy::app::{App, Plugin, Update};
use bevy::asset::{Assets, Handle};
use bevy::gltf::GltfNode;
use bevy::platform_support::collections::HashMap;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Reflect, Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
pub struct Hips;

#[derive(Component, Deref, Reflect)]
pub struct HumanoidBoneRegistry(HashMap<VrmBone, Name>);

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

pub struct VrmHumanoidBonePlugin;

impl Plugin for VrmHumanoidBonePlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.register_type::<HumanoidBonesAttached>()
            .register_type::<HumanoidBoneRegistry>()
            .register_type::<Hips>()
            .add_systems(Update, attach_bones);
    }
}

#[derive(Component, Reflect, Serialize, Deserialize)]
struct HumanoidBonesAttached;

fn attach_bones(
    mut commands: Commands,
    searcher: ChildSearcher,
    vrm: Query<(Entity, &HumanoidBoneRegistry), (Without<HumanoidBonesAttached>, With<Children>)>,
    transforms: Query<(&Transform, &GlobalTransform)>,
) {
    for (vrm_entity, humanoid_bones) in vrm.iter() {
        /*if searcher.find_from_name(vrm_entity, "Root").is_none() &&
            // for blender export names it Root_ instead of Root
            searcher.find_from_name(vrm_entity, "Root_").is_none() &&
            searcher.find_from_name(vrm_entity, "Armature_rootJoint").is_none() {
            println!("NO BONES");
            continue;
        }*/

        for (bone, name) in humanoid_bones.iter() {
            let Some(bone_entity) = searcher.find_from_name(vrm_entity, name.as_str()) else {
                continue;
            };
            let Ok((tf, gtf)) = transforms.get(bone_entity) else {
                continue;
            };
            commands.entity(bone_entity).insert((
                bone.clone(),
                BoneRestTransform(*tf),
                BoneRestGlobalTransform(*gtf),
            ));
            // Use hips when sitting on window and retargeting.
            if bone.0 == "hips" {
                commands
                    .entity(vrm_entity)
                    .insert(VrmHipsBoneTo(bone_entity));
                commands.entity(bone_entity).insert(Hips);
            }
        }
        commands.entity(vrm_entity).insert(HumanoidBonesAttached);
    }
}
