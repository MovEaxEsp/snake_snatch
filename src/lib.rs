mod images;
mod painter;
mod sounds;
mod traits;
mod utils;

use engine_p::interpolable::{Interpolable, Pos2d};
use images::{Images, ImagesConfig};
use painter::{Painter, TextConfig};
use serde::{Serialize,Deserialize};
use sounds::{Sounds, SoundsConfig};
use traits::BaseGame;
use utils::set_panic_hook;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, OffscreenCanvas, OffscreenCanvasRenderingContext2d};
use web_time::Instant;

use std::cell::RefCell;
use std::rc::Rc;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[derive(Deserialize, PartialEq, Debug)]
pub enum MouseEventType {
    Down,
    Up,
    Move
}

#[derive(Deserialize, Debug)]
pub struct MouseEvent {
    event_type: MouseEventType,   
    pos: Pos2d,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UiConfig {
    pub images: ImagesConfig,
    pub sounds: SoundsConfig,
    pub fps: TextConfig,
    pub arena_color: String,
    pub arena_pos: Pos2d,
    pub arena_width: f64,
    pub arena_height: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GameConfig {
    pub snake_grow_speed: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct OuterConfig {
    pub ui: UiConfig,
    pub game: GameConfig,
}

///////// GameState
struct GameImp {
    cur_money: RefCell<i32>,
    painter: Painter,
    sounds: Sounds,
    config: OuterConfig,
    elapsed_time: f64,  // seconds since previous frame start (for calculating current frame)
}

impl BaseGame for GameImp {
    fn get_money(&self) -> i32 {
        *self.cur_money.borrow()
    }

    fn painter<'a>(&'a self) -> &'a Painter {
        &self.painter
    }

    fn sounds(&self) -> &Sounds {
        &self.sounds
    }

    fn elapsed_time(&self) -> f64 {
        self.elapsed_time
    }
}

impl GameImp {
    fn think(&mut self) {
        self.painter.think(self.elapsed_time);
    }
}

struct GameState {
    screen_canvas: HtmlCanvasElement,
    offscreen_canvas: OffscreenCanvas,
    got_first_input: bool,
    frame_times: Vec<(Instant, Instant)>, // for measuring elapsed_time, fps
    fps_str: String,
    imp: GameImp,
    snake_points: Vec<Pos2d>,
    is_mouse_down: bool,
    mouse_pos: Pos2d,
}

impl GameState {
    fn think(&mut self) {

        // Update frame time and FPS status
        let prev_frame = &self.frame_times[self.frame_times.len() - 2];
        let cur_frame = self.frame_times.last().unwrap();
        self.imp.elapsed_time = (cur_frame.0 - prev_frame.0).as_secs_f64();

        let frames_per_update = 10;
        if self.frame_times.len() > frames_per_update + 2 {
            // Update the FPS occasionally
            let fps_frames: Vec<(Instant, Instant)> = self.frame_times.drain(..frames_per_update).collect();
            let processing_time: f64 = fps_frames.iter().map(|v|(v.1-v.0).as_secs_f64()).sum();

            let elapsed_time = (fps_frames.last().unwrap().1 - fps_frames[0].0).as_secs_f64();
            let fps = frames_per_update as f64/elapsed_time;
            let processing_pct = (processing_time/elapsed_time) * 100.0;
            self.fps_str = format!("{:.2} FPS ({:2.2} %)", fps, processing_pct);
        }

        self.imp.think();
        
        // Update the size of our snake depending on if mouse is down or up
        let snake_intr = Interpolable::new(*self.snake_points.last().unwrap(), self.imp.config.game.snake_grow_speed);
        if self.is_mouse_down && self.mouse_pos != *self.snake_points.last().unwrap() {
            // Grow the snake towards the mouse
            snake_intr.set_end(self.mouse_pos);
            snake_intr.advance(self.imp.elapsed_time);
            *self.snake_points.last_mut().unwrap() = snake_intr.cur();
        }
        else if !self.is_mouse_down && self.snake_points.len() > 2 {
            // Shrink the snake while the mouse is up
            let segment_start = self.snake_points[self.snake_points.len()-2];
            snake_intr.set_end(segment_start);
            snake_intr.advance(self.imp.elapsed_time);
            let cur = snake_intr.cur();
            if cur == segment_start {
                self.snake_points.pop();
            }
            else {
                *self.snake_points.last_mut().unwrap() = cur;
            }
        }
    }

    fn draw(&self) {
        let canvas = self.imp.painter().canvas();
        canvas.set_fill_style_str("DimGrey");
        canvas.clear_rect(0.0, 0.0, 2560.0, 1440.0);
        canvas.fill_rect(0.0, 0.0, 2560.0, 1440.0);
        
        let cfg = &self.imp.config.ui;
        
        // Draw the game area
        canvas.set_fill_style_str(&cfg.arena_color);
        canvas.fill_rect(cfg.arena_pos.x, cfg.arena_pos.y, cfg.arena_width, cfg.arena_height);
        
        // Draw the snake
        canvas.set_stroke_style_str("black");
        canvas.set_line_width(10.0);
        canvas.move_to(self.snake_points[0].x, self.snake_points[0].y);
        for pos in self.snake_points[1..].iter() {
            canvas.line_to(pos.x, pos.y);
            canvas.stroke();
            canvas.begin_path();
            canvas.move_to(pos.x, pos.y);
        }
        
        // Draw FPS
        self.imp.painter().draw_text(&self.fps_str, &(2000, 10).into(), 300.0, &self.imp.config.ui.fps);

        let screen_context = self.screen_canvas
        .get_context("2d").unwrap().unwrap()
        .dyn_into::<CanvasRenderingContext2d>().unwrap();

        screen_context.clear_rect(0.0, 0.0, self.screen_canvas.width() as f64, self.screen_canvas.height() as f64);

        screen_context.draw_image_with_offscreen_canvas_and_dw_and_dh(
            &self.offscreen_canvas,
            0.0, 0.0,
            self.screen_canvas.width() as f64, self.screen_canvas.height() as f64)
        .expect("draw offscreen canvas");
    }

    fn update_config(&mut self, cfg: &OuterConfig) {
        self.imp.config = cfg.clone();
        self.imp.painter.update_config(&cfg.ui.images);
    }
    
    fn handle_mouse_event(&mut self, mut evt: MouseEvent) {
        self.got_first_input = true;
        
        // Adjust event x and y for offscreen canvas coordinates
        let width_factor = self.offscreen_canvas.width() as f64 / self.screen_canvas.width() as f64;
        let height_factor = self.offscreen_canvas.height() as f64 / self.screen_canvas.height() as f64;
        
        evt.pos.x *= width_factor;
        evt.pos.y *= height_factor;
        
        if evt.event_type == MouseEventType::Up {
            self.is_mouse_down = false;
        }
        else if evt.event_type == MouseEventType::Down {
            self.is_mouse_down = true;
        }
        
        self.mouse_pos = evt.pos;
        
        // On any mouse move while the mouse is down, start a new segment from wherever the last segment is,
        // if the segment is long enough
        if self.is_mouse_down {
            if self.snake_points.last().unwrap().dist(self.snake_points[self.snake_points.len()-2]) > 20.0 {
                self.snake_points.push(*self.snake_points.last().unwrap());
            }
        }

        log(&format!("Snake parts: {:?}", self.snake_points));

    }

}

static mut S_STATE: Option<Rc<RefCell<GameState>>> = None;

#[wasm_bindgen]
pub fn init_state(config: JsValue, canvas: JsValue, images: JsValue, audio_ctx: JsValue, sounds: JsValue) {
    set_panic_hook();
    
    let game_config: OuterConfig = serde_wasm_bindgen::from_value(config).unwrap();

    let offscreen_canvas = OffscreenCanvas::new(2560, 1440).expect("offscreen canvas");
    let offscreen_context = offscreen_canvas.get_context("2d").unwrap().unwrap()
                        .dyn_into::<OffscreenCanvasRenderingContext2d>().unwrap();

    let screen_canvas= canvas.dyn_into::<HtmlCanvasElement>().expect("canvas");

    let painter_images = Images::new(images, &game_config.ui.images);

    let painter = Painter::new(painter_images, offscreen_context);

    let sounds = Sounds::new(audio_ctx, sounds, &game_config.ui.sounds);

    let game_imp = GameImp {
        cur_money: RefCell::new(0),
        painter: painter,
        sounds: sounds,
        config: game_config,
        elapsed_time: 0.0,
    };

    let mut state = GameState{
        screen_canvas: screen_canvas,
        offscreen_canvas: offscreen_canvas,
        got_first_input: false,
        frame_times: Vec::new(),
        imp: game_imp,
        fps_str: "".to_string(),
        snake_points: vec![(200, 200).into(), (300, 300).into()],
        is_mouse_down: false,
        mouse_pos: (0, 0).into(),
    };

    state.frame_times.push((Instant::now(), Instant::now()));

    unsafe {
        S_STATE = Some(Rc::new(RefCell::new(state)));
    }
    
}

fn run_frame_imp(state_rc: &Rc<RefCell<GameState>>) {
    let mut state = state_rc.borrow_mut();

    let now = Instant::now();
    state.frame_times.push((now, now));

    state.think();
    state.draw();

    state.frame_times.last_mut().unwrap().1 = Instant::now();
}

#[wasm_bindgen]
pub fn run_frame() {
    unsafe {
        #[allow(static_mut_refs)]
        let state: &Rc<RefCell<GameState>> = S_STATE.as_mut().unwrap();
        run_frame_imp(state);
    }
}

#[wasm_bindgen]
pub fn handle_mouse_event(event: JsValue) {
    match serde_wasm_bindgen::from_value::<MouseEvent>(event) {
        Ok(evt) => {
            unsafe {
                #[allow(static_mut_refs)]
                let state: &Rc<RefCell<GameState>> = S_STATE.as_mut().unwrap();
                state.borrow_mut().handle_mouse_event(evt);
            }
        }
        Err(e) => {
            log(&format!("Failed parsing mouse event: {}", e));
        }
    }
}

pub fn build_default_config() -> OuterConfig {
    OuterConfig {
        ui: UiConfig {
            images: Images::default_config(),
            sounds: Sounds::default_config(),
            fps: TextConfig {
                offset: (0, 0).into(),
                stroke: false,
                style: "black".to_string(),
                font: "comic sans".to_string(),
                size: 30,
                center_and_fit: false,
                alpha: 0.7,
                is_command: false,
            },
            arena_color: "pink".to_string(),
            arena_pos: (200,200).into(),
            arena_width: 1000.0,
            arena_height: 1000.0,
        },
        game: GameConfig {
            snake_grow_speed: 100.0,
        }
    }
}

#[wasm_bindgen]
pub fn default_config() -> JsValue {
    serde_wasm_bindgen::to_value(&build_default_config()).unwrap()
}

#[wasm_bindgen]
pub fn update_config(config: JsValue) {
    match serde_wasm_bindgen::from_value::<OuterConfig>(config) {
        Ok(cfg) => {
            unsafe {
                #[allow(static_mut_refs)]
                let state: &Rc<RefCell<GameState>> = S_STATE.as_mut().unwrap();
                state.borrow_mut().update_config(&cfg);
            }
        }
        Err(e) => {
            log(&format!("Failed parsing config: {}", e));
        }
    }
}

#[wasm_bindgen]
pub fn resource_names() -> JsValue {
    #[derive(Serialize)]
    pub struct ResourceList {
        pub images: Vec<String>,
        pub sounds: Vec<String>,
    }

    let cfg = build_default_config();

    let resources = ResourceList {
        images: cfg.ui.images.images.iter().map(|img| img.image_name.clone()).collect(),
        sounds: cfg.ui.sounds.sounds.iter().flat_map(|snd| snd.sound_names.iter().cloned()).collect(),
    };

    serde_wasm_bindgen::to_value(&resources).unwrap()
}