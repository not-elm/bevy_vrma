use crate::vrm::humanoid_bone::HumanoidBoneRegistry;
use crate::vrm::{Vrm, VrmBone};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[derive(SystemParam)]
pub struct ChildSearcher<'w, 's> {
    entities: Query<
        'w,
        's,
        (
            Option<&'static Name>,
            Option<&'static VrmBone>,
            Option<&'static Children>,
        ),
    >,
}

impl ChildSearcher<'_, '_> {
    #[inline]
    pub fn find_root_bone(
        &self,
        vrm: Entity,
    ) -> Option<Entity> {
        self.find_from_name(vrm, Vrm::ROOT_BONE)
    }

    #[inline]
    pub fn find_expressions_root(
        &self,
        vrm: Entity,
    ) -> Option<Entity> {
        self.find_from_name(vrm, Vrm::EXPRESSIONS_ROOT)
    }

    pub fn find_from_name(
        &self,
        root: Entity,
        target_name: &str,
    ) -> Option<Entity> {
        find_entity(target_name, false, root, &self.entities)
    }

    pub fn find_by_bone_name(
        &self,
        root: Entity,
        target_name: &VrmBone,
    ) -> Option<Entity> {
        find_entity(target_name, true, root, &self.entities)
    }

    pub(crate) fn has_been_spawned_all_bones(
        &self,
        root: Entity,
        bone_registry: &HumanoidBoneRegistry,
    ) -> bool {
        bone_registry
            .values()
            .all(|bone_name| self.find_from_name(root, bone_name.as_str()).is_some())
    }
}

fn find_entity(
    target_name: &str,
    is_bone: bool,
    entity: Entity,
    entities: &Query<(Option<&Name>, Option<&VrmBone>, Option<&Children>)>,
) -> Option<Entity> {
    let (name, bone, children) = entities.get(entity).ok()?;
    if is_bone {
        if bone.is_some_and(|bone| bone.0 == target_name) {
            return Some(entity);
        }
    } else if name.is_some_and(|name| name.as_str() == target_name) {
        return Some(entity);
    }

    for child in children? {
        if let Some(entity) = find_entity(target_name, is_bone, *child, entities) {
            return Some(entity);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::tests::test_app;
    use bevy::prelude::*;
    use bevy_test_helper::system::SystemExt;

    #[test]
    fn test_find_root_bone() {
        let mut app = test_app();

        let vrm = app.world_mut().spawn_empty().id();
        app.world_mut()
            .commands()
            .entity(vrm)
            .with_child(Name::new(Vrm::ROOT_BONE));
        app.update();

        app.run_system_once(move |s: ChildSearcher| s.find_root_bone(vrm))
            .expect("Failed to find root bone");
    }
}
