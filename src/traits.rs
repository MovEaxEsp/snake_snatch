
use engine_p::interpolable::Pos2d;
use serde::{Serialize, Deserialize};


use crate::network::NetworkManager;
use crate::painter::Painter;
use crate::snake::SnakeMsg;

// Sent by a new player connecting to the host
// Or by the host to each player to tell it about an existing player.
#[derive(Debug, Deserialize, Serialize)]
pub struct SnakeIntroMsg {
    pub name: String,
    pub snake_stream_id: i32,
    pub start_points: Vec<Pos2d>,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum NetMsg {
    // To measure round-trip latency.  Arg is time when ping was sent.  Positive
    // on request, negative on response
    Ping(f64),

    // Sent by a client or host to inform the peer about a snake in the game
    SnakeIntro(SnakeIntroMsg),
    
    // Message for 'snake.rs', about the state of a particular snake
    // Sent over a dedicated stream
    Snake(SnakeMsg),
    
    // Sent by the host to inform players about a changed list
    // of possible snake start points
    StartPointsUpdate(Vec<Vec<Pos2d>>),
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
    
    fn mouse_pos(&self) -> Pos2d;
    
    fn is_mouse_down(&self) -> bool;
}