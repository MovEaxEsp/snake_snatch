
use crate::painter::Painter;
use crate::network::NetworkManager;

use serde::{Serialize, Deserialize};

#[derive(Debug, Deserialize, Serialize)]
pub enum NetMsg {
    SnakePos((i32, i32)),
}

pub trait BaseGame {
    //fn set_global_alpha(&self, alpha: f64);

    //fn images<'a>(&'a self) -> &'a Images;

    fn painter<'a>(&'a self) -> &'a Painter;
    
    fn network(&mut self) -> &mut NetworkManager<NetMsg>;

    //fn sounds(&self) -> &Sounds;

    //fn image_props<'a>(&'a self, image: &Image) -> &'a ImageProps;

    fn elapsed_time(&self) -> f64;
}