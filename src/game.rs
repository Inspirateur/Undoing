use crate::{
    ai::negamax,
    choss::{draw_choss, ChossGame, SIZE},
    piece::{Action, Color as PieceColor, Piece},
    pos::Pos,
    utils::screen_to_world,
};
use bevy::{prelude::*, tasks::AsyncComputeTaskPool};
use bevy::{render::color::Color, tasks::Task};
use futures_lite::future;
use std::collections::HashMap;

#[derive(Component)]
struct MovingTo(Transform);

#[derive(Component)]
struct Die;

#[derive(Component)]
struct PromoteTo(Piece);

fn play_move(
    mut commands: Commands,
    mut choss: ResMut<ChossGame>,
    mut piece_ents: ResMut<HashMap<Pos, Entity>>,
    pos: Pos,
    actions: Vec<Action>,
) {
    choss.play(pos, &actions);
    let ent = *piece_ents.get(&pos).unwrap();
    for action in actions {
        match action {
            Action::Go(new_pos) => {
                commands
                    .entity(ent)
                    .insert(MovingTo(choss.board_to_world(new_pos)));
                // if anything was on this new square, it should die
                if let Some(o_ent) = piece_ents.get(&new_pos) {
                    commands.entity(*o_ent).insert(Die);
                }
                piece_ents.remove_entry(&pos);
                piece_ents.insert(new_pos, ent);
            }
            Action::Take(new_pos) => {
                let o_ent = *piece_ents.get(&new_pos).unwrap();
                commands.entity(o_ent).insert(Die);
                piece_ents.remove_entry(&new_pos);
            }
            Action::Promotion(new_piece) => {
                commands.entity(ent).insert(PromoteTo(new_piece));
            }
        }
    }
}

fn mouse_button_input(
    commands: Commands,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    buttons: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    mut selected: ResMut<SelectedSquare>,
    choss: ResMut<ChossGame>,
    piece_ents: ResMut<HashMap<Pos, Entity>>,
) {
    if buttons.just_released(MouseButton::Left) {
        let window = windows.get_primary().unwrap();
        if let Some(screen_pos) = window.cursor_position() {
            let (camera, camera_transform) = q_camera.single();
            let world_pos: Vec2 = screen_to_world(window, camera, camera_transform, screen_pos);
            let pos = choss.world_to_board(world_pos);
            if choss.board.in_bound(pos) {
                if let Some(old_pos) = selected.0 {
                    // if the old and new pos correspond to a playable action, play it
                    if let Some(actions) = choss.playable_move(old_pos, pos) {
                        play_move(commands, choss, piece_ents, old_pos, actions);
                        selected.0 = None;
                    } else {
                        selected.0 = Some(pos);
                    }
                } else {
                    selected.0 = Some(pos);
                }
            } else {
                selected.0 = None;
            }
        }
    }
}

#[derive(Component)]
struct MoveDisplay;

fn display_moves(
    query: Query<Entity, With<MoveDisplay>>,
    mut commands: Commands,
    selected: Res<SelectedSquare>,
    choss: Res<ChossGame>,
    server: Res<AssetServer>,
) {
    if selected.is_changed() {
        // despawn all previously shown MoveDisplays
        for move_display in query.iter() {
            commands.entity(move_display).despawn();
        }
        // check if the new selected pos corresponds to a player piece
        if let Some(pos) = selected.0 {
            if let Some(moves) = choss.playable_moves(pos) {
                // spawn a move display for each move of this piece
                let sprite = SpriteBundle {
                    sprite: Sprite {
                        color: Color::rgba(0., 0., 0., 0.5),
                        ..Default::default()
                    },
                    texture: server.load("circle.png"),
                    ..Default::default()
                };
                for actions in moves {
                    for action in actions {
                        if let Action::Go(go_pos) = action {
                            let mut sprite_clone = sprite.clone();
                            sprite_clone.transform = choss.board_to_world(go_pos);
                            commands.spawn_bundle(sprite_clone).insert(MoveDisplay);
                        }
                    }
                }
            }
        }
    }
}

fn move_to(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &MovingTo)>,
    time: Res<Time>,
) {
    for (ent, mut transform, moving_to) in query.iter_mut() {
        let mut diff = moving_to.0.translation - transform.translation;
        let mut length = time.delta_seconds() * SIZE as f32 * 20.;
        // the piece finished moving
        if length >= diff.length() {
            length = diff.length();
            commands.entity(ent).remove::<MovingTo>();
        }
        if length > 0. {
            diff = length * diff / diff.length();
            transform.translation = transform.translation + diff;
        }
    }
}

fn die(mut commands: Commands, mut query: Query<Entity, With<Die>>) {
    for ent in query.iter_mut() {
        // for now we just despawn the entity, might do fancy things later
        commands.entity(ent).despawn();
    }
}

fn start_ai_turn(
    mut commands: Commands,
    choss: Res<ChossGame>,
    thread_pool: Res<AsyncComputeTaskPool>,
) {
    if choss.is_changed() {
        // a play has been made, check if it's the AI's turn
        if choss.player != choss.turn_color() {
            // play the AI move
            // Spawn new task on the AsyncComputeTaskPool
            let board = choss.board.clone();
            let color = choss.turn_color();
            let depth = if choss.remaining_value() < 10. { 3 } else { 1 };
            println!("thinking with base depth {}", depth);
            let task = thread_pool.spawn(async move { negamax(&board, color, depth) });
            // Spawn new entity and add our new task as a component
            commands.spawn().insert(task);
        }
    }
}

fn end_ai_turn(
    mut commands: Commands,
    mut ai_task: Query<(Entity, &mut Task<Option<(Pos, Vec<Action>)>>)>,
    choss: ResMut<ChossGame>,
    piece_ents: ResMut<HashMap<Pos, Entity>>,
) {
    if let Ok((entity, mut task)) = ai_task.get_single_mut() {
        if let Some(Some((pos, actions))) = future::block_on(future::poll_once(&mut *task)) {
            // Task is complete, so remove task component from entity
            commands
                .entity(entity)
                .remove::<Task<Option<(Pos, Vec<Action>)>>>();
            play_move(commands, choss, piece_ents, pos, actions);
        }
    }
}

pub struct SelectedSquare(Option<Pos>);

pub struct HoveredSquare(Option<Pos>);

pub struct Undoing;

impl Plugin for Undoing {
    fn build(&self, app: &mut App) {
        app.insert_resource(HashMap::<Pos, Entity>::new())
            .insert_resource(ChossGame::new(PieceColor::White))
            .insert_resource(SelectedSquare(None))
            .insert_resource(HoveredSquare(None))
            .add_startup_system(draw_choss)
            .add_system(mouse_button_input)
            .add_system(display_moves)
            .add_system(move_to)
            .add_system(die)
            .add_system(start_ai_turn)
            .add_system(end_ai_turn);
    }
}
