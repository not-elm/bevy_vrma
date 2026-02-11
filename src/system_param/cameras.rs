use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    ecs::system::SystemParam,
    math::{Vec2, Vec3},
    prelude::{Camera, Camera3d, Component, Entity, GlobalTransform, InfinitePlane3d, Query, With},
    window::{PrimaryWindow, WindowRef},
};

pub type CameraQuery<'w> = (
    Entity,
    &'w Camera,
    Option<&'w RenderTarget>,
    &'w GlobalTransform,
    Option<&'w RenderLayers>,
);

#[derive(SystemParam)]
pub struct Cameras<'w, 's, Camera: Component = Camera3d> {
    pub cameras: Query<'w, 's, CameraQuery<'static>, With<Camera>>,
    primary_window: Query<'w, 's, Entity, With<PrimaryWindow>>,
}

impl<Camera: Component> Cameras<'_, '_, Camera> {
    pub fn all_layers(&self) -> RenderLayers {
        self.cameras
            .iter()
            .fold(RenderLayers::none(), |l1, (_, _, _, _, l2)| match l2 {
                Some(l2) => l1 | l2.clone(),
                None => l1,
            })
    }

    #[inline]
    pub fn find_camera_from_window(
        &self,
        window_entity: Entity,
    ) -> Option<CameraQuery<'_>> {
        self.cameras.iter().find(|(_, _, target, _, _)| {
            let target = target.cloned().unwrap_or_default();
            match target {
                RenderTarget::Window(WindowRef::Entity(entity)) => entity == window_entity,
                RenderTarget::Window(WindowRef::Primary) => self
                    .primary_window
                    .single()
                    .is_ok_and(|e| e == window_entity),
                _ => false,
            }
        })
    }

    #[inline]
    pub fn find_by_world(
        &self,
        world_pos: Vec3,
    ) -> Option<CameraQuery<'_>> {
        self.cameras.iter().find(|(_, camera, _, gtf, _)| {
            camera.logical_viewport_rect().is_some_and(|viewport| {
                let Ok(pos) = camera.world_to_viewport(gtf, world_pos) else {
                    return false;
                };
                viewport.contains(pos)
            })
        })
    }

    #[inline]
    pub fn find_camera_from_layers(
        &self,
        layers: &RenderLayers,
    ) -> Option<CameraQuery<'_>> {
        self.cameras
            .iter()
            .find(|(_, _, _, _, layer)| layer.is_some_and(|l| layers.intersects(l)))
    }

    #[inline]
    pub fn to_viewport_pos(
        &self,
        layers: &RenderLayers,
        world_pos: Vec3,
    ) -> Option<Vec2> {
        let (_, camera, _, camera_tf, _) = self.find_camera_from_layers(layers)?;
        camera.world_to_viewport(camera_tf, world_pos).ok()
    }

    #[inline]
    pub fn to_world_by_viewport(
        &self,
        window_entity: Entity,
        viewport_pos: Vec2,
        mascot_pos: Vec3,
    ) -> Option<Vec3> {
        let (_, camera, _, camera_gtf, _) = self.find_camera_from_window(window_entity)?;
        let ray = camera.viewport_to_world(camera_gtf, viewport_pos).ok()?;
        let plane = InfinitePlane3d::new(camera_gtf.back());
        let distance = ray.intersect_plane(mascot_pos, plane)?;
        Some(ray.get_point(distance))
    }

    #[inline]
    pub fn to_world_2d_pos_from_viewport(
        &self,
        window_entity: Entity,
        viewport_pos: Vec2,
    ) -> Option<Vec2> {
        let (_, camera, _, camera_gtf, _) = self.find_camera_from_window(window_entity)?;
        camera.viewport_to_world_2d(camera_gtf, viewport_pos).ok()
    }
}

#[cfg(test)]
mod tests {
    use crate::system_param::cameras::Cameras;
    use crate::tests::{TestResult, test_app};
    use bevy::camera::visibility::RenderLayers;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::{Camera, Camera3d, Commands, GlobalTransform};

    #[test]
    fn test_all_layers() -> TestResult {
        let mut app = test_app();
        app.world_mut()
            .run_system_once(|mut commands: Commands| {
                commands.spawn((
                    Camera::default(),
                    GlobalTransform::default(),
                    RenderLayers::layer(1),
                    Camera3d::default(),
                ));
                commands.spawn((
                    Camera::default(),
                    GlobalTransform::default(),
                    RenderLayers::layer(2),
                    Camera3d::default(),
                ));
            })
            .expect("Failed to run system");
        app.update();

        let layers = app
            .world_mut()
            .run_system_once(|cameras: Cameras| cameras.all_layers())
            .expect("Failed to run system");
        assert_eq!(layers, RenderLayers::from_layers(&[1, 2]));
        Ok(())
    }
}
