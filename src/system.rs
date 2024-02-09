
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::schedule::State;
use bevy::ecs::schedule::States;
use bevy::ecs::system::Commands;
use bevy::ecs::system::Query;
use bevy::ecs::system::Res;
use bevy::hierarchy::DespawnRecursiveExt;

#[derive(Debug, Component, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum GameState
{
    #[default]
    PreLoading,
    AssetLoading,
    MainMenu,
    InGame,
    GameOver,
    LevelCompleted,
    FullCompletion
}

pub fn cleanup_after_state(
    mut commands: Commands,
    game_state: Res<State<GameState>>,
    entitities: Query<(Entity, &GameState)>
) {
    let state = game_state.get();
    for (entity, entity_state) in entitities.iter()
    {
        if entity_state != state
        {
            commands.entity(entity).despawn_recursive();
        }
    }
}