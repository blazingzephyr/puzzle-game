
mod animations;
use animations::{update_animation, AnimatableLayer};

use bevy::app::{PluginGroup, StateTransition, Update};
use bevy::asset::{AssetId, AssetServer, Assets, RecursiveDependencyLoadState};
use bevy::core_pipeline::core_2d::Camera2dBundle;
use bevy::ecs::schedule::common_conditions::run_once;
use bevy::ecs::schedule::{Condition, SystemSet};
use bevy::ecs::system::{Commands, Query, RunSystemOnce};
use bevy::ecs::world::World;
use bevy::math::Vec2;
use bevy::pbr::PointLightBundle;
use bevy::render::texture::{Image, ImagePlugin};
use bevy::scene::{DynamicScene, DynamicSceneBundle, Scene, SceneBundle, SceneSpawner};
use bevy::sprite::{Anchor, SpriteBundle, SpriteSheetBundle, TextureAtlasBuilder, TextureAtlasSprite};
use bevy::text::{TextAlignment, TextStyle};
use bevy::time::{Timer, TimerMode};
use bevy::transform::components::Transform;
use bevy::transform::TransformBundle;
use bevy::ui::node_bundles::TextBundle;
use bevy::ui::{PositionType, Style, Val};
use bevy::utils::HashMap;
use bevy::window::{Window, WindowPlugin, WindowResolution};
use bevy::{app::App, asset::Handle, ecs::{schedule::{common_conditions::in_state, IntoSystemConfigs, States}, system::{Res, Resource}}, sprite::TextureAtlas, text::Font, DefaultPlugins};
use bevy_asset_loader::{asset_collection::AssetCollection, loading_state::{config::ConfigureLoadingState, LoadingState, LoadingStateAppExt}, standard_dynamic_asset::StandardDynamicAssetCollection};
use bevy_pixel_camera::{PixelCameraPlugin, PixelViewport, PixelZoom};

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
enum GameState
{
    #[default]
    PreLoading,
    AssetLoading,
    Menu,
    InGame
}

#[derive(AssetCollection, Resource)]
struct MenuAssets
{
    #[asset(key = "preload_background")]
    preload_background: Handle<Image>,
    
    #[asset(key = "loading_icon")]
    loading_icon: Handle<TextureAtlas>,

    #[asset(key = "loading_font")]
    loading_font: Handle<Font>,
}

#[derive(AssetCollection, Resource)]
struct GameAssets
{
    
}

fn main()
{
    App::new()
        .add_state::<GameState>()
        .add_plugins(DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: String::from("Puzzle Game"),
                    resolution: WindowResolution::new(960., 672.),
                    ..Default::default()
                }),
                ..Default::default()
            }))
        .add_plugins(PixelCameraPlugin)
        .add_loading_state(
            LoadingState::new(GameState::PreLoading)
                .continue_to_state(GameState::AssetLoading)
                .with_dynamic_assets_file::<StandardDynamicAssetCollection>("preload_assets.assets.ron")
                .load_collection::<MenuAssets>()
        )
        .add_loading_state(
            LoadingState::new(GameState::AssetLoading)
                .continue_to_state(GameState::Menu)
                .with_dynamic_assets_file::<StandardDynamicAssetCollection>("game_assets.assets.ron")
                .load_collection::<GameAssets>()
        )
        .add_systems(Update, (camera_setup, preload).run_if(in_state(GameState::AssetLoading).and_then(run_once())))
        .add_systems(Update, update_animation)
        .run();
}

fn camera_setup(mut commands: Commands)
{
    commands.spawn((
        Camera2dBundle::default(),
        PixelZoom::FitSize {
            width: 320,
            height: 224,
        },
        PixelViewport
    ));

    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(5.0, 5.0, 5.0),
        ..Default::default()
    });
}

fn preload(mut commands: Commands, menu_assets: Res<MenuAssets>)
{
    let sprite = TextureAtlasSprite
    {
        custom_size: Some(Vec2::new(49., 56.)),
        anchor: Anchor::Center,
        ..Default::default()
    };

    commands.spawn((
        SpriteSheetBundle
        {
            transform: Transform::from_xyz(135.0, -80.0, 0.0),
            texture_atlas: menu_assets.loading_icon.clone(),
            sprite: sprite.clone(),
            ..Default::default()
        },
        AnimatableLayer
        {
            timer: Timer::from_seconds(0.125, TimerMode::Repeating),
            animations: vec![(0, 30)],
            current_animation: 0
        }
    ));

    commands.spawn(TextBundle::from_section(
            "Загрузка...",
            TextStyle {
                font: menu_assets.loading_font.clone(),
                font_size: 25.0,
                ..Default::default()
            },
        )
        .with_text_alignment(TextAlignment::Center)
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(5.0),
            right: Val::Px(5.0),
            ..Default::default()
        }));

    commands.spawn(SpriteBundle
        {
            transform: Transform::from_xyz(-48.0, 0.0, 0.0),
            texture: menu_assets.preload_background.clone(),
            ..Default::default()
        });
}