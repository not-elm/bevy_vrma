use crate::vrm::gltf::extensions::vrmc_node_constraint::VrmcNodeConstraint;
use bevy::asset::Assets;
use bevy::gltf;
use bevy::gltf::GltfNode;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Component, Reflect, Serialize, Deserialize, Deref)]
#[reflect(Component, Serialize, Deserialize)]
pub struct NodeConstraintRegistry(pub HashMap<String, Vec<Constraint>>);

impl NodeConstraintRegistry {
    pub fn new(
        gltf: &gltf::Gltf,
        node_assets: &Assets<GltfNode>,
    ) -> Self {
        let Some(source) = gltf.source.as_ref() else {
            return Self(HashMap::default());
        };

        let constraints = source
            .nodes()
            .flat_map(|n| {
                let extensions = n.extension_value("VRMC_node_constraint")?;
                let node = serde_json::from_value::<VrmcNodeConstraint>(extensions.clone()).ok()?;
                let name = n.name()?.to_string();
                Some((name, parse_constraints(gltf, node_assets, &node)))
            })
            .collect();
        Self(constraints)
    }
}

fn parse_constraints(
    gltf: &gltf::Gltf,
    node_assets: &Assets<GltfNode>,
    node: &VrmcNodeConstraint,
) -> Vec<Constraint> {
    let mut constraints = vec![];
    let nodes = &gltf.nodes;
    if let Some(roll) = &node.constraint.rotation
        && let Some(source_handle) = nodes.get(roll.source)
        && let Some(source) = node_assets.get(source_handle)
    {
        constraints.push(Constraint::Rotation {
            source: source.name.clone(),
            weight: roll.weight,
        });
    }
    if let Some(roll) = &node.constraint.roll
        && let Some(source_handle) = nodes.get(roll.source as usize)
        && let Some(source) = node_assets.get(source_handle)
    {
        let roll_axis = match roll.roll_axis.as_str() {
            "X" => Dir3::X,
            "Y" => Dir3::Y,
            "Z" => Dir3::Z,
            _ => Dir3::Y,
        };
        constraints.push(Constraint::Roll {
            source: source.name.clone(),
            roll_axis,
            weight: roll.weight as f32,
        });
    }

    constraints
}

impl From<HashMap<String, Vec<Constraint>>> for NodeConstraintRegistry {
    fn from(value: HashMap<String, Vec<Constraint>>) -> Self {
        Self(value)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Reflect)]
#[reflect(Serialize, Deserialize)]
pub enum Constraint {
    Rotation { source: String, weight: f32 },
    Roll{
        source: String,
        roll_axis: Dir3,
        weight: f32,
    }
}
