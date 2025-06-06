
use serde::{Serialize,Deserialize};
use wasm_bindgen::prelude::*;
use web_sys::{AudioBuffer, AudioContext};

use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum Sound {
    Coins,
    Frying,
    Done,
}

struct SoundProps {
    bufs: Vec<AudioBuffer>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlaybackConfig {
    pub sound: Sound,
    pub play_length: Option<f64>,
    pub random_start: bool
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SoundConfig {
    pub sound: Sound,
    pub sound_names: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SoundsConfig {
    pub sounds: Vec<SoundConfig>,
}

pub struct Sounds {
    ctx: AudioContext,
    sounds: HashMap<Sound, SoundProps>,
}

impl Sounds {
    pub fn new(js_ctx: JsValue, js_sounds: JsValue, cfg: &SoundsConfig) -> Self {
        let ctx = js_ctx.dyn_into::<AudioContext>().expect("audioctx");

        let mut self_sounds: HashMap<Sound, SoundProps> = HashMap::new();

        for snd_cfg in cfg.sounds.iter() {
            let variations: Vec<AudioBuffer> =
                snd_cfg.sound_names.iter()
                .map(|name| 
                    js_sys::Reflect::get(&js_sounds, &name.into()).expect("snd")
                    .dyn_into::<AudioBuffer>().expect("audiobuf"))
                .collect();

            self_sounds.insert(
                snd_cfg.sound.clone(),
                SoundProps {
                    bufs: variations,
                });
        }

        Sounds {
            ctx: ctx,
            sounds: self_sounds,
        }
    }

    pub fn handle_first_input(&self) {
        // HACK: play a sound quietly the first time the user types anything to avoid a lag the first
        // time a sound is played
        let src = self.ctx.create_buffer_source().expect("buf src");
        let buf = &self.sounds[&Sound::Coins].bufs[0];
        src.set_buffer(Some(buf));
        src.start().expect("start");
    }

    pub fn play_sound(&self, cfg: &PlaybackConfig) {
        let props = &self.sounds[&cfg.sound];

        // Pick random buffer to play from the sound's buffers
        let idx = (js_sys::Math::random() * props.bufs.len() as f64) as usize;
        let buf = &props.bufs[idx];

        let src = self.ctx.create_buffer_source().expect("buf src");
        src.set_buffer(Some(buf));
        src.connect_with_audio_node(&self.ctx.destination()).expect("connect");

        let mut snd_duration = buf.duration();
        let mut snd_offset = 0.0;
        if let Some(dur) = &cfg.play_length {
            snd_duration = *dur;
            if cfg.random_start {
                snd_offset = (buf.duration() - snd_duration) * js_sys::Math::random();
            }
        }

        src.start_with_when_and_grain_offset_and_grain_duration(0.0, snd_offset, snd_duration).expect("play snd");
    }

    pub fn default_config() -> SoundsConfig {
        fn snd(sound: Sound, sound_names: Vec<&str>) -> SoundConfig {
            SoundConfig {
                sound: sound,
                sound_names: sound_names.iter().map(|name| name.to_string()).collect(),
            }
        }

        SoundsConfig {
            sounds: vec![
            ]
        }
    }
}