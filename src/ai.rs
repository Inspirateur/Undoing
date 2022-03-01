use crate::board::Board;
use crate::pgn::move2pgn;
use crate::piece::{Action, Color, Piece};
use crate::pos::Pos;
use itertools::Itertools;
use rand::Rng;

fn piece_value(piece: Piece) -> f32 {
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
    if depth <= 0 {
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

pub fn negamax(board: &Board, color: Color, depth: u32) -> Option<(Pos, Vec<Action>)> {
    let mut moves = board.moves(color, true);
    // sort the moves with move_value heuristic
    moves.sort_by(|(pos1, actions1), (pos2, actions2)| {
        move_value(board, *pos2, actions2)
            .partial_cmp(&move_value(board, *pos1, actions1))
            .unwrap()
    });
    let mut alpha = f32::NEG_INFINITY;
    let beta = f32::INFINITY;
    let mut best_score = f32::NEG_INFINITY;
    let mut best_move = None;
    for (pos, actions) in moves {
        let curr_board = board.play(color, pos, &actions);
        let mut score = -_negamax(&curr_board, depth as i32 - 1, -beta, -alpha, color.next());
        // since the score can change right after, we take -2 off alpha just to be safe
        // otherwise a move that has been cut off by _negamax could be chosen after the score update,
        // which is very bad because _negamax cut it off early knowing that it could NOT be chosen
        alpha = alpha.max(score - 2.);
        // compute an auxiliary score based on how many safe moves are available for both player in the next position
        let own_moves = curr_board.moves(color, false).len() as f32;
        let op_moves = curr_board.moves(color.next(), true).len() as f32;
        if op_moves == 0. {
            // if the opponent has no legal move, the score is 0 (pat)
            score = 0.;
        } else {
            // else, the score is raised if the position has more moves for the player and less for the opponent
            // cannot exceed the value of a pawn
            score += (own_moves / 100. - op_moves / 100.).min(1.);
        }
        if score > best_score {
            best_move = Some((pos, actions));
            best_score = score;
        }
    }
    best_move
}

pub fn random_move(board: &Board, color: Color) -> Option<(Pos, Vec<Action>)> {
    let all_moves = board.moves(color, true);
    if all_moves.len() == 0 {
        return None;
    }
    Some(all_moves[rand::thread_rng().gen_range(0..all_moves.len())].clone())
}

pub fn auto_play(mut board: Board, starting_player: Color, depth: u32) -> String {
    let mut pgn_moves: Vec<String> = Vec::new();
    let mut player = starting_player;
    let mut turn = 0;
    loop {
        println!("{}\n-------------------\n", board);
        let move_opt = negamax(&board, player, depth);
        if move_opt.is_none() {
            println!("\nNo more valid moves");
            break;
        }
        let (pos, actions) = move_opt.unwrap();
        let pgn_move = move2pgn(pos, &actions);
        pgn_moves.push(pgn_move);
        board = board.play(player, pos, &actions);
        player = player.next();
        turn += 1;
        if turn >= 100 {
            println!("\nGame too long");
            break;
        }
    }
    pgn_moves.iter().join(" ")
}

mod tests {
    use crate::{
        ai::auto_play,
        ai::mat_score,
        ai::negamax,
        board::Board,
        make_board::invert_color,
        make_board::{halved_board, standard_board},
        pgn::move2pgn,
        piece::{Action, Color},
        pos::Pos,
    };

    #[test]
    fn depth_3_fork() {
        let mut board = standard_board();
        board = board.play(Color::White, Pos(6, 7), &vec![Action::Go(Pos(5, 5))]);
        board = board.play(Color::Black, Pos(4, 1), &vec![Action::Go(Pos(4, 3))]);
        board = board.play(Color::White, Pos(4, 6), &vec![Action::Go(Pos(4, 5))]);
        board = board.play(Color::Black, Pos(3, 1), &vec![Action::Go(Pos(3, 3))]);
        board = board.play(Color::White, Pos(5, 7), &vec![Action::Go(Pos(3, 5))]);
        board = board.play(Color::Black, Pos(6, 0), &vec![Action::Go(Pos(5, 2))]);
        board = board.play(Color::White, Pos(7, 6), &vec![Action::Go(Pos(7, 5))]);
        println!("{}\n", board);
        let (pos, actions) = negamax(&board, Color::Black, 3).unwrap();
        board = board.play(Color::Black, pos, &actions);
        println!("{}", board);
        assert!(pos == Pos(4, 3) && actions == vec![Action::Go(Pos(4, 4))]);
    }

    #[test]
    fn take_take_take() {
        let mut board = halved_board();
        board = board.play(Color::White, Pos(1, 7), &vec![Action::Go(Pos(2, 5))]);
        board = board.play(Color::Black, Pos(3, 1), &vec![Action::Go(Pos(3, 3))]);
        board = board.play(Color::White, Pos(4, 6), &vec![Action::Go(Pos(4, 4))]);
        board = board.play(Color::Black, Pos(1, 0), &vec![Action::Go(Pos(2, 2))]);
        println!("{}\n", board);
        let (pos, actions) = negamax(&board, Color::White, 1).unwrap();
        board = board.play(Color::White, pos, &actions);
        println!("{}", board);
        assert!(pos == Pos(4, 4) && actions == vec![Action::Go(Pos(3, 3))]);
    }

    #[test]
    fn wtf() {
        let mut board = halved_board();
        board = board.play(Color::White, Pos(3, 6), &vec![Action::Go(Pos(3, 5))]);
        board = board.play(Color::Black, Pos(1, 1), &vec![Action::Go(Pos(1, 2))]);
        board = board.play(Color::White, Pos(2, 7), &vec![Action::Go(Pos(4, 5))]);
        board = board.play(Color::Black, Pos(3, 1), &vec![Action::Go(Pos(3, 2))]);
        board = board.play(Color::White, Pos(4, 7), &vec![Action::Go(Pos(0, 4))]);
        board = board.play(Color::Black, Pos(4, 0), &vec![Action::Go(Pos(3, 3))]);
        println!("{}\n", board);
        let (pos, actions) = negamax(&board, Color::Black, 2).unwrap();
        board = board.play(Color::Black, pos, &actions);
        println!("{}", board);
    }

    #[test]
    fn color_invariant() {
        let board = halved_board();
        let pgn_moves1 = auto_play(board.clone(), Color::White, 1);
        let board = invert_color(board);
        let pgn_moves2 = auto_play(board, Color::Black, 1);
        assert!(pgn_moves1 == pgn_moves2);
    }
}
