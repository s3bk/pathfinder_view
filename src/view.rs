use pathfinder_geometry::vector::{Vector2F};
use pathfinder_renderer::scene::Scene;
use winit::{event::{ElementState, KeyEvent}, keyboard::ModifiersState};
use std::fmt::Debug;
use crate::*;

pub trait Interactive: 'static {
    type Event: Debug + Send + 'static;

    fn scene(&mut self, ctx: &mut Context) -> Scene;

    fn char_input(&mut self, ctx: &mut Context, input: char) {}
    fn text_input(&mut self, ctx: &mut Context, input: String) {
        for c in input.chars() {
            self.char_input(ctx, c);
        }
    }
    fn keyboard_input(&mut self, ctx: &mut Context, modifiers: ModifiersState, event: KeyEvent) {
        match (event.state, modifiers.control_key(), event.physical_key) {
            (ElementState::Pressed, false, KeyCode::PageDown) => ctx.next_page(),
            (ElementState::Pressed, false, KeyCode::PageUp) => ctx.prev_page(),
            (ElementState::Pressed, true, KeyCode::Digit1) => ctx.zoom_by(0.2),
            (ElementState::Pressed, true, KeyCode::Digit2) => ctx.zoom_by(-0.2),
            (ElementState::Pressed, true, KeyCode::Digit0) => ctx.set_zoom(DEFAULT_SCALE),
            _ => return
        }
    }
    fn mouse_input(&mut self, ctx: &mut Context, page: usize, pos: Vector2F, state: ElementState) {}
    fn cursor_moved(&mut self, ctx: &mut Context, pos: Vector2F) {}
    fn exit(&mut self, ctx: &mut Context) {}
    fn title(&self) -> String { "A fantastic window!".into() }
    fn event(&mut self, ctx: &mut Context, event: Self::Event) {}
    fn init(&mut self, ctx: &mut Context, sender: Emitter<Self::Event>) {}
    fn idle(&mut self, ctx: &mut Context) {}
    fn window_size_hint(&self) -> Option<Vector2F> { None }
}

impl Interactive for Scene {
    type Event = ();
    
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
