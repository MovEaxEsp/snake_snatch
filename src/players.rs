
use crate::network::{NetUpdate, NetworkHandle, StreamHandle};
use crate::snake::{Snake, SnakeConfig};
use crate::traits::{BaseGame, NetMsg};
use crate::utils::log;

use engine_p::interpolable::Pos2d;
use serde::{Serialize, Deserialize};

use std::collections::HashMap;

/// Network messages

// Sent by a client (to identify its player) or the host
// (to identify its own, and other players)
// Sent over 'players_stream'
#[derive(Debug, Deserialize, Serialize)]
pub struct NewPlayerMsg {
    pub name: String,
    pub player_stream: i32,
}

// Sent by the host to all connected clients when a client disconnects,
// identified by its 'player_stream'.  Sent over the players_stream
#[derive(Debug, Deserialize, Serialize)]
pub struct PlayerLeftMsg {
    player_stream: i32,
}

// Sent by the host to any client when its available options/actions change
// for example, where/if it can spawn.  Also sent as a response to an unsuccessful
// RequestSnakeMsg.  Sent over a client's own 'player_stream'
#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateChoicesMsg {
    snake_points: Vec<Pos2d>, // if empty, can't spawn
}

// Sent by a client to request to have its snake placed at a specific location
// Sent over a 'player_stream'
#[derive(Debug, Deserialize, Serialize)]
pub struct RequestSnakeMsg {
    pos: Pos2d,
}

// Used by the host to tell a client about a snake at a position.  Sent over the
// corresponding 'player_stream'.  If sent over the client's own player_stream,
// this gives the client confirmation about its own snake, and the stream to use for it.
#[derive(Debug, Deserialize, Serialize)]
pub struct NewSnakeMsg {
    pos: Pos2d,
    snake_stream: i32,
}

// Messages sent over the players_stream, processed by the PlayerManager
#[derive(Debug, Deserialize, Serialize)]
pub enum PlayersMsg {
    NewPlayer(NewPlayerMsg),
    PlayerLeft(PlayerLeftMsg),
}

// Messages sent over a 'player_stream', processed by a 'Player'
#[derive(Debug, Deserialize, Serialize)]
pub enum PlayerMsg {
    UpdateChoices(UpdateChoicesMsg),
    RequestSnake(RequestSnakeMsg),
    NewSnake(NewSnakeMsg),
}

/// Config types
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlayerManagerConfig {
    pub snake_start_points: Vec<Pos2d>,
    pub snake: SnakeConfig,
}

/// Helper types
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct PlayersStream(StreamHandle);
impl PlayersStream {
    fn process_msgs(&self, game: &mut dyn BaseGame, cb: &mut dyn FnMut(&PlayersMsg, &mut dyn BaseGame) -> bool) {
        let msgs = game.network().get_stream_msgs(self.0);
        for outer in msgs.iter() {
            let mut processed = false;
            if let NetMsg::Players(msg) = outer {
                processed = cb(msg, game);
            }
            if !processed {
                log(&format!("Unexpected players message on stream {} :: {:?}", self.0, outer));
            }
        }
    }

    fn send(&self, game: &mut dyn BaseGame, msg: PlayersMsg) {
        game.network().send(&self.0, NetMsg::Players(msg));
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct PlayerStream(StreamHandle);
impl PlayerStream {
    fn process_msgs(&self, game: &mut dyn BaseGame, cb: &mut dyn FnMut(&PlayerMsg, &mut dyn BaseGame) -> bool) {
        let msgs = game.network().get_stream_msgs(self.0);
        for outer in msgs.iter() {
            let mut processed = false;
            if let NetMsg::Player(msg) = outer {
                processed = cb(msg, game);
            }
            if !processed {
                log(&format!("Unexpected player message on stream {} :: {:?}", self.0, outer));
            }
        }
    }

    fn send(&self, game: &mut dyn BaseGame, msg: PlayerMsg) {
        game.network().send(&self.0, NetMsg::Player(msg));
    }
}

/// HostPlayer
/// A player managed by the host

struct HostPlayer {
    name: String,
    snake: Option<Snake>,

    // The client's 'players_stream'
    players_stream: Option<PlayersStream>,

    // Our 'player' stream for this player with the host.  Not set for the
    // self_player.
    player_stream: Option<PlayerStream>,

    // map of other player's players_streams to the player_stream used to forward them
    // information about *this* player.
    peer_streams: HashMap<PlayersStream, PlayerStream>,

    // Do we need to send an UpdateChoicesMsg to this player?
    need_update_choices: bool,
}

impl HostPlayer {
    fn think(&mut self, game: &mut dyn BaseGame, open_positions: &mut Vec<Pos2d>, config: &PlayerManagerConfig) {
        if let Some(stream) = self.player_stream {
            stream.process_msgs(game, &mut |outer, g| match outer {
                PlayerMsg::RequestSnake(msg) => {
                    match open_positions.iter().position(|p| *p == msg.pos) {
                        Some(idx) => {
                            // Player requested an available position
                            open_positions.remove(idx);

                            let snake_stream = g.network().new_sibling_stream(&stream.0).unwrap();

                            stream.send(g, PlayerMsg::NewSnake(NewSnakeMsg {
                                pos: msg.pos,
                                snake_stream: snake_stream.stream_id()
                            }));

                            // Make the snake for the player
                            let mut snake = Snake::new_remote(&self.name, snake_stream, &msg.pos);

                            // Inform all other players about our snake
                            for (_, peer_stream) in self.peer_streams.iter() {
                                let peer_snake_stream = g.network().new_sibling_stream(&peer_stream.0).unwrap();
                                snake.add_peer(peer_snake_stream);
                                peer_stream.send(g, PlayerMsg::NewSnake(NewSnakeMsg {
                                    pos: msg.pos,
                                    snake_stream: peer_snake_stream.stream_id(),
                                }));
                            }

                            self.snake = Some(snake);
                        },
                        None => {
                            // Player requested an invalid position.
                            log(&format!("Player requested invalid snake position.  player_stream {}, msg: {:?}, available: {:?}",
                                stream.0, outer, &open_positions));
                            self.need_update_choices = true;
                        }
                    }
                    true
                }
                _ => false
            });

            if self.need_update_choices {
                stream.send(game, PlayerMsg::UpdateChoices(UpdateChoicesMsg {
                    snake_points: open_positions.clone()
                }));
                self.need_update_choices = false;
            }
        }

        if let Some(snake) = &mut self.snake {
            snake.think(game, &config.snake);
        }
    }

    fn draw(&self, game: &dyn BaseGame) {
        if let Some(snake) = &self.snake {
            snake.draw(game);
        }
    }

    /// Ensure that we have a peer_stream for every stream in 'other_streams', except for our own players_stream
    fn ensure_peer_streams(&mut self, game: &mut dyn BaseGame, other_streams: &Vec<PlayersStream>) {
        if let Some(my_stream) = self.players_stream {
            for other_stream in other_streams {
                if my_stream == *other_stream {
                    continue;
                }

                let name = &self.name;

                self.peer_streams.entry(*other_stream).or_insert_with(|| {
                    // Make a new peer_stream between us and 'other_stream'
                    let new_stream = game.network().new_sibling_stream(&other_stream.0).unwrap();
                    other_stream.send(game, PlayersMsg::NewPlayer(NewPlayerMsg {
                        name: name.clone(),
                        player_stream: new_stream.stream_id(),
                    }));

                    PlayerStream{0: new_stream}
                });
            }
        }
    }

    fn remove_peer_stream(&mut self, game: &mut dyn BaseGame, key_stream: &PlayersStream) {
        if let Some(my_stream) = self.peer_streams.remove(key_stream) {
            key_stream.send(game, PlayersMsg::PlayerLeft(PlayerLeftMsg {
                player_stream: my_stream.0.stream_id()
            }));
        }
    }
}

/// HostPlayerManager
/// PlayerManager used when we're acting as the host
pub struct HostPlayerManager {
    // Available positions for snakes
    open_positions: Vec<Pos2d>,

    // the key is the corresponding client's network handle, or None for the host
    players: HashMap<Option<NetworkHandle>, HostPlayer>,
}

impl HostPlayerManager {
    pub fn new(self_name: &str, config: &PlayerManagerConfig) -> Self {
        let mut open_positions = config.snake_start_points.clone();
        open_positions.remove(0); // Remove the position the host is using

        let host_player = HostPlayer {
            name: self_name.to_string(),
            snake: Some(Snake::new_local("HostSnake", &config.snake_start_points[0])),
            players_stream: None,
            player_stream: None,
            peer_streams: HashMap::new(),
            need_update_choices: false,
        };

        let mut players = HashMap::new();
        players.insert(None, host_player);

        Self {
            open_positions,
            players,
        }
    }

    /// Add a new client with the corresponding 'players_stream'.
    pub fn add_client(&mut self, handle: NetworkHandle) {
        self.players.insert(Some(handle), HostPlayer {
            name: "".to_string(),
            snake: None,
            players_stream: None,
            player_stream: None,
            peer_streams: HashMap::new(),
            need_update_choices: true,
        });
    }

    pub fn think(&mut self, game: &mut dyn BaseGame, config: &PlayerManagerConfig) {
        let mut have_new_player = false;
        let mut closed_handles: Vec<NetworkHandle> = Vec::new();

        // Process all players (including host at key None)
        for (handle_opt, player) in self.players.iter_mut() {
            // Only process network events for actual clients (not the host)
            if let Some(handle) = handle_opt {
                // Process 'handle' events
                for outer in game.network().get_handle_events(*handle) {
                    match outer {
                        NetUpdate::Closed => {
                            log(&format!("Client disconnected: {}", handle));
                            closed_handles.push(*handle);
                        }
                        _ => {
                            log(&format!("Unexpected NetUpdate for client handle {} :: {:?}", handle, outer));
                        }
                    }
                }

                // Process stream-0 events (NewClient)
                for outer in game.network().get_stream_msgs(handle.default_stream()) {
                    match outer {
                        NetMsg::NewClient(msg) => {
                            let players_stream = handle.default_stream().sibling(msg.players_stream);
                            player.players_stream = Some(PlayersStream {0: players_stream});
                        }
                        _ => {
                            log(&format!("Unexpected message over default stream {} :: {:?}", handle.default_stream(), outer));
                        }
                    }
                }

                // Process 'players_stream' messages
                if let Some(stream) = player.players_stream {
                    stream.process_msgs(game, &mut |outer, _g| match outer {
                        PlayersMsg::NewPlayer(msg) => {
                            let player_stream = stream.0.sibling(msg.player_stream);
                            player.name = msg.name.clone();
                            player.player_stream = Some(PlayerStream{0:player_stream});

                            have_new_player = true;

                            true
                        }
                        _ => false
                    });
                }
            }

            // Allow the player itself to think
            player.think(game, &mut self.open_positions, config);
        }

        // Clean up disconnected clients
        for hndl in closed_handles.into_iter() {
            let dead_player = self.players.remove(&Some(hndl)).unwrap();
            if let Some(players_stream) = dead_player.players_stream {
                for (_, player) in self.players.iter_mut() {
                    player.remove_peer_stream(game, &players_stream);
                }
            }
        }

        if have_new_player {
            // Collect information about new players first
            let mut new_player_streams = Vec::new();
            for (handle_opt, player) in self.players.iter() {
                if let Some(_handle) = handle_opt {
                    if let Some(player_stream) = player.player_stream {
                        new_player_streams.push(player_stream);
                    }
                }
            }

            // Add new clients as peers to the host's snake
            if let Some(host_player) = self.players.get_mut(&None) {
                if let Some(host_snake) = &mut host_player.snake {
                    for player_stream in &new_player_streams {
                        let host_snake_stream = game.network().new_sibling_stream(&player_stream.0).unwrap();
                        host_snake.add_peer(host_snake_stream);

                        // Tell the client about the host's snake
                        player_stream.send(game, PlayerMsg::NewSnake(NewSnakeMsg {
                            pos: config.snake_start_points[0], // Host's position
                            snake_stream: host_snake_stream.stream_id(),
                        }));
                    }
                }
            }

            // Make sure each player has a player_stream for every other player
            let mut players_streams = Vec::<PlayersStream>::new();
            for (handle_opt, player) in self.players.iter() {
                // Skip the host player (None key)
                if handle_opt.is_some() {
                    if let Some(str) = player.players_stream {
                        players_streams.push(str);
                    }
                }
            }

            for (handle_opt, player) in self.players.iter_mut() {
                // Only clients need peer streams (not the host)
                if handle_opt.is_some() {
                    player.ensure_peer_streams(game, &players_streams);
                }
            }
        }
    }

    fn draw(&self, game: &dyn BaseGame) {
        for (_, player) in self.players.iter() {
            player.draw(game);
        }
    }
}

/// ClientPlayer
struct ClientPlayer {
    is_local: bool,
    name: String,
    snake: Option<Snake>,

    // Our 'player' stream for this player with the host.
    player_stream: PlayerStream,
}

impl ClientPlayer {
    fn think(&mut self, game: &mut dyn BaseGame, config: &PlayerManagerConfig) {
        let stream = self.player_stream;
        stream.process_msgs(game, &mut |outer, g| match outer {
            PlayerMsg::UpdateChoices(msg) => {
                if self.snake.is_none() && msg.snake_points.len() > 0 {
                    // Try to place our snake at the first available location
                    self.player_stream.send(g, PlayerMsg::RequestSnake(RequestSnakeMsg {
                        pos: msg.snake_points[0]
                    }));
                }
                true
            }
            PlayerMsg::NewSnake(msg) => {
                let mut snake;
                let snake_stream = self.player_stream.0.sibling(msg.snake_stream);
                if self.is_local {
                    snake = Snake::new_local(&self.name, &msg.pos);
                    snake.add_peer(snake_stream);
                }
                else {
                    snake = Snake::new_remote(&self.name, snake_stream, &msg.pos);
                }

                self.snake = Some(snake);
                true
            }
            _ => false
        });

        if let Some(snake) = &mut self.snake {
            snake.think(game, &config.snake);
        }
    }

    fn draw(&self, game: &dyn BaseGame) {
        if let Some(snake) = &self.snake {
            snake.draw(game);
        }
    }
}

/// ClientPlayerManager
/// PlayerManager used when we're acting as a client
pub struct ClientPlayerManager {
    // the key is the player's player_stream.
    players: HashMap<StreamHandle, ClientPlayer>,
    host_players_stream: PlayersStream,
}

impl ClientPlayerManager {
    pub fn new(self_name: &str, host_players_stream: StreamHandle, player_stream: StreamHandle) -> Self {
        let self_player = ClientPlayer {
            is_local: true,
            name: self_name.to_string(),
            snake: None,
            player_stream: PlayerStream{0:player_stream},
        };

        Self {
            players: HashMap::from([
                (player_stream, self_player)
            ]),
            host_players_stream: PlayersStream{0:host_players_stream},
        }
    }

    pub fn think(&mut self, game: &mut dyn BaseGame, config: &PlayerManagerConfig) {
        let stream = self.host_players_stream;
        stream.process_msgs(game, &mut |outer, _g| match outer {
            PlayersMsg::NewPlayer(msg) => {
                let player_stream = self.host_players_stream.0.sibling(msg.player_stream);
                self.players.insert(player_stream, ClientPlayer {
                    is_local: false,
                    name: msg.name.clone(),
                    snake: None,
                    player_stream: PlayerStream{0:player_stream},
                });
                true
            },
            PlayersMsg::PlayerLeft(msg) => {
                self.players.remove(&self.host_players_stream.0.sibling(msg.player_stream));
                true
            },
        });

        for (_, player) in self.players.iter_mut() {
            player.think(game, config);
        }
    }

    fn draw(&self, game: &dyn BaseGame) {
        for (_, player) in self.players.iter() {
            player.draw(game);
        }
    }
}

pub enum PlayerManager {
    Unset,
    Client(ClientPlayerManager),
    Host(HostPlayerManager),
}

impl PlayerManager {
    pub fn new_client(self_name: &str, host_players_stream: &StreamHandle, player_stream: &StreamHandle) -> Self {
        PlayerManager::Client(ClientPlayerManager::new(self_name, *host_players_stream, *player_stream))
    }

    pub fn new_host(self_name: &str, config: &PlayerManagerConfig) -> Self {
        PlayerManager::Host(HostPlayerManager::new(self_name, config))
    }

    pub fn add_client(&mut self, handle: NetworkHandle) {
        match self {
            Self::Host(mgr) => {
                mgr.add_client(handle);
            }
            _ => {
                log(&format!("Bad PlayerManager type for add_client"));
            }
        }
    }

    pub fn think(&mut self, game: &mut dyn BaseGame, config: &PlayerManagerConfig) {
        match self {
            Self::Host(mgr) => {
                mgr.think(game, config);
            }
            Self::Client(mgr) => {
                mgr.think(game, config);
            }
            Self::Unset => {
                // Nothing
            }
        }
    }

    pub fn draw(&self, game: &dyn BaseGame) {
        match self {
            Self::Host(mgr) => {
                mgr.draw(game);
            }
            Self::Client(mgr) => {
                mgr.draw(game);
            }
            Self::Unset => {
                // Nothing
            }
        }
    }
}
