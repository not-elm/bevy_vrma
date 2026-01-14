use bevy::app::{App, Plugin};
use bevy::asset::io::Reader;
use bevy::asset::{Asset, AssetLoader, Handle, LoadContext};
use bevy::gltf::{
    DefaultGltfImageSampler, Gltf, GltfAssetLabel, GltfError, GltfLoader, GltfLoaderSettings,
    extensions::GltfExtensionHandlers,
};
use bevy::image::{
    CompressedImageFormatSupport, CompressedImageFormats, Image, ImageSamplerDescriptor,
};
use bevy::prelude::{AssetApp, Component, TypePath};
use bevy::utils::default;

pub struct VrmLoaderPlugin;

impl Plugin for VrmLoaderPlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.preregister_asset_loader::<VrmLoader>(&["vrm"]);
    }

    fn finish(
        &self,
        app: &mut App,
    ) {
        let supported_compressed_formats =
            if let Some(resource) = app.world().get_resource::<CompressedImageFormatSupport>() {
                resource.0
            } else {
                CompressedImageFormats::NONE
            };
        let default_sampler =
            if let Some(resource) = app.world().get_resource::<DefaultGltfImageSampler>() {
                resource.get_internal()
            } else {
                let resource = DefaultGltfImageSampler::new(&ImageSamplerDescriptor::default());
                let sampler = resource.get_internal();
                app.insert_resource(resource);
                sampler
            };
        let extensions = if let Some(resource) = app.world().get_resource::<GltfExtensionHandlers>()
        {
            resource.0.clone()
        } else {
            let resource = GltfExtensionHandlers::default();
            let handlers = resource.0.clone();
            app.insert_resource(resource);
            handlers
        };
        app.register_asset_loader(VrmLoader(GltfLoader {
            supported_compressed_formats,
            custom_vertex_attributes: Default::default(),
            default_sampler,
            default_convert_coordinates: Default::default(),
            extensions,
        }));
    }
}

/// A handle to load a VRM.
/// This component is removed after the VRM is loaded.
///
/// This handle is used to load a VRM, and after it is loaded, the following components are automatically inserted:
///
/// - [`Vrm`](crate::prelude::Vrm)
/// - [`VrmPath`](crate::prelude::VrmPath)
/// - [`BoneRestTransform`](crate::prelude::RestTransform)
/// - [`BoneRestGlobalTransform`](crate::prelude::RestGlobalTransform)
/// - [`SceneRoot`](bevy::scene::SceneRoot)
/// - Components hold the entity of each bone, refer to [here](crate::vrm::humanoid_bone) for more details.
///
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_vrm1::prelude::*;
///
/// fn spawn_vrm(
///     mut commands: Commands,
///    asset_server: Res<AssetServer>,
/// ){
///     commands.spawn(VrmHandle(asset_server.load("<vrm>.vrm")));
/// }
/// ```
#[derive(Debug, Component)]
pub struct VrmHandle(pub Handle<VrmAsset>);

#[derive(Debug, Asset, TypePath)]
pub struct VrmAsset {
    pub(crate) gltf: Gltf,
    pub(crate) images: Vec<Handle<Image>>,
}

#[derive(TypePath)]
struct VrmLoader(GltfLoader);

impl AssetLoader for VrmLoader {
    type Asset = VrmAsset;
    type Settings = ();
    type Error = GltfError;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        _: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let settings = GltfLoaderSettings {
            include_source: true,
            ..default()
        };
        let gltf = self.0.load(reader, &settings, load_context).await?;
        Ok(VrmAsset {
            images: gltf
                .source
                .as_ref()
                .unwrap()
                .textures()
                .map(|tex| {
                    load_context.get_label_handle(GltfAssetLabel::Texture(tex.index()).to_string())
                })
                .collect(),
            gltf,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["vrm"]
    }
}
