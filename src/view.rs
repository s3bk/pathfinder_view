use pathfinder_geometry::vector::{Vector2F};
use pathfinder_renderer::scene::Scene;

use serde::{Serialize, Deserialize};
use crate::*;

#[derive(Serialize, Deserialize, Default)]
pub struct State {
    scale: f32,
    window_size: Option<(f32, f32)>,
    view_center: Option<(f32, f32)>,
    page_nr: usize
}


pub trait Interactive: 'static {
    fn scene(&mut self, nr: usize) -> Scene;
    fn num_pages(&self) -> usize;

    fn char_input(&mut self, ctx: &mut Context, input: char) {}
    fn keyboard_input(&mut self, ctx: &mut Context, event: &mut KeyEvent) {
        match (event.state, event.modifiers.ctrl, event.keycode) {
            (ElementState::Pressed, false, KeyCode::PageDown) => ctx.next_page(),
            (ElementState::Pressed, false, KeyCode::PageUp) => ctx.prev_page(),
            (ElementState::Pressed, true, KeyCode::Add) => ctx.zoom_by(0.2),
            (ElementState::Pressed, true, KeyCode::Subtract) => ctx.zoom_by(-0.2),
            (ElementState::Pressed, true, KeyCode::Key0) => ctx.set_zoom(DEFAULT_SCALE),
            _ => return
        }
        event.cancel();
    }
    fn mouse_input(&mut self, ctx: &mut Context, page: usize, pos: Vector2F, state: ElementState) {}
    fn exit(&mut self, ctx: &mut Context) {}
    fn title(&self) -> String { "A fantastic window!".into() }
    fn event(&mut self, ctx: &mut Context, event: Vec<u8>) {}
    fn init(&mut self, ctx: &mut Context) {}
    fn idle(&mut self, ctx: &mut Context) {}
}

fn check_scene(scene: Scene) -> Scene {
    let s = scene.view_box().size();
    if s.x() < 0. {
        warn!("scene has a negative width");
    }
    if s.y() < 0. {
        warn!("scene has a negative height");
    }
    scene
}


impl Interactive for Scene {
    fn scene(&mut self, _: usize) -> Scene {
        self.clone()
    }
    fn num_pages(&self) -> usize {
        1
    }
}
