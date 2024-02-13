
use std::ops::Deref;
use bevy::app::AppExit;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::event::EventWriter;
use bevy::ecs::query::Changed;
use bevy::ecs::query::Or;
use bevy::ecs::query::With;
use bevy::ecs::schedule::NextState;
use bevy::ecs::system::Commands;
use bevy::ecs::system::Query;
use bevy::ecs::system::Res;
use bevy::ecs::system::ResMut;
use bevy::hierarchy::BuildChildren;
use bevy::hierarchy::DespawnRecursiveExt;
use bevy::prelude::Deref;
use bevy::prelude::DerefMut;
use bevy::render::color::Color;
use bevy::text::TextAlignment;
use bevy::text::TextStyle;
use bevy::ui::node_bundles::ButtonBundle;
use bevy::ui::node_bundles::TextBundle;
use bevy::ui::widget::Button;
use bevy::ui::Interaction;
use bevy::ui::JustifySelf;
use bevy::ui::PositionType;
use bevy::ui::Style;
use bevy::ui::Val;
use bevy_xpbd_2d::components::CollidingEntities;
use leafwing_input_manager::action_state::ActionState;

use crate::assets::GameAssets;
use crate::player::Immobile;
use crate::player::PlayerAction;
use crate::system::CurrentLevel;
use crate::system::GameState;
use crate::system::QuizClear;

#[derive(Component)]
pub enum MenuButtonAction
{
    Play,
    Quit,
    BackToMenu
}

#[derive(Clone, Component, Debug, Default, Deref, DerefMut, PartialEq, Eq)]
pub struct GroundObject
{
    pub next_game_state: GameState
}


#[derive(Clone, Component, Debug, Default, PartialEq)]
pub struct Interactivity
{
    pub can_interact: bool,
    pub is_interacting: bool,
    pub question: QuestionData,
    pub buttons: [QuizButtonData; 4],
    pub entity: Option<Entity>
}

#[derive(Clone, Component, Debug, Default, PartialEq)]
pub struct QuestionData
{
    pub x: f32,
    pub y: f32,
    pub text: String
}

#[derive(Clone, Component, Debug, Default, PartialEq)]
pub struct QuizButtonData
{
    pub x: f32,
    pub y: f32,
    pub is_correct: bool,
    pub text: String
}

#[derive(Clone, Component, Debug, Default, PartialEq)]
pub struct QuizLabel;

#[derive(Clone, Component, Debug, PartialEq)]
pub struct QuizButton
{
    pub is_correct: bool,
    pub entity: Option<Entity>,
    pub interactivity: Interactivity,
    pub player: Entity
}

pub fn interact_with_menu_button(
    interaction_query: Query<
        (&Interaction, &MenuButtonAction),
        (Changed<Interaction>, With<Button>),
    >,
    mut game_state: ResMut<NextState<GameState>>,
    mut ev_app_exit: EventWriter<AppExit>
) {
    for (interaction, menu_button_action) in &interaction_query
    {
        if *interaction == Interaction::Pressed
        {
            match menu_button_action
            {
                MenuButtonAction::Quit => ev_app_exit.send(AppExit),
                MenuButtonAction::Play => game_state.set(GameState::LevelCompleted),
                MenuButtonAction::BackToMenu => game_state.set(GameState::MainMenu)
            }
        }
    }
}

pub fn interact_with_gobject(
    mut game_stats: ResMut<CurrentLevel>,
    query: Query<(&GroundObject, &CollidingEntities)>,
    mut game_state: ResMut<NextState<GameState>>
) {
    for (object, colliding_entities) in &query
    {
        if colliding_entities.0.len() > 0
        {
            if object.next_game_state != GameState::GameOver
            {
                game_stats.0 += 1;
            }
            
            game_state.set(object.next_game_state);
        }
    }
}

pub fn update_player_interaction(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    mut query: Query<(
        &ActionState<PlayerAction>,
        &mut Interactivity,
        &CollidingEntities
    )>
) {
    for (action_state, mut interactivity, colliding_entities) in query.iter_mut()
    {
        if interactivity.can_interact && colliding_entities.0.len() > 0 && action_state.just_pressed(PlayerAction::Interact)
        {
            let player_entity = colliding_entities.0.iter().next().unwrap();
            let mut player_commands = commands.entity(*player_entity);
            
            if interactivity.is_interacting
            {
                interactivity.is_interacting = false;
                player_commands.remove::<Immobile>();
                commands.insert_resource(QuizClear(true));
            }
            else
            {                
                interactivity.is_interacting = true; 
                player_commands.insert(Immobile {});

                let text_style = TextStyle {
                    font: game_assets.main_font.clone(),
                    font_size: 25.0,
                    color: Color::BLACK
                };

                let label_style = Style {
                    justify_self: JustifySelf::Center,
                    align_self: bevy::ui::AlignSelf::Center,
                    ..Default::default()
                };

                let color = Color::rgb(0.85, 0.61, 0.38);

                commands.spawn(
                    (
                        ButtonBundle {
                            style: Style
                            {
                                left: Val::Px(interactivity.question.x),
                                top: Val::Px(interactivity.question.y),
                                width: Val::Px(250.0),
                                height: Val::Px(65.0),
                                position_type: PositionType::Absolute,
                                ..Default::default()
                            },
                            background_color: color.clone().into(),
                            ..Default::default()
                        },
                        QuizLabel,
                        GameState::InGame
                    ))
                .with_children(
                    |parent|
                    {
                        parent.spawn((
                            TextBundle::from_section(interactivity.question.text.deref(), text_style.clone())
                                .with_text_alignment(TextAlignment::Center)
                                .with_style(label_style.clone()),
                            GameState::InGame,
                        ));
                    }
                );

                for quiz_button in interactivity.buttons.iter()
                {
                    commands.spawn((
                        ButtonBundle {
                            style: Style
                            {
                                left: Val::Px(quiz_button.x),
                                top: Val::Px(quiz_button.y),
                                width: Val::Px(250.0),
                                height: Val::Px(65.0),
                                position_type: PositionType::Absolute,
                                ..Default::default()
                            },
                            background_color: color.clone().into(),
                            ..Default::default()
                        },
                        QuizButton {
                            is_correct: quiz_button.is_correct,
                            entity: interactivity.entity,
                            interactivity: interactivity.deref().to_owned(),
                            player: *player_entity
                        },
                        GameState::InGame
                    ))
                    .with_children(
                        |parent|
                        {
                            parent.spawn((
                                TextBundle::from_section(quiz_button.text.deref(), text_style.clone())
                                    .with_text_alignment(TextAlignment::Center)
                                    .with_style(label_style.clone()),
                                GameState::InGame
                            ));
                        }
                    );
                }
            }
        }
    }
}

pub fn interact_with_quiz_button(
    mut commands: Commands,
    mut interaction_query: Query<
        (&Interaction, &QuizButton),
        (Changed<Interaction>, With<Button>),
    >,
    mut game_state: ResMut<NextState<GameState>>
) {
    for (interaction, quiz_button) in interaction_query.iter_mut()
    {
        if *interaction == Interaction::Pressed
        {
            if quiz_button.is_correct
            {
                if let Some(removed) = quiz_button.entity
                {
                    commands.entity(removed).despawn_recursive();
                }
                
                commands.entity(quiz_button.player).remove::<Immobile>();
                commands.insert_resource(QuizClear(false));
            }
            else
            {
                game_state.set(GameState::GameOver);
                commands.insert_resource(QuizClear(false));
            }
        }
    }
}

pub fn clear_quiz_buttons(
    mut commands: Commands,
    query: Query<Entity, Or<(With<QuizButton>, With<QuizLabel>)>>
) {
    for quiz_button in query.iter()
    {
        commands.entity(quiz_button).despawn_recursive();
    }

    commands.remove_resource::<QuizClear>();
}

pub fn make_uninteractable(
    game_stats: Res<QuizClear>,
    mut query: Query<&mut Interactivity>
) {
    for mut interactivity in query.iter_mut()
    {
        interactivity.can_interact = game_stats.0;
        interactivity.is_interacting = false;
        break;
    }
}
