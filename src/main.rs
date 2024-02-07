
mod animations;
use std::string;

use animations::{update_animation, AnimatableLayer};

use bevy::app::{PluginGroup, StateTransition, Update};
use bevy::asset::{AssetId, AssetServer, Assets, RecursiveDependencyLoadState};
use bevy::core_pipeline::core_2d::Camera2dBundle;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::event::EventReader;
use bevy::ecs::query::{With, Without};
use bevy::ecs::schedule::common_conditions::run_once;
use bevy::ecs::schedule::{Condition, OnEnter, OnExit, SystemSet};
use bevy::ecs::system::{Commands, Query, ResMut, RunSystemOnce};
use bevy::ecs::world::World;
use bevy::hierarchy::DespawnRecursiveExt;
use bevy::input::mouse::{MouseButton, MouseButtonInput};
use bevy::input::Input;
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
use bevy::window::{PrimaryWindow, Window, WindowPlugin, WindowResolution};
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
    #[asset(key = "menu_background")]
    menu_background: Handle<TextureAtlas>,

    #[asset(key = "main_font")]
    main_font: Handle<Font>,

    #[asset(key = "sonic")]
    sonic: Handle<TextureAtlas>,
}

#[derive(Component)]
struct Destructible;

#[derive(Component)]
struct Menu;

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
        .add_systems(OnExit(GameState::PreLoading), (camera_setup, preload))
        .add_systems(OnEnter(GameState::Menu), (cleanup_for_menu, setup_menu))
        .add_systems(Update, update_animation)
        .add_systems(Update, click_on_button.run_if(in_state(GameState::Menu)))
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

    commands.spawn((PointLightBundle {
        transform: Transform::from_xyz(5.0, 5.0, 5.0),
        ..Default::default()
    }));
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
        },
        Destructible
    ));

    commands.spawn((TextBundle::from_section(
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
        }), Destructible));

    commands.spawn((SpriteBundle
        {
            transform: Transform::from_xyz(-48.0, 0.0, 0.0),
            texture: menu_assets.preload_background.clone(),
            ..Default::default()
        }, Destructible));
}

fn cleanup_for_menu(
    mut commands: Commands,
    entities: Query<Entity, (With<Destructible>, Without<Menu>)>,
) {
    for e in entities.iter()
    {
        commands.entity(e).despawn_recursive();
    }
}

fn setup_menu(mut commands: Commands, game_assets: Res<GameAssets>)
{
    let sprite = TextureAtlasSprite
    {
        custom_size: Some(Vec2::new(320., 224.)),
        anchor: Anchor::Center,
        ..Default::default()
    };

    commands.spawn((
        SpriteSheetBundle
        {
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            texture_atlas: game_assets.menu_background.clone(),
            sprite: sprite.clone(),
            ..Default::default()
        },
        AnimatableLayer
        {
            timer: Timer::from_seconds(0.125, TimerMode::Repeating),
            animations: vec![(0, 10)],
            current_animation: 0
        },
        Destructible,
        Menu
    ));

    commands.spawn((TextBundle::from_section(
            "Начать!",
            TextStyle {
                font: game_assets.main_font.clone(),
                font_size: 100.0,
                ..Default::default()
            },
        )
        .with_text_alignment(TextAlignment::Center)
        /*.with_style(Style {
            align_self: AlignSelf::
            position_type: PositionType::Relative,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            ..Default::default()
        })*/, Destructible));
}

//&Window, With<PrimaryWindow>
fn click_on_button(
    q_windows: Query<&Window, With<PrimaryWindow>>,
    buttons: Res<Input<MouseButton>>,
) {
    if buttons.just_released(MouseButton::Left)
    {
        if let Some(position) = q_windows.single().cursor_position()
        {
            
        }
    }
}
