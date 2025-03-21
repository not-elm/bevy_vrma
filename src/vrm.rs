pub mod expressions;
pub mod extensions;
pub mod humanoid_bone;
pub mod loader;
mod spawn;
mod spring_bone;

use crate::new_type;
use crate::vrm::expressions::VrmExpressionPlugin;
use crate::vrm::humanoid_bone::VrmHumanoidBonePlugin;
use crate::vrm::loader::{VrmAsset, VrmLoaderPlugin};
use crate::vrm::spawn::VrmSpawnPlugin;
use crate::vrm::spring_bone::VrmSpringBonePlugin;
use bevy::app::{App, Plugin};
use bevy::asset::AssetApp;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

new_type!(
    /// The bone name obtained from `VRMC_vrm::humanoid`.
    name: VrmBone,
    ty: String,
);

new_type!(
    /// The key name of `VRMC_vrm::expressions::preset`.
    name: VrmExpression,
    ty: String,
);

/// A marker component attached to the entity of VRM.
#[derive(Debug, Component, Reflect, Copy, Clone, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
pub struct Vrm;

/// The path to the VRM file.
#[derive(Debug, Reflect, Clone, Component, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
pub struct VrmPath(pub PathBuf);

impl VrmPath {
    /// Creates a new [`VrmPath`] from the path.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }
}

/// The bone's initial transform.
#[derive(Debug, Reflect, Copy, Clone, Component, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
pub struct BoneRestTransform(pub Transform);

/// The bone's initial global transform.
#[derive(Debug, Reflect, Copy, Clone, Component, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
pub struct BoneRestGlobalTransform(pub GlobalTransform);

/// Holds the entity of the hips bone.
#[derive(Debug, Reflect, Copy, Clone, Component, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
pub struct VrmHipsBoneTo(pub Entity);

pub struct VrmPlugin;

impl Plugin for VrmPlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.init_asset::<VrmAsset>()
            .register_type::<Vrm>()
            .register_type::<VrmPath>()
            .register_type::<BoneRestTransform>()
            .register_type::<BoneRestGlobalTransform>()
            .register_type::<VrmHipsBoneTo>()
            .register_type::<VrmBone>()
            .add_plugins((
                VrmLoaderPlugin,
                VrmSpawnPlugin,
                VrmSpringBonePlugin,
                VrmHumanoidBonePlugin,
                VrmExpressionPlugin,
            ));
    }
}
