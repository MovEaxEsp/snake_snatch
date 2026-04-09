mod mouse;
mod network;
mod painter;
mod players;
mod snake;
mod traits;
mod utils;

use engine_p::interpolable::{Pos2d};
use mouse::MouseManager;
use network::{NetworkHandle, NetworkManager, NetUpdate};
use painter::{Painter, TextConfig};
use players::{NewPlayerMsg, PlayerManager, PlayerManagerConfig, PlayersMsg};
use serde::{Serialize,Deserialize};
use snake::{SnakeConfig};
use traits::{BaseGame, NetMsg};
use utils::{log, set_panic_hook};
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, OffscreenCanvas, OffscreenCanvasRenderingContext2d};
use web_time::Instant;

use std::cell::RefCell;

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
    pub player_manager: PlayerManagerConfig,
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
    mouse: MouseManager,
    elapsed_time: f64,  // seconds since previous frame start (for calculating current frame)
    now: f64,
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

    fn mouse(&self) -> &MouseManager {
        &self.mouse
    }

    fn now(&self) -> f64 {
        self.now
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
    game_start_instant: Instant,
    frame_times: Vec<(Instant, Instant)>, // for measuring elapsed_time, fps
    fps_str: String,
    imp: GameImp,
    listen_handle: Option<NetworkHandle>,
    connect_handle: Option<NetworkHandle>,
    player_manager: PlayerManager,
}

impl GameState {
    fn think(&mut self) {
        // Update frame time and FPS status
        let prev_frame = &self.frame_times[self.frame_times.len() - 2];
        let cur_frame = self.frame_times.last().unwrap();
        self.imp.elapsed_time = (cur_frame.0 - prev_frame.0).as_secs_f64();
        self.imp.now = (cur_frame.0 - self.game_start_instant).as_secs_f64();

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
        if let Some(hndl) = self.listen_handle {
            for msg in self.imp.network().get_handle_events(hndl).into_iter() {
                log(&format!("Listen event: {:?}", msg));
                if let NetUpdate::NewPeer(new_corr) = msg {
                    // Inform any connecting peers about possible start points
                    let client_handle = NetworkHandle::from_correlator(new_corr);
                    self.player_manager.add_client(client_handle);
                }
            }
        }
        if let Some(hndl) = self.connect_handle {
            for outer in self.imp.network().get_handle_events(hndl).into_iter() {
                match outer {
                    NetUpdate::NewPeer(_) => {
                        let players_stream = self.imp.network.new_stream(hndl).unwrap();
                        let player_stream = self.imp.network.new_stream(hndl).unwrap();

                        self.imp.network.send(&hndl.default_stream(), NetMsg::NewClient(traits::NewClientMsg {
                            players_stream: players_stream.stream_id(),
                        }));

                        self.imp.network.send(&players_stream, NetMsg::Players(PlayersMsg::NewPlayer(NewPlayerMsg {
                            name: "GameClient".to_string(),
                            player_stream: player_stream.stream_id(),
                        })));

                        log(&format!("Successfully connected to host with handle {}", hndl));
                        self.player_manager = PlayerManager::new_client("GameClient", &players_stream, &player_stream);
                    }
                    _ => {
                        log(&format!("Connect failed/closed: {:?}", outer));
                    }
                }
            }
        }

        let config = self.imp.config.clone();

        self.imp.think();
        self.player_manager.think(&mut self.imp, &config.game.player_manager);
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

        self.player_manager.draw(&self.imp);

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

    fn be_host(&mut self) {
        self.player_manager = PlayerManager::new_host("GameHost", &self.imp.config.game.player_manager);
        self.listen_handle = Some(self.imp.network().listen("moveaxesp-snake-snatch-game"));
    }

    fn be_client(&mut self) {
        self.connect_handle = Some(self.imp.network().connect("moveaxesp-snake-snatch-game"));
    }

    fn ping_connections(&mut self) {
    }

}

static mut S_STATES: RefCell<Vec<GameState>> = RefCell::new(Vec::new());

#[wasm_bindgen]
pub fn init_state(config: JsValue, canvas: JsValue, _images: JsValue, _audio_ctx: JsValue, _sounds: JsValue) -> usize {
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
        mouse: MouseManager::new(screen_canvas.clone(), 2560.0, 1440.0),
        now: 0.0,
    };

    let mut state = GameState{
        screen_canvas: screen_canvas,
        offscreen_canvas: offscreen_canvas,
        frame_times: Vec::new(),
        game_start_instant: Instant::now(),
        imp: game_imp,
        fps_str: "".to_string(),
        listen_handle: None,
        connect_handle: None,
        player_manager: PlayerManager::Unset,
    };

    state.frame_times.push((Instant::now(), Instant::now()));

    unsafe {
        #[allow(static_mut_refs)]
        let states = S_STATES.get_mut();
        states.push(state);
        states.len()-1
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
        for state in &mut *S_STATES.borrow_mut() {
            run_frame_imp(state);
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
            player_manager: PlayerManagerConfig {
                snake_start_points: vec![
                    (200, 200).into(), (600, 200).into(), (200, 600).into(), (600, 600).into()
                ],
                snake: SnakeConfig {
                    grow_speed: 100.0,
                },
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
                for state in &mut *S_STATES.borrow_mut() {
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
pub fn be_host(state_idx: usize) {
    unsafe {
        #[allow(static_mut_refs)]
        S_STATES.borrow_mut()[state_idx].be_host();
    }
}

#[wasm_bindgen]
pub fn be_client(state_idx: usize) {
    unsafe {
        #[allow(static_mut_refs)]
        S_STATES.borrow_mut()[state_idx].be_client();
    }
}

#[wasm_bindgen]
pub fn ping_connections() {
    unsafe {
        #[allow(static_mut_refs)]
        for state in &mut *S_STATES.borrow_mut() {
            state.ping_connections();
        }
    }
}
