#![windows_subsystem = "windows"]

mod system;
mod animations;
mod player;
mod interactable;
mod assets;

use std::borrow::Borrow;
use std::string;

use animations::{update_animation, AnimatableLayer};

use assets::{GameAssets, MenuAssets};
use bevy::app::{AppExit, PluginGroup, Startup, StateTransition, Update};
use bevy::asset::{AssetId, AssetServer, Assets, RecursiveDependencyLoadState};
use bevy::core_pipeline::core_2d::Camera2dBundle;
use bevy::ecs::bundle::Bundle;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::event::{EventReader, EventWriter};
use bevy::ecs::query::{Changed, With, Without};
use bevy::ecs::schedule::common_conditions::{resource_equals, resource_exists, run_once};
use bevy::ecs::schedule::{Condition, NextState, OnEnter, OnExit, OnTransition, State, SystemSet};
use bevy::ecs::system::{Commands, NonSend, Query, ResMut, RunSystemOnce};
use bevy::ecs::world::World;
use bevy::hierarchy::{BuildChildren, DespawnRecursiveExt};
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
use bevy::text::{self, Text, TextAlignment, TextStyle};
use bevy::time::{Timer, TimerMode};
use bevy::transform::components::Transform;
use bevy::transform::TransformBundle;
use bevy::ui::node_bundles::{ButtonBundle, NodeBundle, TextBundle};
use bevy::ui::widget::Button;
use bevy::ui::{AlignItems, Display, GridPlacement, Interaction, JustifyContent, JustifySelf, PositionType, Style, UiRect, Val};
use bevy::utils::hashbrown::Equivalent;
use bevy::utils::HashMap;
use bevy::window::{PrimaryWindow, Window, WindowPlugin, WindowResolution};
use bevy::winit::WinitWindows;
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
use interactable::{clear_quiz_buttons, interact_with_gobject, interact_with_menu_button, interact_with_quiz_button, make_uninteractable, update_player_interaction, GroundObject, Interactivity, MenuButtonAction, QuestionData, QuizButton, QuizButtonData};
use leafwing_input_manager::action_state::ActionState;
use leafwing_input_manager::input_map::InputMap;
use leafwing_input_manager::plugin::InputManagerPlugin;
use leafwing_input_manager::{Actionlike, InputManagerBundle};
use player::{update_player_movement, Layer, Player, PlayerAction};
use system::{cleanup_after_state, next_level, CurrentLevel, GameState, QuizClear};
use winit::window::Icon;

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
                    title: String::from("Sonic's Eduquest '24"),
                    resolution: WindowResolution::new(1280., 896.),
                    ..Default::default()
                }),
                ..Default::default()
            }))
        .add_plugins(PixelCameraPlugin)
        .add_plugins(InputManagerPlugin::<PlayerAction>::default())
        .add_plugins(PhysicsPlugins::default())
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
        .add_systems(OnExit(GameState::PreLoading), (set_app_icon, camera_setup, preload))
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
            .add_systems(OnEnter(GameState::InGame), level_2.run_if(
                resource_exists::<CurrentLevel>().and_then(resource_equals(CurrentLevel(2)))))
        .add_systems(OnEnter(GameState::GameOver), setup_menu)
        .add_systems(OnEnter(GameState::FullCompletion), setup_menu)
        .add_systems(Update, (make_uninteractable, clear_quiz_buttons).chain().run_if(resource_exists::<QuizClear>()))
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

fn set_app_icon(
    menu_assets: Res<MenuAssets>,
    images: Res<Assets<Image>>,
    windows: NonSend<WinitWindows>,
)
{
    if let Some(app_icon) = images.get(&menu_assets.app_icon)
    {
        let icon = Icon::from_rgba(app_icon.clone().data, app_icon.size().x, app_icon.size().y).unwrap();
        for window in windows.windows.values()
        {
            window.set_window_icon(Some(icon.clone()));
        }
    }
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
    let sprite = Sprite
    {
        custom_size: Some(Vec2::new(320., 224.)),
        anchor: Anchor::Center,
        ..Default::default()
    };

    let text_style = TextStyle {
        font: game_assets.main_font.clone(),
        font_size: 80.0,
        color: Color::BLACK.into(),
        ..Default::default()
    };

    let label_style = Style {
        justify_self: JustifySelf::Center,
        align_self: bevy::ui::AlignSelf::Center,
        ..Default::default()
    };

    let button_bundle = ButtonBundle
    {
        style: Style {
            width: Val::Px(500.0),
            height: Val::Px(85.0),
            margin: UiRect::axes(Val::Px(40.0), Val::Px(1.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..Default::default()
        },
        background_color: Color::rgb(0.977, 0.875, 0.584).into(),
        ..Default::default()
    };

    commands.spawn((
        SpriteBundle
        {
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            texture: match current_state
            {
                GameState::GameOver => game_assets.game_over_bg.clone(),
                GameState::FullCompletion => game_assets.full_completion_bg.clone(),
                _ => game_assets.menu_bg.clone(),
            },
            sprite: sprite.clone(),
            ..Default::default()
        },
        current_state
    ));

    commands
        .spawn(
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..Default::default()
                },
                ..Default::default()
            })
        .with_children(
            |parent|
            {
                if (current_state != GameState::FullCompletion)
                {
                    parent.spawn(
                        (
                            button_bundle.clone(),
                            MenuButtonAction::Play,
                            current_state
                        )
                    )
                    .with_children(
                        |parent|
                        {
                            parent.spawn((
                                TextBundle::from_section(
                                    match current_state
                                    {
                                        GameState::GameOver => "Ещё раз",
                                        _ => "Начать"
                                    },
                                    text_style.clone(),
                                )
                                .with_text_alignment(TextAlignment::Center)
                                .with_style(label_style.clone()),
                                current_state
                            ));
                        }
                    );
                }

                parent.spawn(
                    (
                        button_bundle.clone(),
                        if current_state == GameState::MainMenu { MenuButtonAction::Quit } else { MenuButtonAction::BackToMenu },
                        current_state
                    )
                )
                .with_children(
                    |parent|
                    {
                        parent.spawn((
                            TextBundle::from_section(
                                if current_state == GameState::MainMenu { "Выйти" } else { "Выход в меню" },
                                text_style.clone())
                                    .with_text_alignment(TextAlignment::Center)
                                    .with_style(label_style.clone()),
                            current_state
                        ));
                    }
                );
            }
        );

    if current_state == GameState::GameOver || current_state == GameState::FullCompletion
    {
        let mut top_style = text_style.clone();
        top_style.color = Color::WHITE;

        let mut top_label = label_style.clone();
        top_label.top = Val::Percent(-15.);

        commands.spawn((
            TextBundle::from_section(
                if current_state == GameState::GameOver { "Соник, ты что творишь?!" } else { "Ты победил, Соник!" },
                top_style.clone(),
            )
            .with_text_alignment(TextAlignment::Center)
            .with_style(top_label.clone()),
            current_state
        ));

    }
}

fn spawn_player(
    mut commands: Commands,
    image_assets: Res<GameAssets>,
    current_level: Res<CurrentLevel>
) {
    let query_filter = SpatialQueryFilter::new()
        .with_masks([Layer::Ground]);

    commands.spawn((
        SpriteSheetBundle
        {
            transform: Transform::from_xyz(
                if current_level.0 == 1 { 0. } else { -75. },
                if current_level.0 == 1 { 0. } else { 100. },
                1.),
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
    // Background
    commands.spawn((
        SpriteBundle
        {
            texture: image_assets.level1.clone(),
            transform: Transform::from_xyz(0., 0., -1.),
            ..Default::default()
        },
        GameState::InGame
    ));

    // Ground
    commands.spawn((
        SpriteSheetBundle
        {
            visibility: bevy::render::view::Visibility::Hidden,
            transform: Transform::from_xyz(-70., -25., 0.),
            ..Default::default()
        },
        RigidBody::Static,
        Collider::cuboid(150., 10.1),
        GameState::InGame,
        CollisionLayers::new([Layer::Ground], [Layer::Player, Layer::Enemy])
    ));

    // Ground 2
    commands.spawn((
        SpriteSheetBundle
        {
            visibility: bevy::render::view::Visibility::Hidden,
            transform: Transform::from_xyz(-40., 30., 0.),
            ..Default::default()
        },
        RigidBody::Static,
        Collider::cuboid(90., 10.1),
        GameState::InGame,
        CollisionLayers::new([Layer::Ground], [Layer::Player, Layer::Enemy])
    ));

    // Ground 3
    commands.spawn((
        SpriteSheetBundle
        {
            visibility: bevy::render::view::Visibility::Hidden,
            transform: Transform::from_xyz(-95., 20., 0.),
            ..Default::default()
        },
        RigidBody::Static,
        Collider::cuboid(20., 10.1),
        GameState::InGame,
        CollisionLayers::new([Layer::Ground], [Layer::Player, Layer::Enemy])
    ));

    // Uninteractable wall 1
    commands.spawn((
        SpriteSheetBundle
        {
            visibility: bevy::render::view::Visibility::Hidden,
            transform: Transform::from_xyz(-150., 145., 0.),
            ..Default::default()
        },
        RigidBody::Static,
        Collider::cuboid(10.1, 350.),
        CollisionLayers::new([Layer::Ground], [Layer::Player]),
        GameState::InGame
    ));

    // Spike 0
    commands.spawn((
        SpriteSheetBundle
        {
            visibility: bevy::render::view::Visibility::Hidden,
            transform: Transform::from_xyz(-75., 45., 0.),
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
        Collider::cuboid(15.1, 15.),
        CollisionLayers::new([Layer::Enemy], [Layer::Player]),
        GameState::InGame,
        GroundObject { next_game_state: GameState::GameOver }
    ));

    // Wall
    let wall = commands.spawn((
        SpriteBundle
        {
            texture: image_assets.gate0.clone(),
            sprite: Sprite::default(),
            transform: Transform::from_xyz(-10., 75., 0.),
            ..Default::default()
        },
        RigidBody::Static,
        Collider::cuboid(10.1, 75.),
        CollisionLayers::new([Layer::Ground], [Layer::Player]),
        GameState::InGame
    )).id();

    // Spikes 1
    commands.spawn((
        SpriteSheetBundle
        {
            visibility: bevy::render::view::Visibility::Hidden,
            transform: Transform::from_xyz(40., -30., 0.),
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
        Collider::cuboid(55., 15.),
        CollisionLayers::new([Layer::Enemy], [Layer::Player]),
        GameState::InGame,
        GroundObject { next_game_state: GameState::GameOver }
    ));

    // Interactivity
    commands.spawn((
        SpriteSheetBundle
        {
            visibility: bevy::render::view::Visibility::Hidden,
            transform: Transform::from_xyz(-35., 50., 0.),
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
            input_map: InputMap::new([ (KeyCode::B, PlayerAction::Interact) ]),
        },
        Interactivity
        {
            can_interact: true,
            is_interacting: false,
            question: QuestionData
            {
                text: "Геометрический смысл производной функции".into(),
                x: 850.,
                y: 50.,
            },
            entity: Some(wall.clone()),
            buttons: [
                QuizButtonData { x: 700., y: 150., text: "Тангенс угла касательной".into(), is_correct: false },
                QuizButtonData { x: 1000., y: 150., text: "Скорость изменения процесса".into(), is_correct: true },
                QuizButtonData { x: 700., y: 250., text: "Ускорение процесса".into(), is_correct: false },
                QuizButtonData { x: 1000., y: 250., text: "Дискриминант".into(), is_correct: false },
            ]
        }
    ));

    // Wall
    let wall = commands.spawn((
        SpriteBundle
        {
            texture: image_assets.gate1.clone(),
            sprite: Sprite::default(),
            transform: Transform::from_xyz(100., -25., 0.),
            ..Default::default()
        },
        RigidBody::Static,
        Collider::cuboid(50.1, 10.),
        CollisionLayers::new([Layer::Ground], [Layer::Player]),
        GameState::InGame
    )).id();

    // Uninteractable wall 2
    commands.spawn((
        SpriteSheetBundle
        {
            visibility: bevy::render::view::Visibility::Hidden,
            transform: Transform::from_xyz(130., 0., 0.),
            ..Default::default()
        },
        RigidBody::Static,
        Collider::cuboid(10.1, 500.),
        CollisionLayers::new([Layer::Ground], [Layer::Player]),
        GameState::InGame
    ));

    // Interactivity 2
    commands.spawn((
        SpriteSheetBundle
        {
            visibility: bevy::render::view::Visibility::Hidden,
            transform: Transform::from_xyz(100., -10., 0.),
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
            input_map: InputMap::new([ (KeyCode::B, PlayerAction::Interact) ]),
        },
        Interactivity
        {
            can_interact: true,
            is_interacting: false,
            question: QuestionData
            {
                text: "Временные рамки Великой российской революции".into(),
                x: 850.,
                y: 50.,
            },
            entity: Some(wall.clone()),
            buttons: [
                QuizButtonData { x: 700., y: 150., text: "1905–1907".into(), is_correct: false },
                QuizButtonData { x: 700., y: 250., text: "1941–1945".into(), is_correct: false },
                QuizButtonData { x: 1000., y: 150., text: "1917–1922".into(), is_correct: true },
                QuizButtonData { x: 1000., y: 250., text: "1812–1815".into(), is_correct: false },
            ]
        }
    ));

    // Spikes 2
    commands.spawn((
        SpriteSheetBundle
        {
            visibility: bevy::render::view::Visibility::Hidden,
            transform: Transform::from_xyz(100., -100., 0.),
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
        Collider::cuboid(45., 10.1),
        CollisionLayers::new([Layer::Enemy], [Layer::Player]),
        GameState::InGame,
        GroundObject { next_game_state: GameState::GameOver }
    ));

    // Spikes 3
    commands.spawn((
        SpriteSheetBundle
        {
            visibility: bevy::render::view::Visibility::Hidden,
            transform: Transform::from_xyz(105., -100., 0.),
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
        Collider::cuboid(40., 10.1),
        CollisionLayers::new([Layer::Enemy], [Layer::Player]),
        GameState::InGame,
        GroundObject { next_game_state: GameState::GameOver }
    ));

    // Level Goal
    commands.spawn((
        SpriteSheetBundle
        {
            visibility: bevy::render::view::Visibility::Hidden,
            transform: Transform::from_xyz(65., -100., 0.),
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
}

fn level_2(
    mut commands: Commands,
    image_assets: Res<GameAssets>
) {
    // Background
    commands.spawn((
        SpriteBundle
        {
            texture: image_assets.level2.clone(),
            transform: Transform::from_xyz(0., 0., -1.),
            ..Default::default()
        },
        GameState::InGame
    ));

    // Uninteractable wall 1
    commands.spawn((
        SpriteSheetBundle
        {
            visibility: bevy::render::view::Visibility::Hidden,
            transform: Transform::from_xyz(-150., 0., 0.),
            ..Default::default()
        },
        RigidBody::Static,
        Collider::cuboid(10.1, 500.),
        CollisionLayers::new([Layer::Ground], [Layer::Player]),
        GameState::InGame
    ));

    // Uninteractable wall 2
    commands.spawn((
        SpriteSheetBundle
        {
            visibility: bevy::render::view::Visibility::Hidden,
            transform: Transform::from_xyz(130., 0., 0.),
            ..Default::default()
        },
        RigidBody::Static,
        Collider::cuboid(10.1, 500.),
        CollisionLayers::new([Layer::Ground], [Layer::Player]),
        GameState::InGame
    ));

    let sets: [(f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32,
        String, String, String, String, String, bool, bool, bool, bool, Handle<Image>); 4] = 
    [
        (-100., 70., 90., 10.1, 50., 70., 150., 10.1, -40., 70., 25.1, 10., 0., 85., 10.1, 10.1, 
            "Как звали Раскольникова?".into(), "Родион".into(), "Ростислав".into(), "Ратибор".into(), "Руслан".into(), true, false, false, false, image_assets.gate2.clone()
        ),
        
        (-70., 15., 150., 10.1, 85., 15., 80., 10.1, 25., 15., 35.1, 10., 100., 30., 10.1, 10.1, 
            "Что не является хим. процессом?".into(), "Гидролиз".into(), "Преломление".into(), "Пиролиз".into(), "Галогенирование".into(), false, true, false, false, image_assets.gate3.clone()
        ),
        
        (-85., -40., 120., 10.1, 65., -40., 120., 10.1, -10., -40., 30., 10., -60., -25., 10.1, 10.1,
            "Какая столица не распологается в Азии?".into(), "Пекин".into(), "Скопье".into(), "Бангкок".into(), "Дакка".into(), false, true, false, false, image_assets.gate4.clone()
        ),
        
        (-50., -95., 180., 10.1, 100., -95., 50., 10.1, 57.5, -95., 35., 10., -45., -80., 10.1, 10.1,
            "Другое название низшей точки депрессии?".into(), "Минимум".into(), "Предел".into(), "Пик".into(), "Дно".into(), false, false, false, true, image_assets.gate5.clone()
        )
    ];

    for (g_x1, g_y1, g_cx1, g_cy1,
        g_x2, g_y2, g_cx2, g_cy2,
        w_x, w_y, w_cx, w_cy,
        i_x, i_y, i_cx, i_cy,
        question, a1, a2, a3, a4,
        c1, c2, c3, c4, gate
    ) in sets.iter()
    {
        // Ground #1
        commands.spawn((
            SpriteBundle
            {
                visibility: bevy::render::view::Visibility::Hidden,
                transform: Transform::from_xyz(*g_x1, *g_y1, 0.),
                ..Default::default()
            },
            RigidBody::Static,
            Collider::cuboid(*g_cx1, *g_cy1),
            GameState::InGame,
            CollisionLayers::new([Layer::Ground], [Layer::Player, Layer::Enemy])
        ));

        // Ground #2
        commands.spawn((
            SpriteBundle
            {
                visibility: bevy::render::view::Visibility::Hidden,
                transform: Transform::from_xyz(*g_x2, *g_y2, 0.),
                ..Default::default()
            },
            RigidBody::Static,
            Collider::cuboid(*g_cx2, *g_cy2),
            GameState::InGame,
            CollisionLayers::new([Layer::Ground], [Layer::Player, Layer::Enemy])
        ));

        // Wall 1
        let wall = commands.spawn((
            SpriteBundle
            {
                sprite: Sprite::default(),
                texture: gate.clone(),
                transform: Transform::from_xyz(*w_x, *w_y, 0.),
                ..Default::default()
            },
            RigidBody::Static,
            Collider::cuboid(*w_cx, *w_cy),
            CollisionLayers::new([Layer::Ground], [Layer::Player]),
            GameState::InGame
        )).id();

        // Interactivity 1
        commands.spawn((
            SpriteBundle
            {
                visibility: bevy::render::view::Visibility::Hidden,
                transform: Transform::from_xyz(*i_x, *i_y, 0.),
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
            Collider::cuboid(*i_cx, *i_cy),
            CollisionLayers::new([Layer::Interactable], [Layer::Player]),
            Sensor,
            GameState::InGame,

            InputManagerBundle::<PlayerAction>
            {
                action_state: ActionState::default(),
                input_map: InputMap::new([ (KeyCode::B, PlayerAction::Interact) ]),
            },
            Interactivity
            {
                can_interact: true,
                is_interacting: false,
                question: QuestionData
                {
                    text: question.into(),
                    x: 850.,
                    y: 50.,
                },
                entity: Some(wall.clone()),
                buttons: [
                    QuizButtonData { x: 700., y: 150., text: a1.into(), is_correct: c1.clone() },
                    QuizButtonData { x: 1000., y: 150., text: a2.into(), is_correct: c2.clone() },
                    QuizButtonData { x: 700., y: 250., text: a3.into(), is_correct: c3.clone() },
                    QuizButtonData { x: 1000., y: 250., text: a4.into(), is_correct: c4.clone() },
                ]
            }
        ));
    }

    // Level Goal
    commands.spawn((
        SpriteBundle
        {
            visibility: bevy::render::view::Visibility::Hidden,
            transform: Transform::from_xyz(60., -105., 0.),
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
}
