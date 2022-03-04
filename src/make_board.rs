use crate::board::Board;
use crate::piece::{Color, PawnStatus, Piece};
use crate::pos::Pos;

fn from_backrank(pieces: Vec<Piece>) -> Board {
    let mut board = Board::new(pieces.len(), 8);

    for (i, piece) in pieces
        .iter()
        .chain(
            vec![
                Piece::Pawn {
                    orientation: Pos(0, 1),
                    status: PawnStatus::CanLeap,
                };
                pieces.len()
            ]
            .iter(),
        )
        .enumerate()
    {
        board.squares[i] = Some((Color::Black, *piece));
    }
    let len_squares = board.squares.len();
    for (i, piece) in pieces
        .iter()
        .rev()
        .chain(
            vec![
                Piece::Pawn {
                    orientation: Pos(0, -1),
                    status: PawnStatus::CanLeap,
                };
                pieces.len()
            ]
            .iter(),
        )
        .enumerate()
    {
        board.squares[len_squares - i - 1] = Some((Color::White, *piece));
    }
    board
}

pub fn standard_board() -> Board {
    from_backrank(vec![
        Piece::Rook,
        Piece::Knight,
        Piece::Bishop,
        Piece::Queen,
        Piece::King,
        Piece::Bishop,
        Piece::Knight,
        Piece::Rook,
    ])
}

pub fn halved_board() -> Board {
    from_backrank(vec![
        Piece::Rook,
        Piece::Knight,
        Piece::Bishop,
        Piece::King,
        Piece::Queen,
    ])
}
