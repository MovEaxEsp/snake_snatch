// Component managing the state of the game:
// whether we're playing, where the coins are, etc
// For the host, the manager gets a dedicated stream to each player, to player's state
// For the client, it gets a stream to the host

use crate::BaseGame;
use crate::network::{NetworkHandle, NetUpdate, StreamHandle};
use crate::painter::TextConfig;
use crate::players::{ClientPlayerManager, HostPlayerManager, PlayerManagerConfig};
use crate::utils::log;
use crate::widgets::{Button, ButtonConfig, ButtonThinkResult};

use engine_p::interpolable::Pos2d;
use serde::{Serialize, Deserialize};

use std::collections::HashMap;

// Network messages

// .. sent from host to clients
#[derive(Debug, Deserialize, Serialize)]
struct PlaceCoins {
    coins: Vec<Pos2d>,
}

// .. sent from clients to host

// When in lobby, inform the host of the client's ready status change
#[derive(Debug, Deserialize, Serialize)]
struct UpdateReadyState {
    is_ready: bool
}

/// Config types
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameManagerConfig {
    pub player_mgr: PlayerManagerConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MainMenuUiConfig {
    pub host_button: ButtonConfig,
    pub join_button: ButtonConfig,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct GameManagerUiConfig {
    pub player_names: TextConfig,
    pub main_menu: MainMenuUiConfig,
}

// MainMenu
// Main menu handler, before hosting or joining a game
pub struct MainMenuManager {
    host_button: Button,
    join_button: Button,
}

pub enum MainMenuManagerThinkResult {
    HostGame,
    JoinGame,
}

impl MainMenuManager {
    fn new() -> Self {
        MainMenuManager {
            host_button: Button::new(),
            join_button: Button::new(),
        }
    }

    fn think(&mut self, game: &dyn BaseGame, ui_cfg: &MainMenuUiConfig) -> Option<MainMenuManagerThinkResult> {
        for res in self.host_button.think(game.mouse(), &ui_cfg.host_button) {
            match res {
                ButtonThinkResult::Clicked => return Some(MainMenuManagerThinkResult::HostGame),
            }
        }

        for res in self.join_button.think(game.mouse(), &ui_cfg.join_button) {
            match res {
                ButtonThinkResult::Clicked => return Some(MainMenuManagerThinkResult::JoinGame),
            }
        }

        None
    }

    fn draw(&self, game: &dyn BaseGame, ui_cfg: &MainMenuUiConfig) {
        self.host_button.draw(game.mouse(), game.painter(), &ui_cfg.host_button);
        self.join_button.draw(game.mouse(), game.painter(), &ui_cfg.join_button);
    }
}

// Enums
enum GameState {
    // Waiting for people to join.  Everyone sees a menu, and a button to vote to start
    Lobby,

    // Everyone gets a chance to decide where to place their snake
    _PlacingSnakes,

    // Everyone is playing
    _Playing,
}

// HostGameManager
pub struct HostGameManager {
    _state: GameState,
    listen_handle: NetworkHandle,
    _coins: Vec<Pos2d>,
    players: HostPlayerManager,

    // Map from connection to our GameStream for it
    _streams: HashMap<NetworkHandle, StreamHandle>,
}

impl HostGameManager {
    fn new(game: &mut dyn BaseGame, config: &GameManagerConfig) -> Self {
        HostGameManager {
            _state: GameState::Lobby,
            listen_handle: game.network().listen("moveaxesp-snake-snatch-game"),
            _coins: Vec::new(),
            players: HostPlayerManager::new("GameHost", &config.player_mgr),
            _streams: HashMap::new(),
        }
    }

    fn think(&mut self, game: &mut dyn BaseGame, config: &GameManagerConfig, _ui_cfg: &GameManagerUiConfig) {
        for msg in game.network().get_handle_events(self.listen_handle).into_iter() {
            if let NetUpdate::NewPeer(new_corr) = msg {
                // Inform any connecting peers about possible start points
                let client_handle = NetworkHandle::from_correlator(new_corr);
                self.players.add_client(client_handle);
            }
        }

        self.players.think(game, &config.player_mgr);
    }

    fn draw(&self, game: &dyn BaseGame, _ui_cfg: &GameManagerUiConfig) {
        self.players.draw(game);
    }
}

// ClientGameManager
pub struct ClientGameManager {
    _state: GameState,
    _coins: Vec<Pos2d>,
    players: Option<ClientPlayerManager>,
    host_handle: NetworkHandle,
    _host_stream: Option<StreamHandle>,
}

impl ClientGameManager {
    fn new(game:&mut dyn BaseGame) -> Self {
        ClientGameManager {
            _state: GameState::Lobby,
            _coins: Vec::new(),
            players: None,
            host_handle: game.network().connect("moveaxesp-snake-snatch-game"),
            _host_stream: None
        }
    }

    fn think(&mut self, game: &mut dyn BaseGame, config: &GameManagerConfig) {
        for outer in game.network().get_handle_events(self.host_handle).into_iter() {
            match outer {
                NetUpdate::NewPeer(_) => {
                    log(&format!("Successfully connected to host with handle {}", self.host_handle));
                    self.players = Some(ClientPlayerManager::new("GameClient", self.host_handle, game));
                }
                _ => {
                    log(&format!("Connect failed/closed: {:?}", outer));
                }
            }
        }

        if let Some(mgr) = &mut self.players {
            mgr.think(game, &config.player_mgr);
        }
    }

    fn draw(&self, game: &dyn BaseGame) {
        if let Some(mgr) = &self.players {
            mgr.draw(game);
        }
    }
}

pub enum GameManager {
    MainMenu(MainMenuManager),
    Host(HostGameManager),
    Client(ClientGameManager),
}

impl GameManager {
    pub fn new() -> Self {
        GameManager::MainMenu(MainMenuManager::new())
    }

    pub fn think(&mut self, game: &mut dyn BaseGame, config: &GameManagerConfig, ui_cfg: &GameManagerUiConfig) {
        match self {
            Self::MainMenu(mgr) => {
                if let Some(res) = mgr.think(game, &ui_cfg.main_menu) {
                    match res {
                        MainMenuManagerThinkResult::HostGame => *self = GameManager::Host(HostGameManager::new(game, config)),
                        MainMenuManagerThinkResult::JoinGame => *self = GameManager::Client(ClientGameManager::new(game)),
                    }
                }
            },
            Self::Host(mgr) => mgr.think(game, config, ui_cfg),
            Self::Client(mgr) => mgr.think(game, config),
        }
    }

    pub fn draw(&self, game: &dyn BaseGame, ui_cfg: &GameManagerUiConfig) {
        match self {
            Self::MainMenu(mgr) => mgr.draw(game, &ui_cfg.main_menu),
            Self::Host(mgr) => mgr.draw(game, ui_cfg),
            Self::Client(mgr) => mgr.draw(game),
        }
    }
}
