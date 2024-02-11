
mod system;
mod animations;
mod player;
mod interactable;

use std::borrow::Borrow;
use std::string;

use animations::{update_animation, AnimatableLayer};

use bevy::app::{AppExit, PluginGroup, StateTransition, Update};
use bevy::asset::{AssetId, AssetServer, Assets, RecursiveDependencyLoadState};
use bevy::core_pipeline::core_2d::Camera2dBundle;
use bevy::ecs::bundle::Bundle;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::event::{EventReader, EventWriter};
use bevy::ecs::query::{Changed, With, Without};
use bevy::ecs::schedule::common_conditions::{resource_equals, resource_exists, run_once};
use bevy::ecs::schedule::{Condition, NextState, OnEnter, OnExit, OnTransition, State, SystemSet};
use bevy::ecs::system::{Commands, Query, ResMut, RunSystemOnce};
use bevy::ecs::world::World;
use bevy::hierarchy::DespawnRecursiveExt;
use bevy::input::keyboard::KeyCode;
use bevy::input::mouse::{MouseButton, MouseButtonInput};
use bevy::input::Input;
use bevy::log::info;
use bevy::math::{Vec2, Vec2Swizzles};
use bevy::pbr::PointLightBundle;
use bevy::prelude::{Deref, DerefMut};
use bevy::reflect::Reflect;
use bevy::render::color::Color;
use bevy::render::texture::{Image, ImagePlugin};
use bevy::scene::{DynamicScene, DynamicSceneBundle, Scene, SceneBundle, SceneSpawner};
use bevy::sprite::{Anchor, Sprite, SpriteBundle, SpriteSheetBundle, TextureAtlasBuilder, TextureAtlasSprite};
use bevy::text::{Text, TextAlignment, TextStyle};
use bevy::time::{Timer, TimerMode};
use bevy::transform::components::Transform;
use bevy::transform::TransformBundle;
use bevy::ui::node_bundles::{ButtonBundle, TextBundle};
use bevy::ui::widget::Button;
use bevy::ui::{AlignItems, Interaction, JustifyContent, JustifySelf, PositionType, Style, UiRect, Val};
use bevy::utils::hashbrown::Equivalent;
use bevy::utils::HashMap;
use bevy::window::{PrimaryWindow, Window, WindowPlugin, WindowResolution};
use bevy::{app::App, asset::Handle, ecs::{schedule::{common_conditions::in_state, IntoSystemConfigs, States}, system::{Res, Resource}}, sprite::TextureAtlas, text::Font, DefaultPlugins};
use bevy_asset_loader::{asset_collection::AssetCollection, loading_state::{config::ConfigureLoadingState, LoadingState, LoadingStateAppExt}, standard_dynamic_asset::StandardDynamicAssetCollection};
use bevy_pixel_camera::{PixelCameraPlugin, PixelViewport, PixelZoom};
use bevy_xpbd_2d::components::{CoefficientCombine, Collider, CollidingEntities, CollisionLayers, LinearVelocity, LockedAxes, Restitution, RigidBody, Sensor};
use bevy_xpbd_2d::math::Vector;
use bevy_xpbd_2d::plugins::debug::PhysicsDebugConfig;
use bevy_xpbd_2d::plugins::spatial_query::{ShapeCaster, ShapeHits, SpatialQueryFilter};
use bevy_xpbd_2d::plugins::{PhysicsDebugPlugin, PhysicsPlugins};
use bevy_xpbd_2d::prelude::PhysicsLayer;
use bevy_xpbd_2d::resources::Gravity;
use interactable::{clear_quiz_buttons, interact_with_gobject, interact_with_menu_button, interact_with_quiz_button, make_uninteractable, update_player_interaction, GroundObject, Interactivity, MenuButtonAction, QuizButton, QuizButtonData};
use leafwing_input_manager::action_state::ActionState;
use leafwing_input_manager::input_map::InputMap;
use leafwing_input_manager::plugin::InputManagerPlugin;
use leafwing_input_manager::{Actionlike, InputManagerBundle};
use player::{update_player_movement, Layer, Player, PlayerAction};
use system::{cleanup_after_state, next_level, CurrentLevel, GameState, QuizClear};

#[derive(AssetCollection, Resource)]
struct MenuAssets
{    
    #[asset(key = "loading_icon")]
    loading_icon: Handle<TextureAtlas>,

    #[asset(key = "loading_font")]
    loading_font: Handle<Font>,
}

#[derive(AssetCollection, Resource)]
struct GameAssets
{
    #[asset(key = "menu_bg")]
    menu_bg: Handle<TextureAtlas>,

    #[asset(key = "game_over_bg")]
    game_over_bg: Handle<TextureAtlas>,

    #[asset(key = "full_completion_bg")]
    full_completion_bg: Handle<TextureAtlas>,

    #[asset(key = "main_font")]
    main_font: Handle<Font>,

    #[asset(key = "sonic")]
    sonic: Handle<TextureAtlas>,
}

fn main()
{
    App::new()
        .add_state::<GameState>()
        .insert_resource::<CurrentLevel>(CurrentLevel(1))
        .insert_resource::<QuizClear>(QuizClear(false))
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
        .add_plugins(InputManagerPlugin::<PlayerAction>::default())
        .add_plugins((PhysicsPlugins::default(), PhysicsDebugPlugin::default()))
            .insert_resource(Gravity(Vec2::NEG_Y * 100.0))
            .insert_resource(PhysicsDebugConfig {
                aabb_color: Some(Color::WHITE),
                ..Default::default()
            })
        .add_loading_state(
            LoadingState::new(GameState::PreLoading)
                .continue_to_state(GameState::AssetLoading)
                .with_dynamic_assets_file::<StandardDynamicAssetCollection>("preload_assets.assets.ron")
                .load_collection::<MenuAssets>()
        )
        .add_loading_state(
            LoadingState::new(GameState::AssetLoading)
                .continue_to_state(GameState::MainMenu)
                .with_dynamic_assets_file::<StandardDynamicAssetCollection>("game_assets.assets.ron")
                .load_collection::<GameAssets>()
        )
        .add_systems(StateTransition, cleanup_after_state)
        .add_systems(OnExit(GameState::PreLoading), (camera_setup, preload))
        .add_systems(OnEnter(GameState::MainMenu), setup_menu)
        .add_systems(Update, update_animation)
        .add_systems(Update,
            interact_with_menu_button.run_if(
                in_state(GameState::MainMenu)
                    .or_else(in_state(GameState::GameOver))
                    .or_else(in_state(GameState::FullCompletion))
                    .or_else(in_state(GameState::InGame))
        ))
        .add_systems(OnEnter(GameState::LevelCompleted), next_level)
        .add_systems(OnEnter(GameState::InGame), spawn_player)
        .add_systems(OnEnter(GameState::InGame), level_1.run_if(
            resource_exists::<CurrentLevel>().and_then(resource_equals(CurrentLevel(1)))))
        .add_systems(OnEnter(GameState::GameOver), setup_menu)
        .add_systems(OnEnter(GameState::FullCompletion), setup_menu)
        .add_systems(Update, (make_uninteractable, clear_quiz_buttons).chain().run_if(resource_equals(QuizClear(true))))
        .add_systems(Update,
            (update_player_interaction,
                update_player_movement,
                interact_with_gobject,
                interact_with_quiz_button)
                    .run_if(in_state(GameState::InGame)))
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

    commands.spawn(
        PointLightBundle
        {
            transform: Transform::from_xyz(5.0, 5.0, 5.0),
            ..Default::default()
        }
    );
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
            current_animation: 0,
            next_animation: 0,
            flip_x: false,
            repeat: true
        },
        GameState::AssetLoading
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
        }),
        GameState::AssetLoading
    ));
}

fn setup_menu(
    mut commands: Commands,
    game_state: Res<State<GameState>>,
    game_assets: Res<GameAssets>)
{
    let current_state = *game_state.get();
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
            texture_atlas: match current_state
            {
                GameState::GameOver => game_assets.game_over_bg.clone(),
                GameState::FullCompletion => game_assets.full_completion_bg.clone(),
                _ => game_assets.menu_bg.clone(),
            },
            sprite: sprite.clone(),
            ..Default::default()
        },
        AnimatableLayer
        {
            timer: Timer::from_seconds(0.125, TimerMode::Repeating),
            animations: vec![(0, 10)],
            current_animation: 0,
            next_animation: 0,
            flip_x: false,
            repeat: true
        },
        current_state
    ));

    commands.spawn((
        TextBundle::from_section(
            match current_state
            {
                GameState::GameOver => "Попробовать снова.",
                GameState::FullCompletion => "Поздравляем",
                _ => "Начать!"
            },
            TextStyle {
                font: game_assets.main_font.clone(),
                font_size: 100.0,
                ..Default::default()
            },
        )
        .with_text_alignment(TextAlignment::Center)
        .with_style(Style {
            justify_self: JustifySelf::Center,
            align_self: bevy::ui::AlignSelf::Center,
            ..Default::default()
        }),
        current_state
    ));
    
    commands.spawn((
        ButtonBundle {
            style: Style {
                width: Val::Px(250.0),
                height: Val::Px(65.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            background_color: Color::rgb(0.85, 0.61, 0.38).into(),
            ..Default::default()
        },
        MenuButtonAction::Play,
        current_state
    ));
    
    commands.spawn((
        ButtonBundle {
            style: Style {
                width: Val::Px(100.0),
                height: Val::Px(65.0),
                top: Val::Percent(50.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::End,
                ..Default::default()
            },
            background_color: Color::rgb(1., 0.61, 0.38).into(),
            ..Default::default()
        },
        if current_state == GameState::MainMenu { MenuButtonAction::Quit } else { MenuButtonAction::BackToMenu },
        current_state
    ));
}

fn spawn_player(
    mut commands: Commands,
    image_assets: Res<GameAssets>
) {
    let query_filter = SpatialQueryFilter::new()
        .with_masks([Layer::Ground]);

    commands.spawn((
        SpriteSheetBundle
        {
            texture_atlas: image_assets.sonic.clone(),
            sprite: TextureAtlasSprite::default(),
            ..Default::default()
        },
        AnimatableLayer
        {
            timer: Timer::from_seconds(0.125, TimerMode::Repeating),
            animations: vec![(0, 29), (30, 34), (35, 39), (40, 51), (52, 62)],
            current_animation: 0,
            next_animation: 0,
            flip_x: false,
            repeat: true
        },
        InputManagerBundle::<PlayerAction>
        {
            action_state: ActionState::default(),
            input_map: InputMap::new(
                [
                    (KeyCode::Space, PlayerAction::Jump),
                    (KeyCode::W, PlayerAction::Jump),
                    (KeyCode::Up, PlayerAction::Jump),
                    (KeyCode::A, PlayerAction::Left),
                    (KeyCode::Left, PlayerAction::Left),
                    (KeyCode::D, PlayerAction::Right),
                    (KeyCode::Right, PlayerAction::Right)
                ]
            ),
        },
        RigidBody::Dynamic,
        Collider::cuboid(14., 34.2),

        // Prevent the player from falling over
        LockedAxes::new().lock_rotation(),

        // Cast the player shape downwards to detect when the player is grounded
        ShapeCaster::new(
            Collider::cuboid(13.8, 34.),
            Vector::NEG_Y * 0.05,
            0.,
            Vector::NEG_Y,
        )
        .with_query_filter(query_filter)
        .with_max_time_of_impact(0.1)
        .with_max_hits(1),
        
        // This controls how bouncy a rigid body is.
        Restitution::new(0.0).with_combine_rule(CoefficientCombine::Min),
        Player {},
        GameState::InGame,
        CollisionLayers::new([Layer::Player], [Layer::Ground, Layer::Enemy, Layer::Interactable])
    ));
}

fn level_1(
    mut commands: Commands,
    image_assets: Res<GameAssets>
) {
    commands.spawn((
        SpriteSheetBundle
        {
            texture_atlas: image_assets.sonic.clone(),
            sprite: TextureAtlasSprite::default(),
            transform: Transform::from_xyz(0., -25., 0.),
            ..Default::default()
        },
        AnimatableLayer
        {
            timer: Timer::from_seconds(0.125, TimerMode::Repeating),
            animations: vec![(0, 10)],
            current_animation: 0,
            next_animation: 0,
            flip_x: false,
            repeat: true
        },
        RigidBody::Static,
        Collider::cuboid(100., 10.1),
        GameState::InGame,
        CollisionLayers::new([Layer::Ground], [Layer::Player, Layer::Enemy])
    ));

    commands.spawn((
        SpriteSheetBundle
        {
            texture_atlas: image_assets.sonic.clone(),
            sprite: TextureAtlasSprite::default(),
            transform: Transform::from_xyz(80., -25., 0.),
            ..Default::default()
        },
        AnimatableLayer
        {
            timer: Timer::from_seconds(0.125, TimerMode::Repeating),
            animations: vec![(0, 10)],
            current_animation: 0,
            next_animation: 0,
            flip_x: false,
            repeat: true
        },
        RigidBody::Static,
        Collider::cuboid(10.1, 10.1),
        CollisionLayers::new([Layer::Enemy], [Layer::Player]),
        GameState::InGame,
        GroundObject { next_game_state: GameState::LevelCompleted }
    ));

    //spike
    commands.spawn((
        SpriteSheetBundle
        {
            texture_atlas: image_assets.sonic.clone(),
            sprite: TextureAtlasSprite::default(),
            transform: Transform::from_xyz(100., -25., 0.),
            ..Default::default()
        },
        AnimatableLayer
        {
            timer: Timer::from_seconds(0.125, TimerMode::Repeating),
            animations: vec![(0, 10)],
            current_animation: 0,
            next_animation: 0,
            flip_x: false,
            repeat: true
        },
        RigidBody::Static,
        Collider::cuboid(10.1, 10.1),
        CollisionLayers::new([Layer::Enemy], [Layer::Player]),
        GameState::InGame,
        GroundObject { next_game_state: GameState::GameOver }
    ));

    //wall
    let wall = commands.spawn((
        SpriteSheetBundle
        {
            texture_atlas: image_assets.sonic.clone(),
            sprite: TextureAtlasSprite::default(),
            transform: Transform::from_xyz(-20., 0., 0.),
            ..Default::default()
        },
        RigidBody::Static,
        Collider::cuboid(10.1, 10.1),
        CollisionLayers::new([Layer::Ground], [Layer::Player]),
        GameState::InGame
    )).id();

    //interactivity
    commands.spawn((
        SpriteSheetBundle
        {
            texture_atlas: image_assets.sonic.clone(),
            sprite: TextureAtlasSprite::default(),
            transform: Transform::from_xyz(20., 0., 0.),
            ..Default::default()
        },
        AnimatableLayer
        {
            timer: Timer::from_seconds(0.125, TimerMode::Repeating),
            animations: vec![(0, 10)],
            current_animation: 0,
            next_animation: 0,
            flip_x: false,
            repeat: true
        },
        RigidBody::Static,
        Collider::cuboid(10.1, 10.1),
        CollisionLayers::new([Layer::Interactable], [Layer::Player]),
        Sensor,
        GameState::InGame,

        InputManagerBundle::<PlayerAction>
        {
            action_state: ActionState::default(),
            input_map: InputMap::new(
                [
                    (KeyCode::B, PlayerAction::Interact)
                ]
            ),
        },
        Interactivity
        {
            can_interact: true,
            is_interacting: false,
            buttons: [
                QuizButtonData { x: 200., y: 50., /*text: "A".into(),*/ is_correct: false, entity: None },
                QuizButtonData { x: 500., y: 50., /*text: "B".into(),*/ is_correct: false, entity: None },
                QuizButtonData { x: 200., y: 150., /*text: "C".into(),*/ is_correct: true, entity: Some(wall.clone()) },
                QuizButtonData { x: 500., y: 150., /*text: "D".into(),*/ is_correct: false, entity: None },
            ]
        }
    ));
}
