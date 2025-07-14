use crate::prelude::ChildSearcher;
use crate::vrma::VrmAnimationNodeIndex;
use bevy::animation::{AnimationPlayer, RepeatAnimation};
use bevy::app::{App, Plugin};
use bevy::prelude::{
    AnimationNodeIndex, AnimationTransitions, ChildOf, Children, Entity, Event, Query, Reflect,
    Transform, Trigger,
};
use std::time::Duration;

/// The trigger event to play the Vrma's animation.
///
/// You need to emit this via [`Trigger`] with the target entity of the VRMA you want to play the animation on.
///
/// If there are multiple VRMA entities, the animation of all other VRMAs will be stopped except for the one specified in the trigger.
#[derive(Event, Debug, Reflect)]
pub struct PlayVrma {
    /// Repetition behavior of an animation.
    /// Default is [`RepeatAnimation::Never`].
    pub repeat: RepeatAnimation,

    /// A time until the existing animation fades out.
    /// Default is 300 milliseconds.
    pub transition_duration: Duration,
}

impl Default for PlayVrma {
    fn default() -> Self {
        Self {
            repeat: RepeatAnimation::Never,
            transition_duration: Duration::from_millis(300),
        }
    }
}

/// The trigger event to stop the Vrma's animation.
///You need to emit this via [`Trigger`] with the target entity of the VRMA you want to stop the animation on.
#[derive(Event, Debug)]
pub struct StopVrma;

pub(super) struct VrmaAnimationPlayPlugin;

impl Plugin for VrmaAnimationPlayPlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.register_type::<PlayVrma>()
            .add_observer(apply_play_vrma)
            .add_observer(apply_stop_vrma);
    }
}

fn apply_play_vrma(
    trigger: Trigger<PlayVrma>,
    mut players: Query<(
        &mut Transform,
        &mut AnimationPlayer,
        Option<&mut AnimationTransitions>,
    )>,
    searcher: ChildSearcher,
    parents: Query<&ChildOf>,
    childrens: Query<&Children>,
    vrmas: Query<&VrmAnimationNodeIndex>,
) {
    let vrma_entity = trigger.target();
    let Ok(ChildOf(vrm_entity)) = parents.get(vrma_entity) else {
        return;
    };
    let Ok(node_index) = vrmas.get(vrma_entity) else {
        return;
    };
    play_humanoid_bone_animation(
        *vrm_entity,
        node_index.0,
        trigger.repeat,
        trigger.transition_duration,
        &searcher,
        &mut players,
    );
    play_expression_animations(
        *vrm_entity,
        node_index.0,
        trigger.repeat,
        &mut players,
        &childrens,
        &searcher,
    );
}

fn play_humanoid_bone_animation(
    vrm: Entity,
    node_index: AnimationNodeIndex,
    repeat: RepeatAnimation,
    transition_duration: Duration,
    searcher: &ChildSearcher,
    players: &mut Query<(
        &mut Transform,
        &mut AnimationPlayer,
        Option<&mut AnimationTransitions>,
    )>,
) {
    let Some(root_bone) = searcher.find_root_bone(vrm) else {
        return;
    };
    let Ok((_, mut player, Some(mut transitions))) = players.get_mut(root_bone) else {
        return;
    };
    transitions
        .play(&mut player, node_index, transition_duration)
        .set_repeat(repeat);
}

fn play_expression_animations(
    vrm: Entity,
    node_index: AnimationNodeIndex,
    repeat: RepeatAnimation,
    entities: &mut Query<(
        &mut Transform,
        &mut AnimationPlayer,
        Option<&mut AnimationTransitions>,
    )>,
    childrens: &Query<&Children>,
    searcher: &ChildSearcher,
) {
    let Some(expressions_root) = searcher.find_expressions_root(vrm) else {
        return;
    };
    let Ok(children) = childrens.get(expressions_root) else {
        return;
    };
    for child in children.iter().copied() {
        if let Ok((mut tf, mut player, _)) = entities.get_mut(child) {
            // Reset the expression weight to zero.
            tf.translation.x = 0.0;
            player.stop_all();
            player.play(node_index).set_repeat(repeat);
        };
    }
}

fn apply_stop_vrma(
    trigger: Trigger<StopVrma>,
    mut rig_entities: Query<&mut AnimationPlayer>,
    vrmas: Query<&VrmAnimationNodeIndex>,
    rig_children: Query<&Children>,
) {
    let vrma_entity = trigger.target();
    let Ok(node_index) = vrmas.get(vrma_entity) else {
        return;
    };
    stop_animations(vrma_entity, node_index.0, &mut rig_entities, &rig_children);
}

fn stop_animations(
    entity: Entity,
    node_index: AnimationNodeIndex,
    rig_entities: &mut Query<&mut AnimationPlayer>,
    rig_children: &Query<&Children>,
) {
    if let Ok(mut player) = rig_entities.get_mut(entity) {
        player.stop(node_index);
    };
    if let Ok(children) = rig_children.get(entity) {
        for child in children.iter().copied() {
            stop_animations(child, node_index, rig_entities, rig_children);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::tests::test_app;
    use crate::vrma::VrmAnimationNodeIndex;
    use crate::vrma::animation::play::VrmaAnimationPlayPlugin;
    use bevy::prelude::*;
    use bevy_test_helper::system::SystemExt;

    #[test]
    fn test_play_vrma() {
        let mut app = test_app();
        app.add_plugins(VrmaAnimationPlayPlugin);

        let vrm = app.world_mut().spawn_empty().id();
        let vrma = app.world_mut().spawn(VrmAnimationNodeIndex::default()).id();
        app.world_mut().commands().entity(vrm).add_child(vrma);

        app.world_mut().commands().entity(vrm).with_child((
            Name::new(Vrm::ROOT_BONE),
            Transform::default(),
            AnimationPlayer::default(),
            AnimationTransitions::default(),
        ));

        app.world_mut()
            .commands()
            .entity(vrma)
            .trigger(PlayVrma::default());
        app.update();

        app.run_system_once(|player: Query<&AnimationPlayer>| {
            let player = player.single().expect("Failed to find AnimationPlayer");
            assert!(!player.all_finished());
        });
    }

    #[test]
    fn test_stop_vrma() {
        let mut app = test_app();
        app.add_plugins(VrmaAnimationPlayPlugin);

        let vrm = app.world_mut().spawn_empty().id();
        let vrma = app.world_mut().spawn(VrmAnimationNodeIndex::default()).id();
        app.world_mut().commands().entity(vrm).add_child(vrma);

        app.world_mut().commands().entity(vrm).with_child((
            Name::new(Vrm::ROOT_BONE),
            AnimationPlayer::default(),
            AnimationTransitions::default(),
        ));

        app.world_mut()
            .commands()
            .entity(vrma)
            .trigger(PlayVrma::default());
        app.update();

        app.world_mut().commands().entity(vrma).trigger(StopVrma);
        app.update();

        app.run_system_once(|player: Query<&AnimationPlayer>| {
            let player = player.single().expect("Failed to find AnimationPlayer");
            assert!(player.all_finished());
        });
    }
}
