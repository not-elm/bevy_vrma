use crate::vrma::animation::{AnimationPlayerEntities, VrmAnimationGraph};
use bevy::app::{App, Update};
use bevy::hierarchy::Parent;
use bevy::prelude::{
    Added, AnimationGraphHandle, AnimationPlayer, Commands, Entity, Plugin, Query,
};

/// At the timing when the spawn of the Vrma's animation player is completed,
/// register the animation graph and associate the Player's entity with the root entity.
/// register the animation graph and associate the Player's entity with the root entity.
pub struct VrmaAnimationSetupPlugin;

impl Plugin for VrmaAnimationSetupPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (setup,));
    }
}

fn setup(
    mut commands: Commands,
    mut vrma: Query<(&mut AnimationPlayerEntities, &VrmAnimationGraph)>,
    players: Query<Entity, Added<AnimationPlayer>>,
    parents: Query<&Parent>,
) {
    for player_entity in players.iter() {
        let mut entity = player_entity;
        loop {
            if let Ok((mut players, animation_graph)) = vrma.get_mut(entity) {
                players.push(player_entity);
                commands
                    .entity(player_entity)
                    .insert(AnimationGraphHandle(animation_graph.handle.clone()));
                break;
            }

            if let Ok(parent) = parents.get(entity) {
                entity = parent.get();
            } else {
                break;
            }
        }
    }
}
