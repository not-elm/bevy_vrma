use crate::prelude::ChildSearcher;
use bevy::ecs::system::SystemParam;
use bevy::prelude::{AnimationPlayer, Children, Entity, Query, Reflect};

#[derive(SystemParam, Reflect)]
pub struct VrmAnimation<'w, 's> {
    searcher: ChildSearcher<'w, 's>,
    players: Query<'w, 's, &'static AnimationPlayer>,
    childrens: Query<'w, 's, &'static Children>,
}

impl VrmAnimation<'_, '_> {
    pub fn all_finished(
        &self,
        vrm: Entity,
    ) -> bool {
        if let Some(root_bone) = self.searcher.find_root_bone(vrm)
            && let Ok(animation_player) = self.players.get(root_bone)
            && !animation_player.all_finished()
        {
            return false;
        }
        self.finished_expressions(vrm)
    }

    pub fn finished_humanoid_bones(
        &self,
        vrm: Entity,
    ) -> bool {
        if let Some(root_bone) = self.searcher.find_root_bone(vrm)
            && let Ok(animation_player) = self.players.get(root_bone)
            && !animation_player.all_finished()
        {
            false
        } else {
            true
        }
    }

    pub fn finished_expressions(
        &self,
        vrm: Entity,
    ) -> bool {
        let Some(expressions_root) = self.searcher.find_expressions_root(vrm) else {
            return true;
        };
        let Ok(children) = self.childrens.get(expressions_root) else {
            return true;
        };
        for expression in children {
            if self
                .players
                .get(*expression)
                .is_ok_and(|player| !player.all_finished())
            {
                return false;
            }
        }
        true
    }
}
