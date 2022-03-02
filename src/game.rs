use crate::{
    ai::negamax,
    character::{Character, CharacterPlugin, DialogueText, Say},
    choss::{draw_choss, piece_tex_name, ChossGame, SIZE},
    piece::{Action, Color as PieceColor, Piece},
    pos::Pos,
    utils::screen_to_world,
};
use bevy::{prelude::*, tasks::AsyncComputeTaskPool};
use bevy::{render::color::Color, tasks::Task};
use futures_lite::future;
use rand::Rng;
use std::collections::HashMap;

#[derive(Component)]
struct MovingTo(Transform);

#[derive(Component)]
struct Die;

#[derive(Component)]
struct PromoteTo(Piece, PieceColor);

struct Game {
    opponent: Option<Entity>,
    to_play: Option<(Pos, Vec<Action>)>,
}

impl Game {
    fn new() -> Self {
        Game {
            opponent: None,
            to_play: None,
        }
    }
}

fn create_opponents(mut commands: Commands, server: Res<AssetServer>, mut game: ResMut<Game>) {
    let entity = commands
        .spawn()
        .insert(Character {
            name: "Carl Blok".to_string(),
            faces: HashMap::new(),
            voice: server.load("sounds/uh_carl.ogg"),
        })
        .id();
    game.opponent = Some(entity);
}

fn play_move(
    mut commands: Commands,
    mut choss: ResMut<ChossGame>,
    mut piece_ents: ResMut<HashMap<Pos, Entity>>,
    mut game: ResMut<Game>,
    mut query_text: Query<&mut Text, With<DialogueText>>,
    server: Res<AssetServer>,
    audio: Res<Audio>,
) {
    if let Some((pos, actions)) = &game.to_play {
        let color = choss.turn_color();
        choss.play(*pos, actions);
        let ent = *piece_ents.get(&pos).unwrap();
        let mut is_take = false;
        for action in actions {
            match action {
                Action::Go(new_pos) => {
                    commands
                        .entity(ent)
                        .insert(MovingTo(choss.board_to_world(*new_pos)));
                    // if anything was on this new square, it should die
                    if let Some(o_ent) = piece_ents.get(&new_pos) {
                        is_take = true;
                        commands.entity(*o_ent).insert(Die);
                    }
                    piece_ents.remove_entry(&pos);
                    piece_ents.insert(*new_pos, ent);
                }
                Action::Take(new_pos) => {
                    let o_ent = *piece_ents.get(&new_pos).unwrap();
                    commands.entity(o_ent).insert(Die);
                    piece_ents.remove_entry(&new_pos);
                    is_take = true;
                }
                Action::Promotion(new_piece) => {
                    commands.entity(ent).insert(PromoteTo(*new_piece, color));
                }
            }
        }
        if color == choss.player {
            if let Ok(mut text) = query_text.get_single_mut() {
                text.sections[0].value = "".to_string();
            }
        }
        if choss.board.is_checked(color.next()) {
            audio.play(server.load("sounds/check.ogg"));
        } else if is_take {
            audio.play(server.load("sounds/take.ogg"));
            if color == choss.player {
                commands
                    .entity(game.opponent.unwrap())
                    .insert(Say::new("".to_string(), "Your mom's a hoe.".to_string()));
            }
        } else {
            audio.play(server.load("sounds/move.ogg"));
        }
        game.to_play = None;
    }
}

fn mouse_button_input(
    q_camera: Query<(&Camera, &GlobalTransform)>,
    buttons: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    mut selected: ResMut<SelectedSquare>,
    mut game: ResMut<Game>,
    choss: ResMut<ChossGame>,
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
                        game.to_play = Some((old_pos, actions));
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
                        custom_size: Some(Vec2::new(SIZE as f32 / 2.5, SIZE as f32 / 2.5)),
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

fn promote(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Handle<Image>, &PromoteTo)>,
    server: Res<AssetServer>,
) {
    for (entity, mut image, promote) in query.iter_mut() {
        commands.entity(entity).remove::<PromoteTo>();
        *image = server.load(
            format!(
                "choss_pieces/{}.png",
                piece_tex_name(&promote.0, &promote.1)
            )
            .as_str(),
        );
    }
}

#[derive(Component)]
struct WaitUntil(f64);

fn start_ai_turn(
    mut commands: Commands,
    choss: Res<ChossGame>,
    thread_pool: Res<AsyncComputeTaskPool>,
    time: Res<Time>,
) {
    if choss.is_changed() {
        // a play has been made, check if it's the AI's turn
        if choss.player != choss.turn_color() {
            // play the AI move
            // Spawn new task on the AsyncComputeTaskPool
            let board = choss.board.clone();
            let color = choss.turn_color();
            let value = choss.remaining_value();
            let depth = if value < 5. {
                5
            } else if value < 10. {
                3
            } else {
                1
            };
            println!("thinking with base depth {}", depth);
            let task = thread_pool.spawn(async move { negamax(&board, color, depth) });
            // Spawn new entity and add our new task as a component
            commands
                .spawn()
                .insert(task)
                .insert(WaitUntil(time.seconds_since_startup() + 1.));
        }
    }
}

fn end_ai_turn(
    mut commands: Commands,
    mut ai_task: Query<(Entity, &mut Task<Vec<(f32, Pos, Vec<Action>)>>, &WaitUntil)>,
    query_say: Query<(), With<Say>>,
    mut game: ResMut<Game>,
    time: Res<Time>,
) {
    if let Ok((entity, mut task, wait_until)) = ai_task.get_single_mut() {
        if time.seconds_since_startup() >= wait_until.0 {
            // if no one's talking
            if query_say.is_empty() {
                if let Some(moves) = future::block_on(future::poll_once(&mut *task)) {
                    // Task is complete, so remove task component from entity
                    commands
                        .entity(entity)
                        .remove::<Task<Vec<(f32, Pos, Vec<Action>)>>>();
                    // if a second move with a similar evaluation is available, pick randomly between move 1 and 2
                    let move_id = if moves.len() > 1 && (moves[0].0 - moves[1].0) < 2. {
                        rand::thread_rng().gen_range(0..2)
                    } else {
                        0
                    };
                    let (_, pos, actions) = moves[move_id].to_owned();
                    game.to_play = Some((pos, actions));
                }
            }
        }
    }
}

pub struct SelectedSquare(Option<Pos>);

pub struct HoveredSquare(Option<Pos>);

pub struct Undoing;

impl Plugin for Undoing {
    fn build(&self, app: &mut App) {
        app.add_plugin(CharacterPlugin)
            .insert_resource(Game::new())
            .insert_resource(HashMap::<Pos, Entity>::new())
            .insert_resource(ChossGame::new(PieceColor::White))
            .insert_resource(SelectedSquare(None))
            .insert_resource(HoveredSquare(None))
            .add_startup_system(create_opponents)
            .add_startup_system(draw_choss)
            .add_system(mouse_button_input)
            .add_system(play_move)
            .add_system(display_moves)
            .add_system(move_to)
            .add_system(die)
            .add_system(promote)
            .add_system(start_ai_turn)
            .add_system(end_ai_turn);
    }
}
