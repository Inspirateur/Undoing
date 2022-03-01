use crate::{
    ai::negamax,
    board::Board,
    make_board::{halved_board, invert_color, standard_board},
    piece::{Action, Color as PieceColor, Piece},
    pos::Pos,
};
use bevy::prelude::*;
use bevy::render::color::Color;
use bevy::render::render_resource::{Extent3d, TextureDimension};
use bevy::render::texture::BevyDefault;
use std::collections::HashMap;
const SIZE: u32 = 64;
const HSIZE: f32 = SIZE as f32 / 2.;

pub struct ChossGame {
    pub board: Board,
    player: PieceColor,
    turn: u32,
}

impl ChossGame {
    pub fn new(player: PieceColor) -> Self {
        ChossGame {
            board: invert_color(standard_board()),
            player: player,
            turn: 0,
        }
    }

    fn world_to_board(&self, world_pos: Vec2) -> Pos {
        let world_pos = (world_pos
            + Vec2::new(
                self.board.width as f32 * HSIZE,
                self.board.height as f32 * HSIZE,
            ))
            / SIZE as f32;
        Pos(world_pos.x as i32, world_pos.y as i32)
    }

    fn board_to_world(&self, pos: Pos) -> Transform {
        Transform::from_xyz(
            -HSIZE * self.board.width as f32 + (0.5 + pos.0 as f32) * SIZE as f32,
            -HSIZE * self.board.height as f32 + (0.5 + pos.1 as f32) * SIZE as f32,
            0.,
        )
    }

    pub fn turn_color(&self) -> PieceColor {
        if self.turn % 2 == 0 {
            PieceColor::White
        } else {
            PieceColor::Black
        }
    }

    fn safe_moves(&self, piece: Piece, from: Pos) -> Vec<Vec<Action>> {
        self.board.filter_safe_moves(
            self.turn_color(),
            from,
            piece.moves(&self.board, from, self.turn_color()),
        )
    }

    fn playable_moves(&self, from: Pos) -> Option<Vec<Vec<Action>>> {
        if let Some(Some((color, piece))) = self.board.get(from) {
            if *color == self.player && self.player == self.turn_color() {
                return Some(self.safe_moves(*piece, from));
            }
        }
        None
    }

    fn playable_move(&self, from: Pos, to: Pos) -> Option<Vec<Action>> {
        if let Some(moves) = self.playable_moves(from) {
            for actions in moves {
                for action in &actions {
                    if let Action::Go(pos) = action {
                        if *pos == to {
                            return Some(actions);
                        }
                    }
                }
            }
        }
        None
    }

    fn play(&mut self, pos: Pos, actions: &Vec<Action>) {
        let color = self.turn_color();
        self.board = self.board.play(color, pos, &actions);
        self.turn += 1;
    }
}

fn board_tex(board: &Board, size: u32) -> Image {
    let mut data = vec![255; 4 * board.width * board.height * size as usize * size as usize];
    for i in 0..(data.len() / 4) {
        if (i / (board.width * size as usize * size as usize)
            + (i / size as usize) % board.width as usize)
            % 2
            == 0
        {
            data[i * 4] = 200;
            data[i * 4 + 1] = 200;
            data[i * 4 + 2] = 200;
        } else {
            data[i * 4] = 100;
            data[i * 4 + 1] = 100;
            data[i * 4 + 2] = 200;
        }
    }
    Image::new(
        Extent3d {
            width: board.width as u32 * size,
            height: board.height as u32 * size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        BevyDefault::bevy_default(),
    )
}

fn piece_tex_name(piece: &Piece, color: &PieceColor) -> String {
    format!("{}_", piece) + &format!("{:?}", color)[0..1].to_lowercase()
}

fn draw_choss(
    mut commands: Commands,
    choss: Res<ChossGame>,
    mut textures: ResMut<Assets<Image>>,
    mut piece_ents: ResMut<HashMap<Pos, Entity>>,
    server: Res<AssetServer>,
) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    let board_tex = board_tex(&choss.board, SIZE);
    commands.spawn_bundle(SpriteBundle {
        texture: textures.add(board_tex),
        ..Default::default()
    });
    for (i, square) in choss.board.squares.iter().enumerate() {
        if let Some((color, piece)) = square {
            let handle =
                server.load(format!("choss_pieces/{}.png", piece_tex_name(piece, color)).as_str());
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
}

fn screen_to_world(
    window: &Window,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    screen_pos: Vec2,
) -> Vec2 {
    // get the size of the window
    let window_size = Vec2::new(window.width() as f32, window.height() as f32);

    // convert screen position [0..resolution] to ndc [-1..1] (gpu coordinates)
    let ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;

    // matrix for undoing the projection and camera transform
    let ndc_to_world = camera_transform.compute_matrix() * camera.projection_matrix.inverse();

    // use it to convert ndc to world-space coordinates
    let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));

    // reduce it to a 2D value
    world_pos.truncate()
}

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
        println!("Playing {:?} from {:?}", action, pos);
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
                println!("Clicked on {:?}", pos);
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

fn die(mut commands: Commands, mut query: Query<Entity, With<Die>>, time: Res<Time>) {
    for ent in query.iter_mut() {
        // for now we just despawn the entity, might do fancy things later
        commands.entity(ent).despawn();
    }
}

fn update_board(
    commands: Commands,
    choss: ResMut<ChossGame>,
    piece_ents: ResMut<HashMap<Pos, Entity>>,
) {
    if choss.is_changed() {
        // a play has been made, check if it's the AI's turn
        if choss.player != choss.turn_color() {
            // play the AI move
            println!("thinking ...");
            let (pos, actions) = negamax(&choss.board, choss.turn_color(), 1).unwrap();
            println!("found a move");
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
            .add_system(update_board);
    }
}
