use crate::piece::{Action, Color, Piece};
use crate::pos::Pos;
use std::fmt::Display;

type Square = Option<(Color, Piece)>;

#[derive(Clone)]
pub struct Board {
    pub width: usize,
    pub height: usize,
    pub squares: Vec<Square>,
}

impl Board {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            squares: vec![None; width * height],
        }
    }

    pub fn in_bound(&self, pos: Pos) -> bool {
        0 <= pos.0 && pos.0 < self.width as i32 && 0 <= pos.1 && pos.1 < self.height as i32
    }

    pub fn get(&self, pos: Pos) -> Option<&Square> {
        if !self.in_bound(pos) {
            return None;
        }
        Some(&self.squares[self.i(pos)])
    }

    pub fn set(&mut self, pos: Pos, square: Square) {
        let i = self.i(pos);
        self.squares[i] = square;
    }

    pub fn pos(&self, i: usize) -> Pos {
        Pos((i % self.width) as i32, (i / self.width) as i32)
    }

    pub fn i(&self, pos: Pos) -> usize {
        (pos.0 + pos.1 * self.width as i32) as usize
    }

    fn king_pos(&self, color: Color) -> Option<Pos> {
        for (i, square) in self.squares.iter().enumerate() {
            if let Some((piece_color, piece)) = square {
                if *piece_color == color && *piece == Piece::King {
                    return Some(self.pos(i));
                }
            }
        }
        None
    }

    fn is_checked(&self, color: Color) -> bool {
        // if this panic then there's no king of this color on the board lol
        let king_pos = self.king_pos(color).unwrap();
        // check if the opponent can capture the king
        let o_color = color.next();
        for (_, actions) in self.takes(o_color, false) {
            for action in actions {
                if let Action::Go(go_pos) = action {
                    if go_pos == king_pos {
                        return true;
                    }
                } else if let Action::Take(take_pos) = action {
                    if take_pos == king_pos {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn filter_safe_moves(
        &self,
        color: Color,
        pos: Pos,
        moves: Vec<Vec<Action>>,
    ) -> Vec<Vec<Action>> {
        moves
            .into_iter()
            .filter(|actions| {
                let board = self.play(color, pos, actions);
                !board.is_checked(color)
            })
            .collect()
    }

    pub fn takes(&self, color: Color, safe_moves: bool) -> Vec<(Pos, Vec<Action>)> {
        // generate all taking moves for color
        let mut res = Vec::new();
        for (i, square) in self.squares.iter().enumerate() {
            if let Some((piece_color, piece)) = square {
                if *piece_color == color {
                    let pos = self.pos(i);
                    let mut moves = piece.takes(self, pos, color);
                    if safe_moves {
                        moves = self.filter_safe_moves(color, pos, moves);
                    }
                    res.extend(moves.into_iter().map(|actions| (pos, actions)));
                }
            }
        }
        res
    }

    pub fn moves(&self, color: Color, safe_moves: bool) -> Vec<(Pos, Vec<Action>)> {
        // generate all moves for color
        let mut res = Vec::new();
        for (i, square) in self.squares.iter().enumerate() {
            if let Some((piece_color, piece)) = square {
                if *piece_color == color {
                    let pos = self.pos(i);
                    let mut moves = piece.moves(self, pos, color);
                    if safe_moves {
                        moves = self.filter_safe_moves(color, pos, moves);
                    }
                    res.extend(moves.into_iter().map(|actions| (pos, actions)));
                }
            }
        }
        res
    }

    fn begin_turn(&mut self, color: Color) {
        for i in 0..self.squares.len() {
            if let Some((p_color, piece)) = self.squares[i] {
                if p_color == color {
                    self.squares[i] = Some((p_color, piece.begin_turn()))
                }
            }
        }
    }

    fn moved(&mut self, start: Pos, target: Pos) {
        let (color, piece) = self.get(target).unwrap().unwrap();
        self.set(target, Some((color, piece.moved(start, target))));
    }

    pub fn play(&self, color: Color, pos: Pos, actions: &Vec<Action>) -> Self {
        let mut res = self.clone();
        res.begin_turn(color);
        let mut last_pos = pos;
        // we unwrap because no move can be played out of the board's bound
        let square = self.get(pos).unwrap();
        for action in actions {
            match action {
                Action::Go(go_pos) => {
                    res.set(last_pos, None);
                    res.set(*go_pos, *square);
                    res.moved(last_pos, *go_pos);
                    last_pos = *go_pos;
                }
                Action::Take(take_pos) => res.set(*take_pos, None),
                Action::Promotion(piece) => {
                    let (color, _) = square.unwrap();
                    res.set(last_pos, Some((color, *piece)));
                }
            };
        }
        res
    }
}

impl Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, square) in self.squares.iter().enumerate() {
            if i % self.width == 0 && i != 0 {
                write!(f, "\n")?;
            }
            if let Some((color, piece)) = square {
                write!(
                    f,
                    "{} ",
                    match color {
                        Color::Black => match piece {
                            Piece::Pawn {
                                orientation: _,
                                status: _,
                            } => "♙",
                            Piece::Knight => "♘",
                            Piece::Bishop => "♗",
                            Piece::Rook => "♖",
                            Piece::Queen => "♕",
                            Piece::King => "♔",
                        },
                        Color::White => match piece {
                            Piece::Pawn {
                                orientation: _,
                                status: _,
                            } => "♟︎",
                            Piece::Knight => "♞",
                            Piece::Bishop => "♝",
                            Piece::Rook => "♜",
                            Piece::Queen => "♛",
                            Piece::King => "♚",
                        },
                    }
                )?;
            } else {
                write!(f, "· ")?;
            }
        }
        Ok(())
    }
}
