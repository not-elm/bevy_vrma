use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Reflect)]
#[reflect(Serialize, Deserialize)]
pub struct Rotation {
    pub source: usize,
    pub weight: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Reflect)]
#[reflect(Serialize, Deserialize)]
pub struct Roll {
    #[serde(rename = "rollAxis")]
    pub roll_axis: String,
    pub source: usize,
    pub weight: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Reflect)]
#[reflect(Serialize, Deserialize)]
pub struct Aim {
    #[serde(rename = "aimAxis")]
    pub aim_axis: String,
    pub source: usize,
    pub weight: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Reflect)]
#[reflect(Serialize, Deserialize)]
pub struct Constraint {
    pub rotation: Option<Rotation>,
    pub roll: Option<Roll>,
    pub aim: Option<Aim>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Reflect)]
#[reflect(Serialize, Deserialize)]
pub struct VrmcNodeConstraint {
    pub constraint: Constraint,
    #[serde(rename = "specVersion")]
    pub spec_version: String,
}
