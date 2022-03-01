use crate::piece::{Action, Piece};
use crate::pos::Pos;

fn piece2pgn(piece: Piece) -> &'static str {
    match piece {
        Piece::Pawn {
            orientation: _,
            status: _,
        } => "p",
        Piece::Knight => "N",
        Piece::Bishop => "B",
        Piece::Rook => "R",
        Piece::Queen => "Q",
        Piece::King => "K",
    }
}

fn pos2pgn(pos: Pos) -> String {
    let letters = ["a", "b", "c", "d", "e", "f", "g", "h"];
    format!("{}{}", letters[pos.0 as usize], pos.1)
}

pub fn move2pgn(pos: Pos, actions: &Vec<Action>) -> String {
    let mut res = String::new();
    for action in actions {
        if let Action::Go(go_pos) = action {
            res += format!("{}{}", pos2pgn(pos), pos2pgn(*go_pos)).as_str();
        } else if let Action::Promotion(piece) = action {
            res += format!("={}", piece2pgn(*piece)).as_str();
        }
    }
    res
}
