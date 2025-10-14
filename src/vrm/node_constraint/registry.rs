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
    gltf: &Gltf,
    node_assets: &Assets<GltfNode>,
    node: &VrmcNodeConstraint,
) -> Vec<Constraint> {
    let mut constraints = vec![];
    let nodes = &gltf.nodes;
    if let Some(rotation_constraint) = parse_rotation_constraint(node, nodes, node_assets) {
        constraints.push(rotation_constraint);
    }
    if let Some(roll_constraint) = parse_roll_constraint(node, nodes, node_assets) {
        constraints.push(roll_constraint);
    }
    if let Some(aim_constraint) = parse_aim_constraint(node, nodes, node_assets) {
        constraints.push(aim_constraint);
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
    Rotation {
        source: String,
        weight: f32,
    },
    Roll {
        source: String,
        roll_axis: Dir3,
        weight: f32,
    },
    Aim {
        source: String,
        aim_axis: Dir3,
        weight: f32,
    },
}

fn parse_rotation_constraint(
    node: &VrmcNodeConstraint,
    nodes: &[Handle<GltfNode>],
    node_assets: &Assets<GltfNode>,
) -> Option<Constraint> {
    let rotation = node.constraint.rotation.as_ref()?;
    let source_handle = nodes.get(rotation.source)?;
    let source = node_assets.get(source_handle)?;
    Some(Constraint::Rotation {
        source: source.name.clone(),
        weight: rotation.weight,
    })
}

fn parse_roll_constraint(
    node: &VrmcNodeConstraint,
    nodes: &[Handle<GltfNode>],
    node_assets: &Assets<GltfNode>,
) -> Option<Constraint> {
    let roll = node.constraint.roll.as_ref()?;
    let source_handle = nodes.get(roll.source)?;
    let source = node_assets.get(source_handle)?;
    let roll_axis = match roll.roll_axis.as_str() {
        "X" => Dir3::X,
        "Y" => Dir3::Y,
        "Z" => Dir3::Z,
        _ => return None,
    };
    Some(Constraint::Roll {
        source: source.name.clone(),
        roll_axis,
        weight: roll.weight,
    })
}

fn parse_aim_constraint(
    node: &VrmcNodeConstraint,
    nodes: &[Handle<GltfNode>],
    node_assets: &Assets<GltfNode>,
) -> Option<Constraint> {
    let aim = node.constraint.aim.as_ref()?;
    let source_handle = nodes.get(aim.source)?;
    let source = node_assets.get(source_handle)?;
    let aim_axis = match aim.aim_axis.as_str() {
        "PositiveX" => Dir3::X,
        "NegativeX" => Dir3::NEG_X,
        "PositiveY" => Dir3::Y,
        "NegativeY" => Dir3::NEG_Y,
        "PositiveZ" => Dir3::Z,
        "NegativeZ" => Dir3::NEG_Z,
        _ => return None,
    };
    Some(Constraint::Aim {
        source: source.name.clone(),
        aim_axis,
        weight: aim.weight,
    })
}
