#![recursion_limit = "1024"]
mod ai;
mod board;
mod game;
mod make_board;
mod pgn;
mod piece;
mod pos;
use bevy::prelude::*;
use game::Undoing;

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            width: 720.,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(Undoing)
        .run();
}
