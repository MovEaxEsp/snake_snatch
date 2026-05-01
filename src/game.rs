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

#[derive(Clone, Deserialize, Serialize)]
pub struct GameManagerUiConfig {
    pub player_names: TextConfig,
    pub test_button: ButtonConfig,
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

    test_button: Button,

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
            test_button: Button::new(),
        }
    }

    fn think(&mut self, game: &mut dyn BaseGame, config: &GameManagerConfig, ui_cfg: &GameManagerUiConfig) {
        for msg in game.network().get_handle_events(self.listen_handle).into_iter() {
            if let NetUpdate::NewPeer(new_corr) = msg {
                // Inform any connecting peers about possible start points
                let client_handle = NetworkHandle::from_correlator(new_corr);
                self.players.add_client(client_handle);
            }
        }

        self.players.think(game, &config.player_mgr);
        for evt in self.test_button.think(game.mouse(), &ui_cfg.test_button) {
            match evt {
                ButtonThinkResult::Clicked => log("Button Clicked"),
            }
        }
    }

    fn draw(&self, game: &dyn BaseGame, ui_cfg: &GameManagerUiConfig) {
        self.players.draw(game);

        self.test_button.draw(game.mouse(), game.painter(), &ui_cfg.test_button);
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
    Unset,
    Host(HostGameManager),
    Client(ClientGameManager),
}

impl GameManager {
    pub fn new_client(game: &mut dyn BaseGame) -> Self {
        GameManager::Client(ClientGameManager::new(game))
    }

    pub fn new_host(game: &mut dyn BaseGame, config: &GameManagerConfig) -> Self {
        GameManager::Host(HostGameManager::new(game, config))
    }

    pub fn think(&mut self, game: &mut dyn BaseGame, config: &GameManagerConfig, ui_cfg: &GameManagerUiConfig) {
        match self {
            Self::Host(mgr) => mgr.think(game, config, ui_cfg),
            Self::Client(mgr) => mgr.think(game, config),
            Self::Unset => {}
        }
    }

    pub fn draw(&self, game: &dyn BaseGame, ui_cfg: &GameManagerUiConfig) {
        match self {
            Self::Host(mgr) => mgr.draw(game, ui_cfg),
            Self::Client(mgr) => mgr.draw(game),
            Self::Unset => {}
        }
    }
}
