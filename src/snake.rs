

use engine_p::interpolable::{Interpolable, Pos2d}; 
use serde::{Serialize,Deserialize};

use crate::network::{NetData, NetworkHandle, NetUpdate};
use crate::traits::{BaseGame, NetMsg};
use crate::utils::log;

// Config structs
#[derive(Clone, Serialize, Deserialize)]
pub struct SnakeConfig {
    pub grow_speed: f64,
}

// Network Msgs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EndUpdateMsg {
    prev_segs: usize, // number of pre-existing segments before 'last_segs'
    prev_segs_sum: f64, // sum of prev_segs' x and y values, to serve as a hash
    last_segs: Vec<Pos2d> // the coordinates of the last segments
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SnakeMsg {
    EndUpdate(EndUpdateMsg),
    FullUpdateReq,
}

/// Helper Functions
fn read_snake_msgs(updates: Vec<NetUpdate<NetMsg>>, peer: NetworkHandle, stream_id: i32, cb: &mut dyn FnMut(SnakeMsg)) {
    for upd in updates.into_iter() {
        if let NetUpdate::Data(NetData{msg: NetMsg::Snake(msg), ..}) = upd {
            cb(msg);
            continue;
        }

        log(&format!("Unexpected snake stream msg. Peer: {}, Stream: {}, msg: {:?}", &peer, stream_id, &upd));                    
    }
}

/// SnakeData
struct SnakeData {
    snake_points: Vec<Pos2d>,
    name: String,
    points_changed: bool,
}

impl SnakeData {
    fn points_sum(&self, num_points: usize) -> f64 {
        self.snake_points[0..num_points].iter().map(|p| p.x + p.y).sum::<f64>()
    }
}

/// OwnSnakeImp
struct OwnSnakeImp {
}

impl OwnSnakeImp {
    pub fn think(&mut self, data: &mut SnakeData, game: &dyn BaseGame, config: &SnakeConfig) {
        // Update the size of our snake depending on if mouse is down or up
        let snake_points = &mut data.snake_points;
        let snake_intr = Interpolable::new(*snake_points.last().unwrap(), config.grow_speed);
        if game.mouse().is_down() && game.mouse().pos() != *snake_points.last().unwrap() {
            snake_intr.set_end(game.mouse().pos());
            snake_intr.advance(game.elapsed_time());
            *snake_points.last_mut().unwrap() = snake_intr.cur();

            // Make sure each snake segment isn't too long
            if snake_points.last().unwrap().dist(snake_points[snake_points.len()-2]) > 20.0 {
                snake_points.push(*snake_points.last().unwrap());
            }
            
            data.points_changed = true;
        }
        else if !game.mouse().is_down() && snake_points.len() > 2 {
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

            data.points_changed = true;
        }
    }
}

/// RemoteSnakeImp
// To handle Snake events when it's controlled by a remote peer
struct RemoteSnakeImp {
    peer: NetworkHandle,
    stream_id: i32,
}

impl RemoteSnakeImp {
    fn think(&mut self, data: &mut SnakeData, game: &mut dyn BaseGame) {
        let upds = game.network().get_handle_events(self.peer, self.stream_id);
        read_snake_msgs(upds, self.peer, self.stream_id, &mut |msg| {
            match &msg {
                SnakeMsg::EndUpdate(upd) => {
                    if data.snake_points.len() < upd.prev_segs {
                        log(&format!("Snake({}) not enough segments for update: {:?}", data.name, msg));
                        game.network().send(&self.peer, self.stream_id, NetMsg::Snake(SnakeMsg::FullUpdateReq));
                        return;
                    } 
                    
                    if data.points_sum(upd.prev_segs) != upd.prev_segs_sum {
                        log(&format!("Snake({}) prev_segs_sum wrong for update: {:?}", data.name, msg));
                        game.network().send(&self.peer, self.stream_id, NetMsg::Snake(SnakeMsg::FullUpdateReq));
                        return;
                    }
                    
                    // Update can be processed fine
                    let pts = &mut data.snake_points;
                    pts.truncate(upd.prev_segs);
                    upd.last_segs.iter().for_each(|p| pts.push(*p));
                    
                    data.points_changed = true;
                },
                _ => {
                    log(&format!("Unexpected msg from snake remote: {},{}: {:?}", self.peer, self.stream_id, &msg));
                }
            }
        });
    }
}

/// SnakePeer
struct SnakePeer {
    peer: NetworkHandle,
    stream_id: i32,
    next_send_time: f64,
}

impl SnakePeer {
    fn think(&mut self, data: &mut SnakeData, game: &mut dyn BaseGame) {
        let upds = game.network().get_handle_events(self.peer, self.stream_id);
        read_snake_msgs(upds, self.peer, self.stream_id, &mut |msg| {
            match &msg {
                SnakeMsg::FullUpdateReq => {
                    game.network().send(
                        &self.peer,
                        self.stream_id,
                        NetMsg::Snake(SnakeMsg::EndUpdate(EndUpdateMsg {
                            prev_segs: 0,
                            prev_segs_sum: 0.0,
                            last_segs: data.snake_points.clone(),
                        })));
                },
                _ => {
                    log(&format!("Unexpected msg from snake peer: {},{}: {:?}", self.peer, self.stream_id, &msg));
                }
            }
        });
        
        if data.points_changed && self.next_send_time < game.now() {
            // Time to send our points update.  Send our last 2 points in diffs only.  This should hopefully
            // be good enough and not require too many full updates
            let pts = &data.snake_points;
            if pts.len() > 2 {
                game.network().send(
                    &self.peer,
                    self.stream_id,
                    NetMsg::Snake(SnakeMsg::EndUpdate(EndUpdateMsg {
                        prev_segs: pts.len() - 2,
                        prev_segs_sum: data.points_sum(pts.len() - 2),
                        last_segs: pts[pts.len()-2..].iter().cloned().collect(),
                    })));
            }
            else {
                game.network().send(
                    &self.peer,
                    self.stream_id,
                    NetMsg::Snake(SnakeMsg::EndUpdate(EndUpdateMsg {
                        prev_segs: 0,
                        prev_segs_sum: 0.0,
                        last_segs: pts.clone(),
                    })));
            }

            self.next_send_time = game.now() + 0.1; // At most one update every 100ms
        }
    }
}

pub struct Snake {
    data: SnakeData,
    own_imp: Option<OwnSnakeImp>, // if this snake is controlled locally
    remote_imp: Option<RemoteSnakeImp>, // if this snake is controlled remotely (by a peer)
    peers: Vec<SnakePeer>, // peers to send snake updates to
}

impl Snake {
    pub fn new_local(name: &str, start_points: &Vec<Pos2d>) -> Self {
        Self {
            data: SnakeData {
                snake_points: start_points.clone(),
                name: name.to_string(),
                points_changed: false,
            },
            own_imp: Some(OwnSnakeImp {
            }),
            remote_imp: None,
            peers: Vec::new(),
        }
    }
    
    pub fn new_remote(name: &str, peer: NetworkHandle, stream_id: i32, start_points: &Vec<Pos2d>) -> Self {
        Self {
            data: SnakeData {
                snake_points: start_points.clone(),
                name: name.to_string(),
                points_changed: false,
            },
            own_imp: None,
            remote_imp: Some(RemoteSnakeImp {
                peer,
                stream_id
            }),
            peers: Vec::new(),
        }
    }
    
    pub fn add_peer(&mut self, peer: NetworkHandle, stream_id: i32) {
        self.peers.push(SnakePeer {
            peer,
            stream_id,
            next_send_time: 0.0,
        });
    }
    
    // Return our start_points (first 2 points of the snake)
    pub fn get_start_points(&self) -> Vec<Pos2d> {
        self.data.snake_points[..2].iter().cloned().collect()
    }
    
    // Handle per-frame processing
    pub fn think(&mut self, game: &mut dyn BaseGame, config: &SnakeConfig) {
        self.data.points_changed = false;

        if let Some(own) = &mut self.own_imp {
            own.think(&mut self.data, game, config);
        }

        if let Some(remote) = &mut self.remote_imp {
            remote.think(&mut self.data, game);
        }
        
        for peer in self.peers.iter_mut() {
            peer.think(&mut self.data, game);
        }
    }
    
    // Draw our snake
    pub fn draw(&self, game: &dyn BaseGame) {
        let canvas = game.painter().canvas();

        canvas.set_stroke_style_str("black");
        canvas.set_line_width(10.0);
        canvas.move_to(self.data.snake_points[0].x, self.data.snake_points[0].y);
        for pos in self.data.snake_points[1..].iter() {
            canvas.line_to(pos.x, pos.y);
            canvas.stroke();
            canvas.begin_path();
            canvas.move_to(pos.x, pos.y);
        }
    }
}