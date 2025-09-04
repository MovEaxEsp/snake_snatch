
use engine_p::interpolable::Pos2d;
use serde::{Serialize, Deserialize};


use crate::mouse::MouseManager;
use crate::network::NetworkManager;
use crate::painter::Painter;
use crate::players::{PlayerMsg, PlayersMsg};
use crate::snake::SnakeMsg;

// Handshake sent by a client to the host, telling it the stream to use to send
// player-relaed messages.  Sent over stream 0
#[derive(Debug, Deserialize, Serialize)]
pub struct NewClientMsg {
    pub players_stream: i32,
}


#[derive(Debug, Deserialize, Serialize)]
pub enum NetMsg {
    // To measure round-trip latency.  Arg is time when ping was sent.  Positive
    // on request, negative on response
    Ping(f64),

    // Sent by a new client to the host
    NewClient(NewClientMsg),

    // PlayerManager-specific messages
    Players(PlayersMsg),

    // Player-specific messages
    Player(PlayerMsg),

    // Message for 'snake.rs', about the state of a particular snake
    // Sent over a dedicated stream
    Snake(SnakeMsg),

    // Sent by the host to inform players about a changed list
    // of possible snake start points
    StartPointsUpdate(Vec<Pos2d>),
}

pub trait BaseGame {
    //fn set_global_alpha(&self, alpha: f64);

    //fn images<'a>(&'a self) -> &'a Images;

    fn painter<'a>(&'a self) -> &'a Painter;

    fn network(&mut self) -> &mut NetworkManager<NetMsg>;

    //fn sounds(&self) -> &Sounds;

    //fn image_props<'a>(&'a self, image: &Image) -> &'a ImageProps;

    fn elapsed_time(&self) -> f64;

    fn now(&self) -> f64;

    fn mouse(&self) -> &MouseManager;
}
