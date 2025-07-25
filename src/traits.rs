
use engine_p::interpolable::Pos2d;
use serde::{Serialize, Deserialize};


use crate::painter::Painter;
use crate::network::NetworkManager;

// Sent by a new player connecting to the host (with just a name)
// Or by the host to each player to tell it about an existing player.
#[derive(Debug, Deserialize, Serialize)]
pub struct PlayerIntroMsg {
    name: String,
    id: Option<i32>,
    pos: Option<(i32, i32)>,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum NetMsg {
    SnakePos((i32, i32)),
    PlayerIntro(PlayerIntroMsg),
}

pub trait BaseGame {
    //fn set_global_alpha(&self, alpha: f64);

    //fn images<'a>(&'a self) -> &'a Images;

    fn painter<'a>(&'a self) -> &'a Painter;
    
    fn network(&mut self) -> &mut NetworkManager<NetMsg>;

    //fn sounds(&self) -> &Sounds;

    //fn image_props<'a>(&'a self, image: &Image) -> &'a ImageProps;

    fn elapsed_time(&self) -> f64;
    
    fn mouse_pos(&self) -> Pos2d;
    
    fn is_mouse_down(&self) -> bool;
}