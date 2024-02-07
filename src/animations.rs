
use bevy::ecs::component::Component;
use bevy::ecs::system::Query;
use bevy::ecs::system::Res;
use bevy::sprite::TextureAtlasSprite;
use bevy::time::Time;
use bevy::time::Timer;

#[derive(Component, Debug)]
pub struct AnimatableLayer
{
    pub timer: Timer,
    pub animations: Vec<(usize, usize)>,
    pub current_animation: usize
}

pub fn update_animation(mut sprites: Query<(&mut TextureAtlasSprite, &mut AnimatableLayer)>, time: Res<Time>)
{
    for (mut sprites, mut animatable) in &mut sprites
    {
        animatable.timer.tick(time.delta());

        if animatable.timer.just_finished()
        {
            sprites.index += 1;

            let indices = animatable.animations[animatable.current_animation];
            if sprites.index >= indices.1
            {
                sprites.index = indices.0;
            }
        }
    }
}
