//! This module inserts [`SceneRoot`] and VRMA-related components from the loaded [`VrmaHandle`].

use crate::error::vrm_error;
use crate::vrm::Initialized;
use crate::vrm::humanoid_bone::HumanoidBoneRegistry;
use crate::vrma::animation::animation_graph::RequestUpdateAnimationGraph;
use crate::vrma::animation::expressions::VrmaExpressionNames;
use crate::vrma::gltf::extensions::VrmaExtensions;
use crate::vrma::loader::VrmaAsset;
use crate::vrma::{LoadedVrma, VrmAnimationClipHandle, Vrma, VrmaDuration, VrmaHandle, VrmaPath};
use bevy::gltf::GltfNode;
use bevy::prelude::*;
use bevy::scene::SceneRoot;
use std::time::Duration;

pub(super) struct VrmaInitializePlugin;

impl Plugin for VrmaInitializePlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.add_systems(Update, (spawn_vrma, trigger_loaded));
    }
}

fn spawn_vrma(
    mut commands: Commands,
    vrma_assets: Res<Assets<VrmaAsset>>,
    node_assets: Res<Assets<GltfNode>>,
    mut clip_assets: ResMut<Assets<AnimationClip>>,
    mut scene_assets: ResMut<Assets<Scene>>,
    type_registry: Res<AppTypeRegistry>,
    vrma_handles: Query<(Entity, &VrmaHandle, &ChildOf)>,
    vrms: Query<Has<Initialized>>,
) {
    for (handle_entity, handle, child_of) in vrma_handles.iter() {
        if !vrms
            .get(child_of.parent())
            .is_ok_and(|initialized| initialized)
        {
            continue;
        }
        let Some(vrma_path) = handle.0.path().map(|path| path.path().to_path_buf()) else {
            continue;
        };
        let Some(name) = handle.0.path().map(|p| p.to_string()) else {
            continue;
        };
        let Some(vrma) = vrma_assets.get(handle.0.id()) else {
            continue;
        };
        commands.entity(handle_entity).remove::<VrmaHandle>();

        let Some(scene_handle) = vrma.gltf.scenes.first() else {
            vrm_error!("[VRMA] Not found vrma scene in {name}");
            continue;
        };
        // Clone the Scene asset per-VRMA instance to prevent SceneSpawner
        // from grouping instances by shared AssetId and respawning them together.
        let Some(source_scene) = scene_assets.get(scene_handle) else {
            vrm_error!("[VRMA] Not found scene data for {name}");
            continue;
        };
        let Ok(cloned_scene) = source_scene.clone_with(&type_registry) else {
            vrm_error!("[VRMA] Failed to clone scene for {name}");
            continue;
        };
        let scene_root = scene_assets.add(cloned_scene);
        let extensions = match VrmaExtensions::from_gltf(&vrma.gltf) {
            Ok(extensions) => extensions,
            Err(_e) => {
                vrm_error!("[VRMA] Not found vrma extensions in {name}:\n{_e}");
                continue;
            }
        };
        let Some(animation_clip_handle) = vrma.gltf.animations.first() else {
            vrm_error!("[VRMA] Not found vrma animations in {name}");
            continue;
        };
        let Some(animation_clip) = clip_assets.get(animation_clip_handle).cloned() else {
            vrm_error!("[VRMA] Not found animation clip for {name}");
            continue;
        };
        let animation_clip_handle = clip_assets.add(animation_clip);
        commands.entity(handle_entity).insert((
            Vrma,
            Name::new(name),
            VrmAnimationClipHandle(animation_clip_handle.clone()),
            SceneRoot(scene_root),
            VrmaDuration(obtain_vrma_duration(&clip_assets, &vrma.gltf.animations)),
            VrmaPath(vrma_path),
            VrmaExpressionNames::new(&extensions),
            HumanoidBoneRegistry::new(
                &extensions.vrmc_vrm_animation.humanoid.human_bones,
                &node_assets,
                &vrma.gltf.nodes,
            ),
        ));
    }
}

fn obtain_vrma_duration(
    assets: &Assets<AnimationClip>,
    handles: &[Handle<AnimationClip>],
) -> Duration {
    let duration = handles
        .iter()
        .filter_map(|handle| assets.get(handle))
        .map(|clip| clip.duration() as f64)
        .fold(0., |v1, v2| v2.max(v1));
    Duration::from_secs_f64(duration)
}

fn trigger_loaded(
    mut commands: Commands,
    vrmas: Query<(Entity, &ChildOf), (Added<Initialized>, With<Vrma>)>,
) {
    for (vrma_entity, child_of) in vrmas.iter() {
        let vrm_entity = child_of.parent();
        // Trigger animation graph setup first (deferred from request_initialize
        // to ensure VrmBone components are fully applied on the parent VRM's bones)
        commands.trigger(RequestUpdateAnimationGraph {
            vrma: vrma_entity,
            vrm: vrm_entity,
        });
        commands.trigger(LoadedVrma {
            vrm: vrm_entity,
            vrma: vrma_entity,
        });
    }
}
