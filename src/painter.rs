extern crate engine_p;

use crate::images::{Image, Images, ImagesConfig};

use engine_p::interpolable::{Interpolable, Pos2d};

use serde::{Serialize,Deserialize};
use web_sys::OffscreenCanvasRenderingContext2d;


#[derive(Serialize, Deserialize, Clone)]
pub struct BackgroundConfig {
    pub offset: Pos2d,
    pub width: f64,
    pub height: f64,
    pub corner_radius: f64,
    pub border_style: String,
    pub border_alpha: f64,
    pub border_width: f64,
    pub bg_style: String,
    pub bg_alpha: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TextConfig {
    pub offset: Pos2d,
    pub stroke: bool,
    pub style: String,
    pub font: String,
    pub size: i32,
    pub center_and_fit: bool,
    pub is_command: bool,
    pub alpha: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ProgressBarConfig {
    pub bg: BackgroundConfig,
    pub done_style: String,
    pub done_alpha: f64,
}


#[derive(Serialize, Deserialize, Clone)]
pub struct RingConfig {
    pub stroke: bool,
    pub style: String,
}


pub struct Painter {
    canvas: OffscreenCanvasRenderingContext2d,
    images: Images,
    entered_keywords: Vec<String>,
    keyword_r: Interpolable<f64>,
    keyword_g: Interpolable<f64>,
    keyword_b: Interpolable<f64>,
}

impl Painter {

    pub fn new(images: Images, canvas: OffscreenCanvasRenderingContext2d) -> Self {
        Painter {
            canvas: canvas,
            images: images,
            entered_keywords: Vec::new(),
            keyword_r: Interpolable::new(72.0, 111.0),
            keyword_g: Interpolable::new(23.0, 79.0),
            keyword_b: Interpolable::new(219.0, 231.0),
        }
    }

    pub fn think(&mut self, elapsed_time: f64) {
        let advance_color = |intr: &mut Interpolable<f64>, elapsed_time: f64| {
            intr.advance(elapsed_time);
            if !intr.is_moving() {
                if intr.cur() == 0.0 {
                    intr.set_end(255.0);
                }
                else {
                    intr.set_end(0.0);
                }
            }
        };

        advance_color(&mut self.keyword_r, elapsed_time);
        advance_color(&mut self.keyword_g, elapsed_time);
        advance_color(&mut self.keyword_b, elapsed_time);
    }

    pub fn set_global_alpha(&self, alpha: f64) {
        self.canvas.set_global_alpha(alpha);
    }

    pub fn draw_image(&self, image: &Image, pos: &Pos2d) {
        self.images.draw_image(&self.canvas, image, pos.x, pos.y);
    }

    pub fn draw_gray_image(&self, image: &Image, pos: &Pos2d) {
        self.images.draw_gray_image(&self.canvas, image, pos.x, pos.y);
    }

    pub fn draw_area_background(&self, pos: &Pos2d, cfg: &BackgroundConfig) {
        let c = &self.canvas;

        c.set_stroke_style_str(&cfg.border_style);
        c.set_fill_style_str(&cfg.bg_style);
        c.set_line_width(cfg.border_width);

        // Draw backgound first
        c.set_global_alpha(cfg.bg_alpha);
        c.begin_path();
        c.round_rect_with_f64(
            pos.x + cfg.offset.x,
            pos.y + cfg.offset.y,
            cfg.width,
            cfg.height,
            cfg.corner_radius).expect("bg");
        c.fill();

        // Draw border
        c.set_global_alpha(cfg.border_alpha);
        c.begin_path();
        c.round_rect_with_f64(
            pos.x + cfg.offset.x,
            pos.y + cfg.offset.y,
            cfg.width,
            cfg.height,
            cfg.corner_radius).expect("border");
        c.stroke();

        c.set_global_alpha(1.0);
    }

    pub fn draw_progress_bar(&self, pos: &Pos2d, pct: f64, cfg: &ProgressBarConfig) {
        self.draw_area_background(pos, &cfg.bg);

        // Draw the progress indicator
        self.canvas.set_global_alpha(cfg.done_alpha);
        self.canvas.set_fill_style_str(&cfg.done_style);
        self.canvas.begin_path();
        self.canvas.round_rect_with_f64(
            pos.x + cfg.bg.offset.x,
            pos.y + cfg.bg.offset.y,
            cfg.bg.width * pct,
            cfg.bg.height,
            cfg.bg.corner_radius).expect("progress");
        self.canvas.fill();

        self.canvas.set_global_alpha(1.0);
    }

    
    pub fn draw_ring(&self, pos: &Pos2d, r1: f64, r2: f64, rad1: f64, rad2: f64, cfg: &RingConfig)
    {
        let c = &self.canvas;

        c.begin_path();
        c.arc(pos.x, pos.y, r1, rad1, rad2).expect("ring1");
        c.arc_with_anticlockwise(pos.x, pos.y, r2, rad2, rad1, true).expect("ring2");
        c.close_path();

        if cfg.stroke {
            c.set_stroke_style_str(&cfg.style);
            c.set_line_width(10.0);
            c.stroke();
        }
        else {
            c.set_fill_style_str(&cfg.style);
            c.fill();
        }
    }

    /*
    fn draw_halo(&self, xpos: f64, ypos: f64, width: f64, height: f64) {
        let middle_x = xpos + width/2.0;
        let middle_y = (ypos + height/2.0) * 2.0;
        let gradient = self.canvas.create_radial_gradient(middle_x, middle_y, 10.0, middle_x, middle_y, width/2.0).unwrap();
        gradient.add_color_stop(0.0, "rgba(255, 255, 255, .5)").unwrap();
        gradient.add_color_stop(1.0, "rgba(255,255,255,0)").unwrap();
        self.canvas.set_fill_style_canvas_gradient(&gradient);
        self.canvas.set_transform(1.0,0.0, 0.0, 0.5, 0.0, 0.0).unwrap();

        self.canvas.begin_path();
        self.canvas.ellipse(middle_x, middle_y, width/2.0, width/2.0, 0.0, 0.0, 2.0*f64::consts::PI).unwrap();
        self.canvas.fill();
        self.canvas.reset_transform().unwrap();
    }
    */

    pub fn draw_text(&self, text: &str, pos: &Pos2d, width: f64, cfg: &TextConfig) {
        let mut font_size: usize = cfg.size as usize;

        self.canvas.set_global_alpha(cfg.alpha);
        self.canvas.set_fill_style_str(&cfg.style);
        self.canvas.set_stroke_style_str(&cfg.style);
        self.canvas.set_text_baseline("top");

        let mut draw_pos = *pos;
        if cfg.center_and_fit {
            // Figure out where to draw the text, and at what size

            font_size += 1;
            let mut text_width = width + 1.0;
            while text_width > width {
                font_size -= 1;
                self.canvas.set_font(&format!("{}px {}", font_size, cfg.font));
                text_width = self.canvas.measure_text(text).expect("measure text").width();
            }

            // Senter horizontally
            draw_pos = draw_pos + ((width-text_width)/2.0, 0).into();
        }

        draw_pos = draw_pos + cfg.offset;

        let draw_fn: Box<dyn Fn(&str, f64, f64)>;
        if cfg.stroke {
            draw_fn = Box::new(|text, xpos, ypos| {
                self.canvas.stroke_text(text, xpos, ypos).expect("text");
            });
        }
        else {
            draw_fn = Box::new(|text, xpos, ypos| {
                self.canvas.fill_text(text, xpos, ypos).expect("text");
            });
        }
    
        let mut drawn = false;
        if cfg.is_command {
            if self.entered_keywords.iter().find(|x| *x == text).is_some() {
                self.canvas.set_fill_style_str(
                    &format!("rgb({},{},{})", self.keyword_r.cur() as i32, self.keyword_g.cur() as i32, self.keyword_b.cur() as i32));
                self.canvas.set_font(&format!("bold {}px {}", font_size, cfg.font));
                draw_fn(text, draw_pos.x, draw_pos.y);
                drawn = true;
            }
            else if !self.entered_keywords.is_empty() &&
                    text.starts_with(self.entered_keywords.last().unwrap())
            {
                let last_keyword = self.entered_keywords.last().unwrap();
                // Underline the matching part of the word
                self.canvas.set_font(&format!("italic {}px {}", font_size, cfg.font));
                draw_fn(last_keyword, draw_pos.x, draw_pos.y);

                let underlined_width = self.canvas.measure_text(last_keyword).expect("measure text");
                //let new_x = draw_pos.xpos + underlined_width.actual_bounding_box_left() + underlined_width.actual_bounding_box_right();
                let new_x = draw_pos.x + underlined_width.width();

                self.canvas.set_font(&format!("{}px {}", font_size, cfg.font));
                draw_fn(&text[last_keyword.len()..], new_x, draw_pos.y);
                drawn = true;
            }
        }

        if !drawn {
            self.canvas.set_font(&format!("{}px {}", font_size, cfg.font));
            draw_fn(text, draw_pos.x, draw_pos.y);
        }

        self.canvas.set_global_alpha(1.0);
    }

    pub fn update_config(&mut self, cfg_ui_images: &ImagesConfig) {
        self.images.update_config(cfg_ui_images);
    }

    pub fn entered_keywords(&mut self) -> &mut Vec<String>{
        &mut self.entered_keywords
    }

    pub fn images<'a>(&'a self) -> &'a Images {
        &self.images
    }

    pub fn canvas(&self) -> &OffscreenCanvasRenderingContext2d {
        &self.canvas
    }
}