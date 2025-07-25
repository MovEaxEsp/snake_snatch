

use engine_p::interpolable::{Interpolable, Pos2d}; 
use serde::{Serialize,Deserialize};

use crate::traits::BaseGame;

#[derive(Clone, Serialize, Deserialize)]
pub struct SnakeConfig {
    pub grow_speed: f64,
}

struct OwnSnakeImp {
}

impl OwnSnakeImp {
    pub fn think(&mut self, snake_points: &mut Vec<Pos2d>, game: &dyn BaseGame, config: &SnakeConfig) {
        // Update the size of our snake depending on if mouse is down or up
        let snake_intr = Interpolable::new(*snake_points.last().unwrap(), config.grow_speed);
        if game.is_mouse_down() && game.mouse_pos() != *snake_points.last().unwrap() {
            snake_intr.set_end(game.mouse_pos());
            snake_intr.advance(game.elapsed_time());
            *snake_points.last_mut().unwrap() = snake_intr.cur();

            // Make sure each snake segment isn't too long
            if snake_points.last().unwrap().dist(snake_points[snake_points.len()-2]) > 20.0 {
                snake_points.push(*snake_points.last().unwrap());
            }
        }
        else if !game.is_mouse_down() && snake_points.len() > 2 {
            // Shrink the snake while the mouse is up
            let segment_start = snake_points[snake_points.len()-2];
            snake_intr.set_end(segment_start);
            snake_intr.advance(game.elapsed_time());
            let cur = snake_intr.cur();
            if cur == segment_start {
                snake_points.pop();
            }
            else {
                *snake_points.last_mut().unwrap() = cur;
            }
        }
    }
}

pub struct Snake {
    snake_points: Vec<Pos2d>,
    own_imp: Option<OwnSnakeImp>, // if this snake is controlled locally
}

impl Snake {
    
    pub fn new() -> Self {
        Self {
            snake_points: vec![(200, 200).into(), (300, 300).into()],
            own_imp: Some(OwnSnakeImp {
            })
        }
    }
    
    pub fn think(&mut self, game: &dyn BaseGame, config: &SnakeConfig) {
        if let Some(own) = &mut self.own_imp {
            own.think(&mut self.snake_points, game, config);
        }
    }
    
    pub fn draw(&self, game: &dyn BaseGame) {
        let canvas = game.painter().canvas();

        canvas.set_stroke_style_str("black");
        canvas.set_line_width(10.0);
        canvas.move_to(self.snake_points[0].x, self.snake_points[0].y);
        for pos in self.snake_points[1..].iter() {
            canvas.line_to(pos.x, pos.y);
            canvas.stroke();
            canvas.begin_path();
            canvas.move_to(pos.x, pos.y);
        }
        
    }
}