use crate::{
    ai::negamax,
    character::{Character, CharacterPlugin, DialogueFace, DialogueText, Say},
    choss::{draw_choss, piece_tex_name, ChossGame, SIZE},
    piece::{Action, Color as PieceColor, Piece},
    pos::Pos,
    utils::screen_to_world,
};
use bevy::prelude::*;
use bevy::{render::color::Color, tasks::Task};
use rand::seq::SliceRandom;
use std::collections::{HashMap, HashSet};

#[derive(Component)]
struct MovingTo(Transform);

#[derive(Component)]
struct Die;

#[derive(Component)]
struct PromoteTo(Piece, PieceColor);

#[derive(PartialEq, Eq)]
enum GameStatus {
    Playing,
    Draw,
    Win,
    Loss,
    Preparing,
    Placing,
    Ending,
}

impl Default for GameStatus {
    fn default() -> Self {
        GameStatus::Preparing
    }
}

#[derive(Default)]
struct Game {
    opponents: Vec<Entity>,
    opponent: usize,
    to_play: Option<(Pos, Vec<Action>)>,
    last_eval: Option<f32>,
    lines_sent: HashSet<String>,
    status: GameStatus,
    cached_moves: Vec<(f32, Pos, Vec<Action>)>,
    turn: u32,
    last_state: Option<ChossGame>,
    carl_lines: Vec<String>,
    last_move_time: f64,
}

impl Game {
    fn new() -> Self {
        let mut carl_lines = vec![
            "Oh... that won't do.".to_string(),
            "Mh, that doesn't work.".to_string(),
            "Nope, this is not good.".to_string(),
            "Ugh, I need another move !".to_string(),
            "Again ...".to_string(),
        ];
        carl_lines.reverse();
        Game {
            carl_lines,
            ..Default::default()
        }
    }

    fn opponent(&self) -> Entity {
        self.opponents[self.opponent]
    }

    fn get_dialogue(&mut self, score: f32) -> Option<(String, String)> {
        let mut res = None;
        if self.opponent == 0 {
            // Alice's dialogues
            if let Some(last_eval) = self.last_eval {
                let score_diff = last_eval - score;
                println!(
                    "prev e: {}, new e: {}, diff: {}",
                    last_eval, score, score_diff
                );
                if score < -5. {
                    res = Some(("neutral", "Oof, now I'm in trouble ..."));
                }
                if score_diff.abs() > 2. {
                    if score_diff < 0. {
                        // player made a mistake (probably)
                        res = Some((
                            "neutral",
                            "Oh, that looks like a mistake ?\nWell it happens.",
                        ));
                    } else if score < 0. {
                        // alice made a mistake (probably)
                        res = Some(("weary", "Ugh, I think I blundered...\nDon't you wish you could \nundo your moves sometimes ?"));
                    }
                }
            } else {
                res = Some(("happy", "You got it !\nNow may the best player win !"));
            }
        } else if self.opponent == 1 {
            // Carl's dialogues
            if let Some(last_eval) = self.last_eval {
                let score_diff = last_eval - score;
                if score_diff.abs() > 2. {
                    if score_diff < 0. {
                        // player made a mistake (probably)
                        res = Some(("smug", "All according to my calculations."));
                    } else if self.should_undo(score) {
                        // Carl made a mistake (probably)
                        let line = if self.carl_lines.len() > 1 {
                            self.carl_lines.pop().unwrap()
                        } else {
                            self.carl_lines[0].clone()
                        };
                        return Some(("neutral".to_string(), line));
                    } else if score < 0. {
                        res = Some(("panicked", "Nothing is working !!"));
                    }
                }
            }
        }
        // make sure no dialogue line is sent twice
        if let Some((face, line)) = res {
            if !self.lines_sent.insert(line.to_string()) {
                return None;
            }
            return Some((face.to_string(), line.to_string()));
        }
        None
    }

    fn cached_moves_mut(&mut self, turn: u32) -> Option<&mut Vec<(f32, Pos, Vec<Action>)>> {
        if turn == self.turn {
            return Some(&mut self.cached_moves);
        }
        None
    }

    fn update_cached_moves(&mut self, moves: Vec<(f32, Pos, Vec<Action>)>, turn: u32) {
        self.cached_moves = moves;
        self.turn = turn;
    }

    fn should_undo(&self, score: f32) -> bool {
        if self.opponent == 1 && self.cached_moves.len() > 0 {
            if let Some(last_eval) = self.last_eval {
                return last_eval - score > 2. && score < 2.;
            }
        }
        false
    }
}

#[derive(Component)]
struct UndoingComp {
    max_speed: f32,
    speed: f32,
    ascending: bool,
}

impl UndoingComp {
    fn new() -> Self {
        UndoingComp {
            max_speed: 8000.,
            speed: 0.,
            ascending: true,
        }
    }
}

fn create_opponents(mut commands: Commands, server: Res<AssetServer>, mut game: ResMut<Game>) {
    let alice_entity = commands
        .spawn()
        .insert(Character::new(
            "Alice",
            vec![
                "happy".to_string(),
                "neutral".to_string(),
                "weary".to_string(),
            ],
            &server,
        ))
        .id();
    game.opponents.push(alice_entity);
    let carl_entity = commands
        .spawn()
        .insert(Character::new(
            "Carl Blok",
            vec![
                "exhausted".to_string(),
                "neutral".to_string(),
                "panicked".to_string(),
                "smug".to_string(),
            ],
            &server,
        ))
        .id();
    game.opponents.push(carl_entity);
}

fn play_move(
    mut commands: Commands,
    mut choss: ResMut<ChossGame>,
    mut piece_ents: ResMut<HashMap<Pos, Entity>>,
    mut game: ResMut<Game>,
    query_say: Query<(), With<Say>>,
    query_undo: Query<(), With<UndoingComp>>,
    mut query_text: Query<&mut Text, With<DialogueText>>,
    mut query_face: Query<&mut Handle<Image>, With<DialogueFace>>,
    server: Res<AssetServer>,
    audio: Res<Audio>,
    time: Res<Time>,
) {
    // only play the move if no one's talking and no one's undoing
    if query_say.is_empty()
        && query_undo.is_empty()
        && time.seconds_since_startup() - game.last_move_time > 1.
    {
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
                if let Ok(mut face) = query_face.get_single_mut() {
                    *face = server.load("empty.png");
                }
            }
            if choss.board.is_checked(color.next()) {
                audio.play(server.load("sounds/check.ogg"));
            } else if is_take {
                audio.play(server.load("sounds/take.ogg"));
            } else {
                audio.play(server.load("sounds/move.ogg"));
            }
            game.to_play = None;
            // check if the game is over
            if choss.board.moves(color.next(), true).len() == 0 {
                if choss.board.is_checked(color.next()) {
                    if color == choss.player {
                        game.status = GameStatus::Win;
                    } else {
                        game.status = GameStatus::Loss;
                    }
                } else {
                    game.status = GameStatus::Draw;
                }
            }
            game.last_move_time = time.seconds_since_startup();
        }
    }
}

fn mouse_button_input(
    q_camera: Query<(&Camera, &GlobalTransform)>,
    q_say: Query<(), With<Say>>,
    buttons: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    mut selected: ResMut<SelectedSquare>,
    mut game: ResMut<Game>,
    choss: ResMut<ChossGame>,
) {
    if buttons.just_released(MouseButton::Left) {
        // only take input when no one's talking
        if q_say.is_empty() && game.status == GameStatus::Playing {
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

#[derive(Component)]
struct AITask(Task<Vec<(f32, Pos, Vec<Action>)>>);

fn start_ai_turn(
    mut commands: Commands,
    mut game: ResMut<Game>,
    choss: Res<ChossGame>,
    moving_query: Query<(), With<MovingTo>>,
    query_undo: Query<(), With<UndoingComp>>,
) {
    if moving_query.is_empty()
        && query_undo.is_empty()
        && game.status == GameStatus::Playing
        && choss.player != choss.turn_color()
        && game.to_play.is_none()
    {
        // play the AI move
        if let Some(cached_moves) = game.cached_moves_mut(choss.turn) {
            let (_, pos, actions) = cached_moves.pop().unwrap();
            if cached_moves.len() == 0 {
                commands
                    .entity(game.opponent())
                    .insert(Say::new("panicked", "If this doesn't work ..."));
            }
            game.to_play = Some((pos, actions));
        } else {
            let value = choss.remaining_value();
            let depth = if value < 5. {
                4
            } else if value < 10. {
                2
            } else {
                1
            };
            println!("thinking with base depth {}", depth);
            let moves = negamax(&choss.board, choss.turn_color(), depth);
            // Randomly pick a move with that's not too far away from best in the 3 first moves
            let best_move = moves[0].clone();
            let best_score = best_move.0;
            let mut filtered_moves: Vec<_> = moves
                .into_iter()
                .take(3)
                .filter(|(score, _, _)| *score >= best_score - 3.)
                .collect();
            if filtered_moves.len() == 0 {
                // this shouldn't be possible but it seems like it is lol
                println!("wtf ? {}", best_score);
                filtered_moves = vec![best_move];
            }
            filtered_moves.shuffle(&mut rand::thread_rng());
            let (_, pos, actions) = filtered_moves.pop().unwrap();
            if let Some((face, text)) = game.get_dialogue(best_score) {
                commands
                    .entity(game.opponent())
                    .insert(Say::new(face, text));
            }
            // check if we must undo here
            if game.should_undo(best_score) {
                commands.spawn().insert(UndoingComp::new());
            } else {
                game.last_state = Some((*choss).clone());
                game.last_eval = Some(best_score);
                game.to_play = Some((pos, actions));
                game.update_cached_moves(filtered_moves, choss.turn);
            }
        }
    }
}

fn undo(
    mut commands: Commands,
    query_say: Query<(), With<Say>>,
    mut query_cam: Query<&mut Transform, With<Camera>>,
    mut query_undo: Query<(Entity, &mut UndoingComp)>,
    mut query_text: Query<&mut Text, With<DialogueText>>,
    mut query_face: Query<&mut Handle<Image>, With<DialogueFace>>,
    mut game: ResMut<Game>,
    mut choss: ResMut<ChossGame>,
    server: Res<AssetServer>,
    time: Res<Time>,
) {
    if query_say.is_empty() {
        if let Ok((entity, mut undoingcomp)) = query_undo.get_single_mut() {
            if let Ok(mut transform) = query_cam.get_single_mut() {
                transform.translation.x += undoingcomp.speed * time.delta_seconds();
                if transform.translation.x > 1000. {
                    transform.translation.x = -1000.;
                }
                if undoingcomp.ascending {
                    undoingcomp.speed += undoingcomp.max_speed * 0.5 * time.delta_seconds();
                    if undoingcomp.speed > undoingcomp.max_speed {
                        // "zenith" of the undoing, we can replace the board here
                        if let Ok(mut text) = query_text.get_single_mut() {
                            text.sections[0].value = "".to_string();
                        }
                        if let Ok(mut face) = query_face.get_single_mut() {
                            *face = server.load("empty.png");
                        }
                        *choss = game.last_state.clone().unwrap();
                        game.status = GameStatus::Placing;
                        undoingcomp.speed = undoingcomp.max_speed;
                        undoingcomp.ascending = false;
                    }
                } else {
                    undoingcomp.speed -= undoingcomp.max_speed * 0.5 * time.delta_seconds();
                    if undoingcomp.speed < 400. {
                        // undoing is over
                        game.last_move_time = time.seconds_since_startup() + 1.;
                        undoingcomp.speed = 0.;
                        transform.translation.x = 0.;
                        commands.entity(entity).despawn();
                    }
                }
            }
        }
    }
}

fn clean_up_pieces(commands: &mut Commands, piece_ents: &mut HashMap<Pos, Entity>) {
    for entity in piece_ents.values() {
        commands.entity(*entity).despawn();
    }
    piece_ents.clear();
}

fn start_game(
    query_say: Query<(), With<Say>>,
    mut commands: Commands,
    mut game: ResMut<Game>,
    mut choss: ResMut<ChossGame>,
) {
    if query_say.is_empty() && game.status == GameStatus::Preparing {
        if game.opponent == 0 {
            // start the alice game
            commands.entity(game.opponent()).insert(Say::new(
                "happy",
                "Welcome to the Choss club !\n\
                 It's your first game right ?\n\
                 Well you win if you capture my King,\n\
                 the piece with a cross on its head.\n\
                 Select a white piece to make a move.",
            ));
        } else {
            // start the carl game
            commands.entity(game.opponent()).insert(Say::new(
                "smug",
                "My name's Carl Brok.\nI've never lost a game here,\nso I don't expect much from you\nbut let's see what you got.",
            ));
        }
        // setup the board
        *choss = ChossGame::new(PieceColor::White);
        game.last_eval = Some(0.);
        game.cached_moves = Vec::new();
        game.status = GameStatus::Placing;
    }
}

fn place_pieces(
    mut commands: Commands,
    mut piece_ents: ResMut<HashMap<Pos, Entity>>,
    mut game: ResMut<Game>,
    choss: Res<ChossGame>,
    server: Res<AssetServer>,
) {
    if game.status == GameStatus::Placing {
        clean_up_pieces(&mut commands, &mut piece_ents);
        for (i, square) in choss.board.squares.iter().enumerate() {
            if let Some((color, piece)) = square {
                let handle = server
                    .load(format!("choss_pieces/{}.png", piece_tex_name(piece, color)).as_str());
                let pos = choss.board.pos(i);
                piece_ents.insert(
                    pos,
                    commands
                        .spawn_bundle(SpriteBundle {
                            sprite: Sprite {
                                custom_size: Some(Vec2::new(SIZE as f32 * 0.8, SIZE as f32 * 0.8)),
                                ..Default::default()
                            },
                            texture: handle,
                            transform: choss.board_to_world(pos),
                            ..Default::default()
                        })
                        .id(),
                );
            }
        }
        game.status = GameStatus::Playing;
    }
}

fn end_game(mut commands: Commands, mut game: ResMut<Game>) {
    if game.status == GameStatus::Win
        || game.status == GameStatus::Loss
        || game.status == GameStatus::Draw
    {
        if game.opponent == 0 {
            // end the alice game
            if game.status == GameStatus::Win {
                commands.entity(game.opponent()).insert(Say::new(
                    "happy",
                    "Wow you actually won ! Amazing !\nWell, your next opponent won't be as easy.\nHe's kinda annoying but really strong.",
                ));
            } else if game.status == GameStatus::Loss {
                commands.entity(game.opponent()).insert(Say::new(
                    "happy",
                    "Chockmate ! I won but it's okay,\nit was your first game after all.\nAll this reflexion got me tired though,\nI'm going to relax and leave you with Carl,\nhe's strong so you'll learn a lot !",
                ));
            } else {
                commands.entity(game.opponent()).insert(Say::new(
                    "happy",
                    "Uh, it's a draw then ! Not bad !\nAll this reflexion got me tired though,\nI'm going to relax and leave you with Carl,\nhe's strong so you'll learn a lot !",
                ));
            }
            game.opponent = 1;
            game.status = GameStatus::Preparing;
        } else {
            // end the carl game
            if game.status == GameStatus::Win {
                commands.entity(game.opponent()).insert(Say::new(
                    "exhausted",
                    "I - I actually lost...\n\
                    I'm starting to realise now .\n\
                    Even since I started using it,\n\
                    I stopped improving...\n\
                    Was this ability my undoing ? . . . . .",
                ));
                game.status = GameStatus::Ending;
            } else if game.status == GameStatus::Loss {
                commands.entity(game.opponent()).insert(Say::new(
                    "smug",
                    "Chockmate. I won as expected.\nStay if you want to play me again !",
                ));
                game.status = GameStatus::Preparing;
            } else {
                commands.entity(game.opponent()).insert(Say::new(
                    "neutral",
                    "Eh, I let you draw on purpose.\nStay if you want to play me again !",
                ));
                game.status = GameStatus::Preparing;
            }
        }
    }
}

#[derive(Component)]
struct Title;

fn display_end(
    mut commands: Commands,
    mut piece_ents: ResMut<HashMap<Pos, Entity>>,
    mut game: ResMut<Game>,
    query_title: Query<Entity, With<Title>>,
    server: Res<AssetServer>,
    keys: Res<Input<KeyCode>>,
    query_say: Query<(), With<Say>>,
    mut query_text: Query<&mut Text, With<DialogueText>>,
    mut query_face: Query<&mut Handle<Image>, With<DialogueFace>>,
    audio: Res<Audio>,
) {
    if game.status == GameStatus::Ending && query_say.is_empty() {
        if let Ok(entity) = query_title.get_single() {
            if keys.just_pressed(KeyCode::R) {
                // R was pressed
                commands.entity(entity).despawn();
                game.opponent = 0;
                game.status = GameStatus::Preparing;
            }
        } else {
            // clean the pieces
            clean_up_pieces(&mut commands, &mut piece_ents);
            if let Ok(mut text) = query_text.get_single_mut() {
                text.sections[0].value = "".to_string();
            }
            if let Ok(mut face) = query_face.get_single_mut() {
                *face = server.load("empty.png");
            }
            // display the title
            audio.play(server.load("sounds/take.ogg"));
            commands
                .spawn_bundle(SpriteBundle {
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(570., 100.)),
                        ..Default::default()
                    },
                    texture: server.load("title.png"),
                    ..Default::default()
                })
                .insert(Title);
        }
    }
}

pub struct SelectedSquare(Option<Pos>);

pub struct HoveredSquare(Option<Pos>);

pub struct Undoing;

impl Plugin for Undoing {
    fn build(&self, app: &mut App) {
        app.insert_resource(ChossGame::new(PieceColor::White))
            .add_plugin(CharacterPlugin)
            .insert_resource(Game::new())
            .insert_resource(HashMap::<Pos, Entity>::new())
            .insert_resource(SelectedSquare(None))
            .insert_resource(HoveredSquare(None))
            .add_startup_system(create_opponents)
            .add_startup_system(draw_choss)
            .add_system(play_move.label("play"))
            .add_system(mouse_button_input)
            .add_system(display_moves)
            .add_system(move_to)
            .add_system(die)
            .add_system(promote)
            .add_system(start_ai_turn.after("play"))
            // ensure dialogue gets instanciated before the next play_move call
            .add_system(start_game.label("start"))
            .add_system(end_game.after("start"))
            .add_system(place_pieces)
            .add_system(undo)
            .add_system(display_end.before("start"));
    }
}
