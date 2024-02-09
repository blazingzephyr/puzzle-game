
mod system;
mod animations;
mod player_movement;

use std::string;

use animations::{update_animation, AnimatableLayer};

use bevy::app::{AppExit, PluginGroup, StateTransition, Update};
use bevy::asset::{AssetId, AssetServer, Assets, RecursiveDependencyLoadState};
use bevy::core_pipeline::core_2d::Camera2dBundle;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::event::{EventReader, EventWriter};
use bevy::ecs::query::{Changed, With, Without};
use bevy::ecs::schedule::common_conditions::run_once;
use bevy::ecs::schedule::{Condition, NextState, OnEnter, OnExit, OnTransition, State, SystemSet};
use bevy::ecs::system::{Commands, Query, ResMut, RunSystemOnce};
use bevy::ecs::world::World;
use bevy::hierarchy::DespawnRecursiveExt;
use bevy::input::keyboard::KeyCode;
use bevy::input::mouse::{MouseButton, MouseButtonInput};
use bevy::input::Input;
use bevy::math::{Vec2, Vec2Swizzles};
use bevy::pbr::PointLightBundle;
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
use bevy::utils::HashMap;
use bevy::window::{PrimaryWindow, Window, WindowPlugin, WindowResolution};
use bevy::{app::App, asset::Handle, ecs::{schedule::{common_conditions::in_state, IntoSystemConfigs, States}, system::{Res, Resource}}, sprite::TextureAtlas, text::Font, DefaultPlugins};
use bevy_asset_loader::{asset_collection::AssetCollection, loading_state::{config::ConfigureLoadingState, LoadingState, LoadingStateAppExt}, standard_dynamic_asset::StandardDynamicAssetCollection};
use bevy_pixel_camera::{PixelCameraPlugin, PixelViewport, PixelZoom};
use bevy_xpbd_2d::components::{CoefficientCombine, Collider, LinearVelocity, LockedAxes, Restitution, RigidBody};
use bevy_xpbd_2d::math::Vector;
use bevy_xpbd_2d::plugins::debug::PhysicsDebugConfig;
use bevy_xpbd_2d::plugins::spatial_query::{ShapeCaster, ShapeHits};
use bevy_xpbd_2d::plugins::{PhysicsDebugPlugin, PhysicsPlugins};
use bevy_xpbd_2d::resources::Gravity;
use leafwing_input_manager::action_state::ActionState;
use leafwing_input_manager::input_map::InputMap;
use leafwing_input_manager::plugin::InputManagerPlugin;
use leafwing_input_manager::{Actionlike, InputManagerBundle};
use player_movement::{update_player_movement, Player, PlayerAction};
use system::{cleanup_after_state, GameState};

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
    #[asset(key = "menu_background")]
    menu_background: Handle<TextureAtlas>,

    #[asset(key = "main_font")]
    main_font: Handle<Font>,

    #[asset(key = "sonic")]
    sonic: Handle<TextureAtlas>,
}

#[derive(Component)]
enum MainMenuButtonAction
{
    Play,
    Quit,
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
                .continue_to_state(GameState::Menu)
                .with_dynamic_assets_file::<StandardDynamicAssetCollection>("game_assets.assets.ron")
                .load_collection::<GameAssets>()
        )
        .add_systems(StateTransition, cleanup_after_state)
        .add_systems(OnExit(GameState::PreLoading), (camera_setup, preload))
        .add_systems(OnEnter(GameState::Menu), setup_menu)
        .add_systems(Update, update_animation)
        .add_systems(Update, click_on_button.run_if(in_state(GameState::Menu)))
        .add_systems(OnEnter(GameState::InGame), spawn_player)
        .add_systems(Update, update_player_movement.run_if(in_state(GameState::InGame)))
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
            flip_x: false
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
            current_animation: 0,
            next_animation: 0,
            flip_x: false
        },
        GameState::Menu
    ));

    commands.spawn((
        TextBundle::from_section(
            "Начать!",
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
        GameState::Menu
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
        MainMenuButtonAction::Play,
        GameState::Menu
    ));
}

fn click_on_button(
    interaction_query: Query<
        (&Interaction, &MainMenuButtonAction),
        (Changed<Interaction>, With<Button>),
    >,
    mut game_state: ResMut<NextState<GameState>>,
    mut ev_app_exit: EventWriter<AppExit>,
) {
    for (interaction, menu_button_action) in &interaction_query
    {
        if *interaction == Interaction::Pressed
        {
            match menu_button_action
            {
                MainMenuButtonAction::Quit => ev_app_exit.send(AppExit),
                MainMenuButtonAction::Play => game_state.set(GameState::InGame)
            }
        }
    }
}

fn spawn_player(mut commands: Commands, image_assets: Res<GameAssets>)
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
            texture_atlas: image_assets.sonic.clone(),
            sprite: sprite.clone(),
            ..Default::default()
        },
        AnimatableLayer
        {
            timer: Timer::from_seconds(0.125, TimerMode::Repeating),
            animations: vec![(0, 29), (30, 34), (35, 39), (40, 51), (52, 62)],
            current_animation: 0,
            next_animation: 0,
            flip_x: false
        },
        InputManagerBundle::<PlayerAction>
        {
            action_state: ActionState::default(),
            input_map: InputMap::new(
                [
                    (KeyCode::Space, PlayerAction::Jump),
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
            Collider::cuboid(13.95, 34.15),
            Vector::NEG_Y * 0.05,
            0.,
            Vector::NEG_Y,
        )
        .with_max_time_of_impact(0.2)
        .with_max_hits(1),
        
        // This controls how bouncy a rigid body is.
        Restitution::new(0.0).with_combine_rule(CoefficientCombine::Min),
        Player { is_interacting: false }
    ));

    commands.spawn((
        SpriteSheetBundle
        {
            texture_atlas: image_assets.sonic.clone(),
            sprite,
            transform: Transform::from_xyz(0., -25., 0.),
            ..Default::default()
        },
        AnimatableLayer
        {
            timer: Timer::from_seconds(0.125, TimerMode::Repeating),
            animations: vec![(0, 10)],
            current_animation: 0,
            next_animation: 0,
            flip_x: false
        },
        RigidBody::Static,
        Collider::cuboid(100., 10.1)
    ));

}