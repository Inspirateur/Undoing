use crate::{
    ai::piece_value,
    board::Board,
    make_board::*,
    piece::{Action, Color, Piece},
    pos::Pos,
};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension};
use bevy::render::texture::BevyDefault;
use std::collections::HashMap;

pub const SIZE: u32 = 64;
const HSIZE: f32 = SIZE as f32 / 2.;

pub struct ChossGame {
    pub board: Board,
    pub player: Color,
    turn: u32,
}

impl ChossGame {
    pub fn new(player: Color) -> Self {
        ChossGame {
            board: standard_board(),
            player: player,
            turn: 0,
        }
    }

    pub fn world_to_board(&self, world_pos: Vec2) -> Pos {
        let world_pos = (world_pos
            + Vec2::new(
                self.board.width as f32 * HSIZE,
                self.board.height as f32 * HSIZE,
            ))
            / SIZE as f32;
        Pos(
            world_pos.x as i32,
            self.board.height as i32 - 1 - world_pos.y as i32,
        )
    }

    pub fn board_to_world(&self, pos: Pos) -> Transform {
        Transform::from_xyz(
            -HSIZE * self.board.width as f32 + (0.5 + pos.0 as f32) * SIZE as f32,
            HSIZE * self.board.height as f32 - (0.5 + pos.1 as f32) * SIZE as f32,
            0.,
        )
    }

    pub fn turn_color(&self) -> Color {
        if self.turn % 2 == 0 {
            Color::White
        } else {
            Color::Black
        }
    }

    fn safe_moves(&self, piece: Piece, from: Pos) -> Vec<Vec<Action>> {
        self.board.filter_safe_moves(
            self.turn_color(),
            from,
            piece.moves(&self.board, from, self.turn_color()),
        )
    }

    pub fn playable_moves(&self, from: Pos) -> Option<Vec<Vec<Action>>> {
        if let Some(Some((color, piece))) = self.board.get(from) {
            if *color == self.player && self.player == self.turn_color() {
                return Some(self.safe_moves(*piece, from));
            }
        }
        None
    }

    pub fn playable_move(&self, from: Pos, to: Pos) -> Option<Vec<Action>> {
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

    pub fn play(&mut self, pos: Pos, actions: &Vec<Action>) {
        let color = self.turn_color();
        self.board = self.board.play(color, pos, &actions);
        self.turn += 1;
    }

    pub fn remaining_value(&self) -> f32 {
        let mut sum = 0.;
        for square in &self.board.squares {
            if let Some((color, piece)) = square {
                if *color == self.player && *piece != Piece::King {
                    sum += piece_value(*piece);
                }
            }
        }
        sum
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

fn piece_tex_name(piece: &Piece, color: &Color) -> String {
    format!("{}_", piece) + &format!("{:?}", color)[0..1].to_lowercase()
}

pub fn draw_choss(
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
