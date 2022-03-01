use std::ops::{Add, Mul};
pub const LINES: [Pos; 4] = [Pos(0, 1), Pos(0, -1), Pos(1, 0), Pos(-1, 0)];
pub const DIAGS: [Pos; 4] = [Pos(1, 1), Pos(1, -1), Pos(-1, 1), Pos(-1, -1)];
pub const LOS: [Pos; 8] = [
    Pos(0, 1),
    Pos(0, -1),
    Pos(1, 0),
    Pos(-1, 0),
    Pos(1, 1),
    Pos(1, -1),
    Pos(-1, 1),
    Pos(-1, -1),
];

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Pos(pub i32, pub i32);

impl Add for Pos {
    type Output = Pos;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl Mul<i32> for Pos {
    type Output = Pos;

    fn mul(self, rhs: i32) -> Self::Output {
        Self(self.0 * rhs, self.1 * rhs)
    }
}

impl Pos {
    pub fn neighbors(&self) -> [Pos; 2] {
        if self.0 == 0 {
            // it's vertical
            [Pos(1, self.1), Pos(-1, self.1)]
        } else if self.1 == 0 {
            // it's horizontal
            [Pos(self.0, 1), Pos(self.0, -1)]
        } else {
            // it's a diag
            [Pos(self.0, 0), Pos(0, self.1)]
        }
    }
}
