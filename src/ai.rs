use crate::board::Board;
use crate::piece::{Action, Color, Piece};
use crate::pos::Pos;
const MAX_DEPTH: i32 = -6;

pub fn piece_value(piece: Piece) -> f32 {
    match piece {
        Piece::Pawn {
            orientation: _,
            status: _,
        } => 1.,
        Piece::Knight => 3.,
        Piece::Bishop => 3.5,
        Piece::Rook => 5.,
        Piece::Queen => 9.,
        Piece::King => 1000.,
    }
}

fn move_value(board: &Board, pos: Pos, actions: &Vec<Action>) -> f32 {
    // compute the material value of a move, *assuming that if the moves win any material the piece is lost at 90%*
    let (color, piece) = board.get(pos).unwrap().unwrap();
    let mut value = 0.;
    for action in actions {
        match *action {
            Action::Go(go_pos) => {
                if let Some(Some((o_color, o_piece))) = board.get(go_pos) {
                    value += piece_value(*o_piece) * if *o_color == color { -1. } else { 1. };
                }
            }
            Action::Take(take_pos) => {
                if let Some(Some((o_color, o_piece))) = board.get(take_pos) {
                    value += piece_value(*o_piece) * if *o_color == color { -1. } else { 1. };
                }
            }
            Action::Promotion(n_piece) => {
                value += piece_value(n_piece);
            }
        }
    }
    if value > 0. {
        value -= piece_value(piece) * 0.9;
    }
    value
}

fn mat_score(board: &Board) -> f32 {
    board
        .squares
        .iter()
        .map(|square| {
            if let Some((color, piece)) = square {
                piece_value(*piece) * if *color == Color::White { 1. } else { -1. }
            } else {
                0.
            }
        })
        .fold(0., |a, b| a + b)
}

fn _negamax(board: &Board, depth: i32, mut alpha: f32, beta: f32, color: Color) -> f32 {
    let mut moves;
    if depth <= MAX_DEPTH {
        return mat_score(board) * if color == Color::White { 1. } else { -1. };
    } else if depth <= 0 {
        // if we're out of depth, only explore taking moves
        moves = board.takes(color, false);
    } else {
        moves = board.moves(color, false);
    }
    // sort the moves with move_value heuristic
    moves.sort_by(|(pos1, actions1), (pos2, actions2)| {
        move_value(board, *pos2, actions2)
            .partial_cmp(&move_value(board, *pos1, actions1))
            .unwrap()
    });
    let mut best_score = f32::NEG_INFINITY;

    for (pos, actions) in moves {
        best_score = f32::max(
            best_score,
            -_negamax(
                &board.play(color, pos, &actions),
                depth - 1,
                -beta,
                -alpha,
                color.next(),
            ),
        );
        alpha = f32::max(alpha, best_score);
        if alpha >= beta {
            return alpha;
        }
    }
    if depth <= 0 {
        // if we're out of depth, consider that the score can't be worse than current board eval
        best_score.max(mat_score(board) * if color == Color::White { 1. } else { -1. })
    } else {
        best_score
    }
}

pub fn negamax(board: &Board, color: Color, depth: u32) -> Vec<(f32, Pos, Vec<Action>)> {
    println!("{}", board);
    let mut moves = board.moves(color, true);
    // sort the moves with move_value heuristic
    moves.sort_by(|(pos1, actions1), (pos2, actions2)| {
        move_value(board, *pos2, actions2)
            .partial_cmp(&move_value(board, *pos1, actions1))
            .unwrap()
    });
    let mut res = Vec::new();
    for (pos, actions) in moves {
        let curr_board = board.play(color, pos, &actions);
        let mut score = -_negamax(
            &curr_board,
            depth as i32 - 1,
            f32::NEG_INFINITY,
            f32::INFINITY,
            color.next(),
        );
        // compute an auxiliary score based on how many safe moves are available for both player in the next position
        let own_moves = curr_board.moves(color, false).len() as f32;
        let op_moves = curr_board.moves(color.next(), true).len() as f32;
        if op_moves == 0. {
            // if the opponent has no legal move it is either a draw or a win
            if curr_board.is_checked(color.next()) {
                score = f32::INFINITY;
            } else {
                score = 0.;
            }
        } else {
            // else, the score is raised if the position has more moves for the player and less for the opponent
            // cannot exceed the value of a pawn
            score += (own_moves / 100. - op_moves / 100.).min(1.);
        }
        res.push((score, pos, actions));
    }
    res.sort_by(|(score1, _, _), (score2, _, _)| score2.partial_cmp(score1).unwrap());
    res
}
