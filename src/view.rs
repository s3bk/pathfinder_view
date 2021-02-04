use pathfinder_geometry::vector::{Vector2F};
use pathfinder_renderer::scene::Scene;
use std::fmt::Debug;
use crate::*;

pub trait Interactive: 'static {
    type Event: Debug + Send + 'static = ();

    fn scene(&mut self, ctx: &mut Context) -> Scene;

    fn char_input(&mut self, ctx: &mut Context, input: char) {}
    fn text_input(&mut self, ctx: &mut Context, input: String) {
        for c in input.chars() {
            self.char_input(ctx, c);
        }
    }
    fn keyboard_input(&mut self, ctx: &mut Context, event: &mut KeyEvent) {
        match (event.state, event.modifiers.ctrl, event.keycode) {
            (ElementState::Pressed, false, KeyCode::PageDown) => ctx.next_page(),
            (ElementState::Pressed, false, KeyCode::PageUp) => ctx.prev_page(),
            (ElementState::Pressed, true, KeyCode::Plus) => ctx.zoom_by(0.2),
            (ElementState::Pressed, true, KeyCode::Minus) => ctx.zoom_by(-0.2),
            (ElementState::Pressed, true, KeyCode::Key0) => ctx.set_zoom(DEFAULT_SCALE),
            _ => return
        }
        event.cancel();
    }
    fn mouse_input(&mut self, ctx: &mut Context, page: usize, pos: Vector2F, state: ElementState) {}
    fn exit(&mut self, ctx: &mut Context) {}
    fn title(&self) -> String { "A fantastic window!".into() }
    fn event(&mut self, ctx: &mut Context, event: Self::Event) {}
    fn init(&mut self, ctx: &mut Context, sender: Emitter<Self::Event>) {}
    fn idle(&mut self, ctx: &mut Context) {}
    fn window_size_hint(&self) -> Option<Vector2F> { None }
}

impl Interactive for Scene {
    fn init(&mut self, ctx: &mut Context, sender: Emitter<Self::Event>) {
        ctx.set_view_box(self.view_box());
    }
    fn scene(&mut self, ctx: &mut Context) -> Scene {
        self.clone()
    }
    fn window_size_hint(&self) -> Option<Vector2F> {
        let size = self.view_box().size();
        if size.is_zero() {
            None
        } else {
            Some(size)
        }
    }
}
