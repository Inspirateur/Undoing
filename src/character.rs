use bevy::prelude::*;
use std::collections::HashMap;
const DEFAULT_CHAR_S: f64 = 0.05;
#[derive(Component)]
pub struct Character {
    pub name: String,
    pub faces: HashMap<String, Handle<Image>>,
    pub voice: Handle<AudioSource>,
}

#[derive(Component)]
pub struct Say {
    face: String,
    text: String,
    i: usize,
    start: f64,
}

impl Say {
    pub fn new(face: String, text: String) -> Self {
        Say {
            face,
            text,
            i: 0,
            start: 0.,
        }
    }
}

#[derive(Component)]
pub struct DialogueText;

fn setup(mut commands: Commands, server: Res<AssetServer>) {
    let font = server.load("fonts/RobotoMono-Regular.ttf");
    let text_style = TextStyle {
        font,
        font_size: 40.0,
        color: Color::rgb(0.9, 0.9, 0.9),
    };
    let text_alignment = TextAlignment {
        vertical: VerticalAlign::Bottom,
        horizontal: HorizontalAlign::Center,
    };
    commands
        .spawn_bundle(Text2dBundle {
            text: Text::with_section("", text_style, text_alignment),
            transform: Transform::from_xyz(0., -300., 0.),
            ..Default::default()
        })
        .insert(DialogueText);
}

fn dialogue(
    mut commands: Commands,
    mut query: Query<(Entity, &Character, &mut Say)>,
    mut query_text: Query<&mut Text, With<DialogueText>>,
    audio: Res<Audio>,
    time: Res<Time>,
) {
    if let Ok((entity, character, mut say)) = query.get_single_mut() {
        if let Ok(mut text) = query_text.get_single_mut() {
            if say.i == 0 {
                say.start = time.seconds_since_startup();
            }
            // compute the new i
            let mut new_i =
                (((time.seconds_since_startup() - say.start) / DEFAULT_CHAR_S) as usize).max(1);
            // if we finished
            if say.i == say.text.len() {
                // and 1 sec has passed
                if (new_i - say.i) as f64 * DEFAULT_CHAR_S > 1. {
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

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup).add_system(dialogue);
    }
}
