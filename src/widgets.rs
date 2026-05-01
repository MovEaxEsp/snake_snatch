use crate::MouseManager;
use crate::painter::{BackgroundConfig, Painter, TextConfig};

use serde::{Serialize,Deserialize};

// Button

#[derive(Serialize, Deserialize, Clone)]
pub struct ButtonConfig {
    pub bg_normal: BackgroundConfig,
    pub bg_pressed: BackgroundConfig,
    pub bg_disabled: BackgroundConfig,
    pub text_cfg: TextConfig,
    pub text: String,
}

pub struct Button {
    pub enabled: bool,
}

pub enum ButtonThinkResult {
    Clicked
}

impl Button {
    pub fn new() -> Self {
        Button {
            enabled: true
        }
    }

    pub fn think(&mut self, mouse: &MouseManager, config: &ButtonConfig) -> Vec<ButtonThinkResult> {
        if mouse.is_click_in_rect(&config.bg_normal.offset, config.bg_normal.width, config.bg_normal.height) {
            // This button was clicked
            return Vec::from([ButtonThinkResult::Clicked]);
        }

        Vec::new()
    }

    pub fn draw(&self, mouse: &MouseManager, painter: &Painter, config: &ButtonConfig) {
        let bg;
        if !self.enabled {
            bg = &config.bg_disabled;
        }
        else if mouse.is_down_in_rect(&config.bg_normal.offset, config.bg_normal.width, config.bg_normal.height) {
            bg = &config.bg_pressed;
        }
        else {
            bg = &config.bg_normal;
        }

        painter.draw_area_background(&(0,0).into(), bg);
        painter.draw_text(&config.text, &bg.offset, bg.width, &config.text_cfg);
    }
}
