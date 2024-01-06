
use bevy::{prelude::*, sprite::Anchor, window::WindowResolution, input::keyboard::{KeyboardInput, self}};
use bevy_asset_loader::prelude::*;
use bevy_pixel_camera::{PixelCameraPlugin, PixelZoom, PixelViewport};

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
enum GameState
{
    #[default]
    Loading,
    InGame
}

#[derive(AssetCollection, Resource)]
struct ImageAssets
{
    #[asset(texture_atlas(tile_size_x = 49., tile_size_y = 56., columns = 10, rows = 7))]
    #[asset(path = "sonic.png")]
    sonic: Handle<TextureAtlas>
}

////

#[derive(Component, Debug)]
struct Player;

#[derive(Component, Debug)]
struct MaxVelocity(f32);

#[derive(Component, Debug)]
struct Velocity(Vec3);

////

#[derive(Component, Debug)]
struct AnimationTimer
{
    timer: Timer,
    frame_count: usize
}

fn setup(mut commands: Commands)
{
    commands.spawn((
        Camera2dBundle::default(),
        PixelZoom::FitSize {
            width: 320,
            height: 224,
        },
        PixelViewport
    ));
}

fn spawn_player(mut commands: Commands, image_assets: Res<ImageAssets>)
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
            sprite,
            ..Default::default()
        },
        AnimationTimer
        {
            timer: Timer::from_seconds(0.125, TimerMode::Repeating),
            frame_count: 69
        },
        Velocity(Vec3::ZERO),
        MaxVelocity(100.0),
        Player
    ));
}

fn update_animation(mut sprites: Query<(&mut TextureAtlasSprite, &mut AnimationTimer)>, time: Res<Time>)
{
    for (mut sprites, mut animation) in &mut sprites
    {
        animation.timer.tick(time.delta());

        if animation.timer.just_finished()
        {
            sprites.index += 1;

            if sprites.index >= animation.frame_count
            {
                sprites.index = 0;
            }
        }
    }
}

fn update_position(
    mut query: Query<(&mut Transform, &Velocity)>,
    time: Res<Time>
)
{
    for (mut transform, velocity) in &mut query
    {
        transform.translation += velocity.0 * time.delta_seconds();
        info!("{:?}", transform);
    }
}

fn query_input(
    mut query: Query<(&mut Transform, &mut Velocity, &mut MaxVelocity), With<Player>>,
    input: Res<Input<KeyCode>>
)
{
    let (mut transform, mut velocity, max_vel) = query.single_mut();
    let mut movement = 0.0;

    if input.any_pressed([KeyCode::D, KeyCode::Right])
    {
        movement = max_vel.0;
        transform.scale = Vec3::new(1., 1., 1.);
    }
    else if input.any_pressed([KeyCode::A, KeyCode::Left])
    {
        movement = -max_vel.0;
        transform.scale = Vec3::new(-1., 1., 1.);
    }

    velocity.0 = transform.local_x() * movement;
}

fn main()
{
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: String::from("Puzzle Game"),
                        resolution: WindowResolution::new(960., 672.),
                        ..default()
                    }),
                    ..default()
                })
        )
        .add_plugins(PixelCameraPlugin)
        .add_state::<GameState>()
        .add_loading_state(
            LoadingState::new(GameState::Loading)
                .continue_to_state(GameState::InGame)
                .load_collection::<ImageAssets>()
        )
        .add_systems(Startup, setup)
        .add_systems(OnEnter(GameState::InGame), spawn_player)
        .add_systems(
            Update,
            (query_input, update_position, update_animation).run_if(in_state(GameState::InGame))
        )
        .run();
}