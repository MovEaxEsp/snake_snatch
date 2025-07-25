mod network;
mod painter;
mod snake;
mod traits;
mod utils;

use engine_p::interpolable::{Pos2d};
use network::{NetworkHandle, NetworkManager, NetUpdate};
use painter::{Painter, TextConfig};
use serde::{Serialize,Deserialize};
use snake::{Snake, SnakeConfig};
use traits::{BaseGame, NetMsg};
use utils::set_panic_hook;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, OffscreenCanvas, OffscreenCanvasRenderingContext2d};
use web_time::Instant;

use std::cell::RefCell;

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
    pub fps: TextConfig,
    pub arena_color: String,
    pub arena_pos: Pos2d,
    pub arena_width: f64,
    pub arena_height: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GameConfig {
    pub snake: SnakeConfig,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct OuterConfig {
    pub ui: UiConfig,
    pub game: GameConfig,
}

///////// GameState
struct GameImp {
    painter: Painter,
    network: NetworkManager<NetMsg>,
    config: OuterConfig,
    is_mouse_down: bool,
    mouse_pos: Pos2d,
    elapsed_time: f64,  // seconds since previous frame start (for calculating current frame)
}

impl BaseGame for GameImp {
    fn painter<'a>(&'a self) -> &'a Painter {
        &self.painter
    }
    
    fn network(&mut self) -> &mut NetworkManager<NetMsg> {
        &mut self.network
    }

    fn elapsed_time(&self) -> f64 {
        self.elapsed_time
    }
    
    fn mouse_pos(&self) -> Pos2d {
        self.mouse_pos
    }
    
    fn is_mouse_down(&self) -> bool {
        self.is_mouse_down
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
    snake: Snake,
    listen_handle: NetworkHandle,
    client_handle: NetworkHandle,
    connect_handle: NetworkHandle,
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
        
        // Check for network messages
        for msg in self.imp.network().get_handle_events(self.listen_handle).into_iter() {
            log(&format!("Listen event: {:?}", msg));
            if let NetUpdate::NewPeer(new_corr) = msg {
                self.client_handle = NetworkHandle::from_correlator(new_corr);
            }
        }
        for msg in self.imp.network().get_handle_events(self.connect_handle).into_iter() {
            log(&format!("Connect event: {:?}", msg));
        }
        for msg in self.imp.network().get_handle_events(self.client_handle).into_iter() {
            log(&format!("Client event: {:?}", msg));
        }

        self.imp.think();
        
        self.snake.think(&self.imp, &self.imp.config.game.snake);
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
        
        self.snake.draw(&self.imp);
        
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
        //self.imp.painter.update_config(&cfg.ui.images);
    }
    
    fn handle_mouse_event(&mut self, mut evt: MouseEvent) {
        self.got_first_input = true;
        
        // Adjust event x and y for offscreen canvas coordinates
        let width_factor = self.offscreen_canvas.width() as f64 / self.screen_canvas.width() as f64;
        let height_factor = self.offscreen_canvas.height() as f64 / self.screen_canvas.height() as f64;
        
        evt.pos.x *= width_factor;
        evt.pos.y *= height_factor;
        
        if evt.event_type == MouseEventType::Up {
            self.imp.is_mouse_down = false;
            if self.connect_handle != NetworkHandle::invalid() {
                self.imp.network().send(self.connect_handle, NetMsg::SnakePos((evt.pos.x as i32, evt.pos.y as i32)));
            }
            if self.client_handle != NetworkHandle::invalid() {
                self.imp.network().send(self.client_handle, NetMsg::SnakePos((evt.pos.x as i32, evt.pos.y as i32)));
            }
        }
        else if evt.event_type == MouseEventType::Down {
            self.imp.is_mouse_down = true;
        }
        
        self.imp.mouse_pos = evt.pos;
    }
    
    fn be_host(&mut self) {
        self.listen_handle = self.imp.network().listen("moveaxesp-snake-snatch-game");
    }
    
    fn be_client(&mut self) {
        self.connect_handle = self.imp.network().connect("moveaxesp-snake-snatch-game");        
    }

}

static mut S_STATE: RefCell<Option<GameState>> = RefCell::new(None);

#[wasm_bindgen]
pub fn init_state(config: JsValue, canvas: JsValue, _images: JsValue, _audio_ctx: JsValue, _sounds: JsValue) {
    set_panic_hook();
    
    let game_config: OuterConfig = serde_wasm_bindgen::from_value(config).unwrap();

    let offscreen_canvas = OffscreenCanvas::new(2560, 1440).expect("offscreen canvas");
    let offscreen_context = offscreen_canvas.get_context("2d").unwrap().unwrap()
                        .dyn_into::<OffscreenCanvasRenderingContext2d>().unwrap();

    let screen_canvas= canvas.dyn_into::<HtmlCanvasElement>().expect("canvas");

    let painter = Painter::new(offscreen_context);

    let game_imp = GameImp {
        painter: painter,
        network: NetworkManager::new(),
        config: game_config,
        elapsed_time: 0.0,
        is_mouse_down: false,
        mouse_pos: (0,0).into(),
    };

    let mut state = GameState{
        screen_canvas: screen_canvas,
        offscreen_canvas: offscreen_canvas,
        got_first_input: false,
        frame_times: Vec::new(),
        imp: game_imp,
        fps_str: "".to_string(),
        listen_handle: NetworkHandle::invalid(),
        connect_handle: NetworkHandle::invalid(),
        client_handle: NetworkHandle::invalid(),
        snake: Snake::new(),
    };

    state.frame_times.push((Instant::now(), Instant::now()));

    unsafe {
        #[allow(static_mut_refs)]
        S_STATE.get_mut().replace(state);
    }
    
}

fn run_frame_imp(state: &mut GameState) {
    let now = Instant::now();
    state.frame_times.push((now, now));
    
    state.think();
    state.draw();
    
    state.frame_times.last_mut().unwrap().1 = Instant::now();
}

#[wasm_bindgen]
pub fn run_frame(){
    unsafe {
        #[allow(static_mut_refs)]
        if let Some(state) = &mut *S_STATE.borrow_mut() {
            run_frame_imp(state);
        }
    }
}

#[wasm_bindgen]
pub fn handle_mouse_event(event: JsValue) {
    match serde_wasm_bindgen::from_value::<MouseEvent>(event) {
        Ok(evt) => {
            unsafe {
                #[allow(static_mut_refs)]
                if let Some(state) = &mut *S_STATE.borrow_mut() {
                    state.handle_mouse_event(evt);
                }
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
            snake: SnakeConfig {
                grow_speed: 100.0,
            }
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
                if let Some(state) = &mut *S_STATE.borrow_mut() {
                    state.update_config(&cfg);
                }
            }
        }
        Err(e) => {
            log(&format!("Failed parsing config: {}", e));
        }
    }
}

#[wasm_bindgen]
pub fn be_host() {
    unsafe {
        #[allow(static_mut_refs)]
        if let Some(state) = &mut *S_STATE.borrow_mut() {
            state.be_host();
        }
    }
}

#[wasm_bindgen]
pub fn be_client() {
    unsafe {
        #[allow(static_mut_refs)]
        if let Some(state) = &mut *S_STATE.borrow_mut() {
            state.be_client();
        }
    }
}