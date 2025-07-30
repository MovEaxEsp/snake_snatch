
use engine_p::interpolable::Pos2d;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, MouseEvent, TouchEvent};

use std::cell::RefCell;
use std::rc::Rc;

#[derive(PartialEq)]
pub enum MouseEventType {
    Up,
    Down,
    Move,
}

struct MouseManagerImp {
    canvas: HtmlCanvasElement,
    real_width: f64,
    real_height: f64,
    is_down: bool,
    pos: Pos2d,
}

impl MouseManagerImp {
    fn handle_event(&mut self, event_type: MouseEventType, event_x: i32, event_y: i32) {
        if event_type == MouseEventType::Down {
            self.is_down = true;
        }
        else if event_type == MouseEventType::Up {
            self.is_down = false;
        }

        // Adjust event x and y for offscreen canvas coordinates
        let width_factor = self.real_width / self.canvas.width() as f64;
        let height_factor = self.real_height / self.canvas.height() as f64;
        
        let rect = self.canvas.get_bounding_client_rect();
        self.pos.x = (event_x as f64 - rect.left()) * width_factor;
        self.pos.y = (event_y as f64 - rect.top()) * height_factor;
    }
}

pub struct MouseManager {
    imp: Rc<RefCell<MouseManagerImp>>,
    _on_mousedown_closure: Closure::<dyn FnMut(MouseEvent)>,
    _on_mouseup_closure: Closure::<dyn FnMut(MouseEvent)>,
    _on_mousemove_closure: Closure::<dyn FnMut(MouseEvent)>,
    _on_touchstart_closure: Closure::<dyn FnMut(TouchEvent)>,
    _on_touchend_closure: Closure::<dyn FnMut(TouchEvent)>,
    _on_touchmove_closure: Closure::<dyn FnMut(TouchEvent)>,
}

impl MouseManager {
    // PUBLIC
    pub fn new(canvas: HtmlCanvasElement, real_width: f64, real_height: f64) -> Self {
        let imp = Rc::new(RefCell::new(MouseManagerImp {
            canvas: canvas.clone(),
            real_width,
            real_height,
            is_down: false,
            pos: (0,0).into(),
        }));

        // Mouse down
        let mut imp_ref = imp.clone();
        let mouse_down_closure = Closure::<dyn FnMut(MouseEvent)>::new(move |evt: MouseEvent| {
            let cb_imp = &mut *imp_ref.borrow_mut();
            cb_imp.handle_event(MouseEventType::Down, evt.x(), evt.y());
        });
        canvas.set_onmousedown(Some(mouse_down_closure.as_ref().unchecked_ref()));
        
        // Mouse up
        imp_ref = imp.clone();
        let mouse_up_closure = Closure::<dyn FnMut(MouseEvent)>::new(move |evt: MouseEvent| {
            let cb_imp = &mut *imp_ref.borrow_mut();
            cb_imp.handle_event(MouseEventType::Up, evt.x(), evt.y());
        });
        canvas.set_onmouseup(Some(mouse_up_closure.as_ref().unchecked_ref()));
        
        // Mouse move
        imp_ref = imp.clone();
        let mouse_move_closure = Closure::<dyn FnMut(MouseEvent)>::new(move |evt: MouseEvent| {
            let cb_imp = &mut *imp_ref.borrow_mut();
            cb_imp.handle_event(MouseEventType::Move, evt.x(), evt.y());
        });
        canvas.set_onmousemove(Some(mouse_move_closure.as_ref().unchecked_ref()));
        
        // Touch start
        imp_ref = imp.clone();
        let touch_start_closure = Closure::<dyn FnMut(TouchEvent)>::new(move |evt: TouchEvent| {
            let cb_imp = &mut *imp_ref.borrow_mut();
            let touch = evt.target_touches().item(0).unwrap();
            cb_imp.handle_event(MouseEventType::Down,
                                touch.client_x(),
                                touch.client_y());
        });
        canvas.add_event_listener_with_callback_and_bool(
                                   "touchstart",
                                   touch_start_closure.as_ref().unchecked_ref(),
                                   false).expect("touchstart");
        
        // Touch end
        imp_ref = imp.clone();
        let touch_end_closure = Closure::<dyn FnMut(TouchEvent)>::new(move |evt: TouchEvent| {
            let cb_imp = &mut *imp_ref.borrow_mut();
            let touch = evt.target_touches().item(0).unwrap();
            cb_imp.handle_event(MouseEventType::Up,
                                touch.client_x(),
                                touch.client_y());
        });
        canvas.add_event_listener_with_callback_and_bool(
                                   "touchsend",
                                   touch_end_closure.as_ref().unchecked_ref(),
                                   false).expect("touchend");
        canvas.add_event_listener_with_callback_and_bool(
                                   "touchcancel",
                                   touch_end_closure.as_ref().unchecked_ref(),
                                   false).expect("touchcancel");
        
        imp_ref = imp.clone();
        let touch_move_closure = Closure::<dyn FnMut(TouchEvent)>::new(move |evt: TouchEvent| {
            let cb_imp = &mut *imp_ref.borrow_mut();
            let touch = evt.target_touches().item(0).unwrap();
            cb_imp.handle_event(MouseEventType::Move,
                                touch.client_x(),
                                touch.client_y());
            evt.prevent_default();
        });
        canvas.add_event_listener_with_callback_and_bool(
                                   "touchmove",
                                   touch_move_closure.as_ref().unchecked_ref(),
                                   false).expect("touchmove");

        Self {
            imp,
            _on_mousedown_closure: mouse_down_closure,
            _on_mouseup_closure: mouse_up_closure,
            _on_mousemove_closure: mouse_move_closure,
            _on_touchstart_closure: touch_start_closure,
            _on_touchend_closure: touch_end_closure,
            _on_touchmove_closure: touch_move_closure,
        }
    }
    
    pub fn is_down(&self) -> bool {
        (*self.imp).borrow().is_down
    }
    pub fn pos(&self) -> Pos2d {
        (*self.imp).borrow().pos
    }
}