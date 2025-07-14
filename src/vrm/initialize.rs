use crate::error::vrm_error;
use crate::prelude::ChildSearcher;
use crate::vrm::expressions::{RequestInitializeExpressions, VrmExpressionRegistry};
use crate::vrm::gltf::extensions::VrmExtensions;
use crate::vrm::humanoid_bone::{HumanoidBoneRegistry, RequestInitializeHumanoidBones};
use crate::vrm::loader::{VrmAsset, VrmHandle};
use crate::vrm::mtoon::VrmcMaterialRegistry;
use crate::vrm::spring_bone::initialize::RequestInitializeSpringBone;
use crate::vrm::spring_bone::registry::*;
use crate::vrm::{Initialized, Vrm, VrmPath};
use crate::vrma::Vrma;
use crate::vrma::animation::animation_graph::RequestUpdateAnimationGraph;
use bevy::app::{App, Update};
use bevy::asset::Assets;
use bevy::gltf::GltfNode;
use bevy::prelude::*;
use bevy::scene::SceneRoot;

pub(crate) struct VrmInitializePlugin;

impl Plugin for VrmInitializePlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.add_systems(Update, (spawn_vrm, request_initialize));
    }
}

fn spawn_vrm(
    mut commands: Commands,
    node_assets: Res<Assets<GltfNode>>,
    vrm_assets: Res<Assets<VrmAsset>>,
    handles: Query<(Entity, &VrmHandle)>,
) {
    for (vrm_handle_entity, handle) in handles.iter() {
        let Some(vrm) = vrm_assets.get(handle.0.id()) else {
            continue;
        };
        commands.entity(vrm_handle_entity).remove::<VrmHandle>();

        let Some(scene) = vrm.gltf.scenes.first() else {
            continue;
        };
        let extensions = match VrmExtensions::from_gltf(&vrm.gltf) {
            Ok(extensions) => extensions,
            Err(e) => {
                vrm_error!("Failed to load VRM extensions", e);
                continue;
            }
        };

        let mut cmd = commands.entity(vrm_handle_entity);
        cmd.insert((
            Vrm,
            Name::new(extensions.name().unwrap_or_else(|| "VRM".to_string())),
            SceneRoot(scene.clone()),
            VrmcMaterialRegistry::new(&vrm.gltf, vrm.images.clone()),
            VrmExpressionRegistry::new(&extensions, &node_assets, &vrm.gltf.nodes),
            HumanoidBoneRegistry::new(
                &extensions.vrmc_vrm.humanoid.human_bones,
                &node_assets,
                &vrm.gltf.nodes,
            ),
        ));

        if let Some(spring_bone) = extensions.vrmc_spring_bone.as_ref() {
            cmd.insert((
                SpringJointPropsRegistry::new(
                    &spring_bone.all_joints(),
                    &node_assets,
                    &vrm.gltf.nodes,
                ),
                SpringColliderRegistry::new(&spring_bone.colliders, &node_assets, &vrm.gltf.nodes),
                SpringNodeRegistry::new(spring_bone, &node_assets, &vrm.gltf.nodes),
            ));
        }

        if let Some(look_at) = extensions.vrmc_vrm.look_at.clone() {
            cmd.insert(look_at);
        }

        if let Some(vrm_path) = handle.0.path() {
            #[cfg(feature = "develop")]
            {
                if let Some(vrm_name) = vrm_path.path().file_stem() {
                    output_vrm(vrm_name, &vrm.gltf);
                    output_vrm_materials(vrm_name, &vrm.gltf);
                    output_vrm_extensions(vrm_name, &extensions);
                }
            }
            cmd.insert(VrmPath::new(vrm_path.path()));
        }
    }
}

fn request_initialize(
    mut commands: Commands,
    models: Query<(Entity, &HumanoidBoneRegistry, Has<Vrma>), Without<Initialized>>,
    parents: Query<&ChildOf>,
    searcher: ChildSearcher,
) {
    for (root, registry, has_vrma) in models.iter() {
        if !searcher.has_been_spawned_all_bones(root, registry) {
            continue;
        }
        commands
            .entity(root)
            .trigger(RequestInitializeHumanoidBones)
            .trigger(RequestInitializeSpringBone);
        if has_vrma {
            if let Ok(ChildOf(vrm)) = parents.get(root) {
                commands.entity(root).trigger(RequestUpdateAnimationGraph {
                    vrma: root,
                    vrm: *vrm,
                });
            };
        } else {
            commands.entity(root).trigger(RequestInitializeExpressions);
        }
        commands.entity(root).insert(Initialized);
    }
}

#[cfg(feature = "develop")]
fn output_vrm(
    vrm_name: &std::ffi::OsStr,
    gltf: &Gltf,
) {
    let name = vrm_name.to_str().unwrap();
    std::fs::write(
        format!("./develop/{name}.json"),
        serde_json::to_string_pretty(&gltf.source.as_ref().unwrap().as_json()).unwrap(),
    )
    .unwrap();
}

#[cfg(feature = "develop")]
fn output_vrm_materials(
    vrm_name: &std::ffi::OsStr,
    gltf: &Gltf,
) {
    let name = vrm_name.to_str().unwrap();
    std::fs::write(
        format!("./develop/{name}_materials.json"),
        serde_json::to_string_pretty(&gltf.source.as_ref().unwrap().as_json().materials).unwrap(),
    )
    .unwrap();
}

#[cfg(feature = "develop")]
fn output_vrm_extensions(
    vrm_name: &std::ffi::OsStr,
    extensions: &VrmExtensions,
) {
    let name = vrm_name.to_str().unwrap();
    std::fs::write(
        format!("./develop/{name}_extensions.json"),
        serde_json::to_string_pretty(extensions).unwrap(),
    )
    .unwrap();
}
