use crate::board::Board;
use crate::pos::{Pos, DIAGS, LINES, LOS};
use itertools::iproduct;
use std::fmt::Display;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn next(self) -> Self {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Go(Pos),
    Take(Pos),
    Promotion(Piece),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PawnStatus {
    CanLeap,
    JustLeaped,
    CannotLeap,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Piece {
    Pawn {
        orientation: Pos,
        status: PawnStatus,
    },
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl Display for Piece {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Piece::Pawn {
                orientation: _,
                status: _,
            } => "pawn".to_string(),
            piece => format!("{:?}", piece).to_lowercase(),
        };
        f.write_str(&str)
    }
}

fn pawn_promotion(
    board: &Board,
    pos: Pos,
    orientation: Pos,
    moves: Vec<Vec<Action>>,
) -> Vec<Vec<Action>> {
    // Handle promotions
    let mut res_prom: Vec<Vec<Action>> = Vec::new();
    for actions in moves.into_iter() {
        let mut last_pos = pos;
        for action in actions.iter() {
            if let Action::Go(go_pos) = action {
                last_pos = *go_pos;
            }
        }
        if board.get(last_pos + orientation).is_none() {
            let mut action_q = actions.clone();
            action_q.push(Action::Promotion(Piece::Queen));
            let mut action_n = actions.clone();
            action_n.push(Action::Promotion(Piece::Knight));
            res_prom.push(action_q);
            res_prom.push(action_n);
        } else {
            res_prom.push(actions.clone())
        }
    }
    res_prom
}

fn pawn_takes(board: &Board, pos: Pos, color: Color, orientation: Pos) -> Vec<Vec<Action>> {
    let mut res = Vec::new();
    // Taking moves
    for diag_dir in orientation.neighbors() {
        let diag_pos = diag_dir + pos;
        let diag = board.get(diag_pos);
        // if it's a square
        if let Some(square) = diag {
            // if there's a piece on a taking square
            if let Some((other_color, _)) = square {
                // if it's an opponent
                if color != *other_color {
                    res.push(vec![Action::Go(diag_pos)]);
                }
            } else {
                // the square is empty
                let en_passant_pos = diag_pos + orientation * -1;
                // if there's a piece in en passant pos
                if let Some(Some((other_color, piece))) = board.get(en_passant_pos) {
                    // if it's an opponent
                    if color != *other_color {
                        // if it's a pawn
                        if let Piece::Pawn {
                            orientation: _,
                            status,
                        } = piece
                        {
                            // if it just leaped forward
                            if *status == PawnStatus::JustLeaped {
                                res.push(vec![Action::Go(diag_pos), Action::Take(en_passant_pos)])
                            }
                        }
                    }
                }
            }
        }
    }
    pawn_promotion(board, pos, orientation, res)
}

fn pawn_moves(
    board: &Board,
    pos: Pos,
    color: Color,
    orientation: Pos,
    status: PawnStatus,
) -> Vec<Vec<Action>> {
    let mut moves = Vec::new();
    // Non-Taking moves
    let forward_pos = orientation + pos;
    let leap_pos = orientation * 2 + pos;
    // if there is a free cell forward
    if let Some(None) = board.get(forward_pos) {
        moves.push(vec![Action::Go(forward_pos)]);
        // if we can leap
        if status == PawnStatus::CanLeap {
            // and the square is available
            if let Some(None) = board.get(leap_pos) {
                moves.push(vec![Action::Go(leap_pos)]);
            }
        }
    }
    moves = pawn_promotion(board, pos, orientation, moves);
    moves.extend(pawn_takes(board, pos, color, orientation));
    moves
}

fn knight_takes(board: &Board, pos: Pos, color: Color) -> Vec<Vec<Action>> {
    iproduct!([-2, 2], [-1, 1])
        .flat_map(|(long, short)| [Pos(long, short) + pos, Pos(short, long) + pos])
        .filter(|take_pos| {
            if let Some(Some((other_color, _))) = board.get(*take_pos) {
                if color != *other_color {
                    return true;
                }
            }
            false
        })
        .map(|take_pos| vec![Action::Go(take_pos)])
        .collect()
}

fn knight_moves(board: &Board, pos: Pos, color: Color) -> Vec<Vec<Action>> {
    iproduct!([-2, 2], [-1, 1])
        .flat_map(|(long, short)| [Pos(long, short) + pos, Pos(short, long) + pos])
        .filter(|take_pos| {
            if let Some(square) = board.get(*take_pos) {
                if let Some((other_color, _)) = square {
                    if color == *other_color {
                        return false;
                    }
                }
                return true;
            }
            return false;
        })
        .map(|take_pos| vec![Action::Go(take_pos)])
        .collect()
}

fn los_takes(board: &Board, pos: Pos, color: Color, dirs: &[Pos]) -> Vec<Vec<Action>> {
    let mut moves = Vec::new();
    for dir in dirs {
        let mut curr_pos = pos;
        loop {
            curr_pos = curr_pos + *dir;
            let line = board.get(curr_pos);
            if let Some(square) = line {
                if let Some((other_color, _)) = square {
                    // it's a square with a piece
                    if color != *other_color {
                        // it's a square with an opponent
                        moves.push(vec![Action::Go(curr_pos)]);
                    }
                    break;
                }
            } else {
                // it's out of the board
                break;
            }
        }
    }
    moves
}

fn los_moves(board: &Board, pos: Pos, color: Color, dirs: &[Pos]) -> Vec<Vec<Action>> {
    let mut moves = Vec::new();
    for dir in dirs {
        let mut curr_pos = pos;
        loop {
            curr_pos = curr_pos + *dir;
            let line = board.get(curr_pos);
            if let Some(square) = line {
                if let Some((other_color, _)) = square {
                    // it's a square with a piece
                    if color != *other_color {
                        // it's a square with an opponent
                        moves.push(vec![Action::Go(curr_pos)]);
                    }
                    break;
                } else {
                    // it's a free square
                    moves.push(vec![Action::Go(curr_pos)]);
                }
            } else {
                // it's out of the board
                break;
            }
        }
    }
    moves
}

fn king_takes(board: &Board, pos: Pos, color: Color) -> Vec<Vec<Action>> {
    LOS.iter()
        .map(|los_dir| *los_dir + pos)
        .filter(|take_pos| {
            if let Some(Some((other_color, _))) = board.get(*take_pos) {
                if color != *other_color {
                    return true;
                }
            }
            false
        })
        .map(|take_pos| vec![Action::Go(take_pos)])
        .collect()
}

fn king_moves(board: &Board, pos: Pos, color: Color) -> Vec<Vec<Action>> {
    // NOTE: we don't do castling because in the game you place your pieces at the start of the match
    // so it's both useless and inapplicable in our case (also a pain to implement)
    LOS.iter()
        .map(|los_dir| *los_dir + pos)
        .filter(|take_pos| {
            if let Some(square) = board.get(*take_pos) {
                if let Some((other_color, _)) = square {
                    if color == *other_color {
                        return false;
                    }
                }
                return true;
            }
            return false;
        })
        .map(|take_pos| vec![Action::Go(take_pos)])
        .collect()
}

impl Piece {
    pub fn begin_turn(self) -> Self {
        match self {
            Piece::Pawn {
                orientation,
                status,
            } => {
                let mut newstatus = status;
                if status == PawnStatus::JustLeaped {
                    newstatus = PawnStatus::CannotLeap;
                }
                Piece::Pawn {
                    orientation,
                    status: newstatus,
                }
            }
            _ => self,
        }
    }

    pub fn moved(self, start: Pos, target: Pos) -> Self {
        match self {
            Piece::Pawn {
                orientation,
                status: _,
            } => {
                if start + orientation * 2 == target {
                    Piece::Pawn {
                        orientation,
                        status: PawnStatus::JustLeaped,
                    }
                } else {
                    Piece::Pawn {
                        orientation,
                        status: PawnStatus::CannotLeap,
                    }
                }
            }
            _ => self,
        }
    }

    pub fn moves(self, board: &Board, pos: Pos, color: Color) -> Vec<Vec<Action>> {
        match self {
            Piece::Pawn {
                orientation,
                status,
            } => pawn_moves(board, pos, color, orientation, status),
            Piece::Knight => knight_moves(board, pos, color),
            Piece::Bishop => los_moves(board, pos, color, &DIAGS),
            Piece::Rook => los_moves(board, pos, color, &LINES),
            Piece::Queen => los_moves(board, pos, color, &LOS),
            Piece::King => king_moves(board, pos, color),
        }
    }

    pub fn takes(self, board: &Board, pos: Pos, color: Color) -> Vec<Vec<Action>> {
        match self {
            Piece::Pawn {
                orientation,
                status: _,
            } => pawn_takes(board, pos, color, orientation),
            Piece::Knight => knight_takes(board, pos, color),
            Piece::Bishop => los_takes(board, pos, color, &DIAGS),
            Piece::Rook => los_takes(board, pos, color, &LINES),
            Piece::Queen => los_takes(board, pos, color, &LOS),
            Piece::King => king_takes(board, pos, color),
        }
    }
}
