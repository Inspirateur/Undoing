mod ai;
mod board;
mod character;
mod choss;
mod game;
mod make_board;
mod piece;
mod pos;
mod utils;
use bevy::prelude::*;
use game::Undoing;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.15, 0.15, 0.2)))
        .insert_resource(WindowDescriptor {
            width: 720.,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(Undoing)
        .run();
}
