
use crate::painter::Painter;
use crate::sounds::Sounds;

pub trait BaseGame {
    //fn set_global_alpha(&self, alpha: f64);

    fn get_money(&self) -> i32;

    //fn images<'a>(&'a self) -> &'a Images;

    fn painter<'a>(&'a self) -> &'a Painter;

    fn sounds(&self) -> &Sounds;

    //fn image_props<'a>(&'a self, image: &Image) -> &'a ImageProps;

    fn elapsed_time(&self) -> f64;
}