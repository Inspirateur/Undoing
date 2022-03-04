use crate::choss::{ChossGame, HSIZE};
use bevy::prelude::*;
use std::collections::HashMap;
use std::fs;

const CHAR_SEC: f64 = 0.04;
#[derive(Component)]
pub struct Character {
    pub name: String,
    pub faces: HashMap<String, Handle<Image>>,
    pub voice: Handle<AudioSource>,
}

impl Character {
    pub fn new(name: impl ToString, server: &AssetServer) -> Self {
        let name = name.to_string();
        // try to load faces
        let mut faces = HashMap::new();
        if let Ok(paths) = fs::read_dir(format!("./assets/{}/", name.to_lowercase())) {
            for path_res in paths {
                if let Ok(path) = path_res {
                    // we pop the assets folder from the path because the AssetServer starts from there
                    if let Ok(path) = path.path().strip_prefix("./assets") {
                        if let Some(path_str) = path.as_os_str().to_str() {
                            if let Some(filename_os) = path.file_stem() {
                                if let Some(filename) = filename_os.to_str() {
                                    faces.insert(filename.to_string(), server.load(path_str));
                                }
                            }
                        }
                    }
                }
            }
        }
        // try to load voice
        let voice = server.load(&format!(
            "sounds/{}.ogg",
            name.to_lowercase().replace(" ", "_")
        ));
        Character { name, faces, voice }
    }
}

#[derive(Component)]
pub struct Say {
    face: String,
    text: String,
    i: usize,
    start: f64,
    duration: f64,
}

impl Say {
    pub fn new(face: impl ToString, text: impl ToString) -> Self {
        Say {
            face: face.to_string(),
            text: text.to_string(),
            i: 0,
            start: -1.,
            duration: text
                .to_string()
                .chars()
                .map(Say::char_duration)
                .fold(0., |acc, x| acc + x)
                * CHAR_SEC,
        }
    }

    fn char_duration(char: char) -> f64 {
        match char {
            ' ' => 1.5,
            ',' => 3.,
            '.' | '!' | '?' => 6.,
            _ => 1.,
        }
    }

    pub fn compute_i(&self, now: f64) -> usize {
        let delta = now - self.start;
        let mut count = 0.;
        let mut new_i = 0;
        for char in self.text.chars() {
            count += CHAR_SEC * Say::char_duration(char);
            new_i += 1;
            if count > delta {
                break;
            }
        }
        new_i
    }
}

#[derive(Component)]
pub struct DialogueText;

#[derive(Component)]
pub struct DialogueFace;

fn setup(mut commands: Commands, server: Res<AssetServer>, choss: Res<ChossGame>) {
    let font = server.load("fonts/RobotoMono-Regular.ttf");
    let text_style = TextStyle {
        font,
        font_size: 25.0,
        color: Color::rgb(0.9, 0.9, 0.9),
    };
    let text_alignment = TextAlignment {
        vertical: VerticalAlign::Center,
        horizontal: HorizontalAlign::Left,
    };
    commands
        .spawn_bundle(Text2dBundle {
            text: Text::with_section("", text_style, text_alignment),
            transform: Transform::from_xyz(
                -HSIZE * choss.board.width as f32,
                HSIZE * (choss.board.height + 2) as f32,
                0.,
            ),
            ..Default::default()
        })
        .insert(DialogueText);
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(128., 128.)),
                ..Default::default()
            },
            texture: server.load("empty.png"),
            transform: Transform::from_xyz(
                -HSIZE * choss.board.width as f32 - 100.,
                HSIZE * (choss.board.height + 2) as f32,
                0.,
            ),
            ..Default::default()
        })
        .insert(DialogueFace);
}

fn dialogue(
    mut commands: Commands,
    mut query: Query<(Entity, &Character, &mut Say)>,
    mut query_text: Query<&mut Text, With<DialogueText>>,
    mut query_face: Query<&mut Handle<Image>, With<DialogueFace>>,
    audio: Res<Audio>,
    time: Res<Time>,
) {
    if let Ok((entity, character, mut say)) = query.get_single_mut() {
        if let Ok(mut text) = query_text.get_single_mut() {
            if let Ok(mut face) = query_face.get_single_mut() {
                if say.i == 0 {
                    say.start = time.seconds_since_startup();
                    if let Some(face_handle) = character.faces.get(&say.face) {
                        *face = face_handle.clone();
                    }
                }
                // compute the new i
                let now = time.seconds_since_startup();
                let mut new_i = say.compute_i(now);
                // if we finished
                if say.i >= say.text.len() {
                    // and 1 sec has passed
                    if now - say.duration - say.start > 1. {
                        commands.entity(entity).remove::<Say>();
                    }
                } else if new_i != say.i {
                    // there's new characters to say
                    new_i = new_i.min(say.text.len());
                    text.sections[0].value = say.text[0..new_i].to_string();
                    // if i..new_i is not only spaces, produce a sound
                    if say.text[say.i..new_i].trim().len() > 0 {
                        audio.play(character.voice.clone());
                    }
                    say.i = new_i;
                }
            }
        }
    }
}

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup).add_system(dialogue);
    }
}
