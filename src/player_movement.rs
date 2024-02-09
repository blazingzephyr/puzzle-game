
use bevy::ecs::component::Component;
use bevy::ecs::system::Query;
use bevy::reflect::Reflect;
use bevy_xpbd_2d::components::LinearVelocity;
use bevy_xpbd_2d::plugins::spatial_query::ShapeHits;
use leafwing_input_manager::action_state::ActionState;
use leafwing_input_manager::Actionlike;

use crate::animations::AnimatableLayer;

#[derive(Component, Debug)]
pub struct Player
{
    pub is_interacting: bool
}

#[derive(Actionlike, PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect)]
pub enum PlayerAction
{
    Left,
    Right,
    Jump,
}

pub fn update_player_movement(
    mut query: Query<(
        &ActionState<PlayerAction>,
        &mut LinearVelocity,
        &ShapeHits,
        &Player,
        Option<&mut AnimatableLayer>
    )>
) {
    for (action_state, mut linear_velocity, ground_hits, player, mut animatable) in query.iter_mut()
    {
        if !player.is_interacting
        {
            let mut gonna_jump = false;
            if action_state.just_pressed(PlayerAction::Jump) && !ground_hits.is_empty()
            {
                linear_velocity.y += 60.0;
                gonna_jump = true;

                if let Some(ref mut anim) = animatable
                {
                    anim.next_animation = 4;
                }
            }

            let left = action_state.pressed(PlayerAction::Left);
            let right = action_state.pressed(PlayerAction::Right);
            
            if left || right
            {
                linear_velocity.x += 1.2 * left.then_some(-1.).unwrap_or(1.);

                if let Some(ref mut anim) = animatable
                {
                    anim.flip_x = left;
                }
            }

            if !gonna_jump && !ground_hits.is_empty()
            {
                if let Some(mut anim) = animatable
                {
                    anim.next_animation = if linear_velocity.x < 0.5 && linear_velocity.y.abs() < 0.5 { 0 } else { 3 };
                }
            }
        }
    }
}
