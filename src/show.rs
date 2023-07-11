
use winit::event::{Event, ElementState as WinitElementState, WindowEvent, MouseButton, MouseScrollDelta, StartCause};
use winit::event_loop::{ControlFlow, EventLoopProxy};
use winit::keyboard::{ModifiersState};
use winit::platform::{run_return::EventLoopExtRunReturn};
use winit::dpi::{PhysicalSize, PhysicalPosition};
use crate::view::{Interactive};
use crate::{Config, Context};
use crate::{Icon};
use pathfinder_geometry::vector::{Vector2F, vec2f};
use pathfinder_renderer::{
    options::{BuildOptions, RenderTransform},
};
use std::time::{Instant, Duration};

pub struct Emitter<E: 'static>(EventLoopProxy<E>);
impl<E: 'static> Emitter<E> {
    pub fn send(&self, event: E) {
        let _ = self.0.send_event(event);
    }
}
impl<E: 'static> Clone for Emitter<E> {
    fn clone(&self) -> Self {
        Emitter(self.0.clone())
    }
}
pub struct Backend {
    window: crate::gl::GlWindow,
}
impl Backend {
    pub fn new(window: crate::gl::GlWindow) -> Backend {
        Backend {
            window,
        }
    }
    pub fn resize(&mut self, size: Vector2F) {
        self.window.resize(size);
    }
    pub fn get_scroll_factors(&self) -> (Vector2F, Vector2F) {
        (
            env_vec("PIXEL_SCROLL_FACTOR").unwrap_or(Vector2F::new(1.0, 1.0)),
            env_vec("LINE_SCROLL_FACTOR").unwrap_or(Vector2F::new(10.0, -10.0)),
        )
    }
    pub fn set_icon(&mut self, icon: Icon) {
        self.window.window().set_window_icon(Some(winit::window::Icon::from_rgba(
            icon.data,
            icon.width,
            icon.height
        ).unwrap()));
    }
}
fn env_vec(name: &str) -> Option<Vector2F> {
    use tuple::{T2, Map, TupleElements};
    let val = std::env::var(name).ok()?;
    let t2 = T2::from_iter(val.splitn(2, ","))?;
    let T2(x, y) = t2.map(|s: &str| s.parse().ok()).collect()?;
    Some(Vector2F::new(x, y))
}

#[cfg(not(target_arch="wasm32"))]
pub fn show<T: Interactive>(mut item: T, config: Config) {
    use winit::{event_loop::EventLoopBuilder, event::{KeyEvent, Modifiers}};

    info!("creating event loop");
    let mut event_loop = EventLoopBuilder::with_user_event()
        .build();
    
    // EventLoop::<<T as Interactive>::Event>::with_user_event();

    let mut cursor_pos = Vector2F::default();
    let mut dragging = false;

    let window_size = item.window_size_hint().unwrap_or(vec2f(600., 400.));
    let window = crate::gl::GlWindow::new(&event_loop, item.title(), window_size, &config);
    let backend = Backend::new(window);
    let mut ctx = Context::new(config, backend);
    let scale_factor = ctx.backend.window.scale_factor();
    ctx.set_scale_factor(scale_factor);
    ctx.request_redraw();
    ctx.window_size = window_size;


    let proxy = event_loop.create_proxy();

    item.init(&mut ctx, Emitter(proxy));

    let mut modifiers = ModifiersState::default();
    info!("entering the event loop");
    event_loop.run_return(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::NewEvents(StartCause::Init) => {
            }
            Event::NewEvents(StartCause::ResumeTimeReached { start: _, requested_resume: _ }) => {
                ctx.request_redraw();
            }
            Event::RedrawRequested(_) => {
                let options = BuildOptions {
                    transform: RenderTransform::default(),
                    dilation: Vector2F::default(),
                    subpixel_aa_enabled: false
                };

                ctx.backend.window.resized(ctx.window_size);
                let scene = item.scene(&mut ctx);
                ctx.backend.window.render(scene, options);
                ctx.redraw_requested = false;
            },
            Event::UserEvent(e) => {
                item.event(&mut ctx, e);
            }
            Event::MainEventsCleared => item.idle(&mut ctx),
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::ScaleFactorChanged { scale_factor, new_inner_size: PhysicalSize { width, height } } => {
                        ctx.set_scale_factor(scale_factor as f32);
                        ctx.set_window_size(Vector2F::new(*width as f32, *height as f32));
                        *width = ctx.window_size.x().ceil() as u32;
                        *height = ctx.window_size.y().ceil() as u32;
                        ctx.request_redraw();
                    }
                    WindowEvent::Focused { ..} => ctx.request_redraw(),
                    WindowEvent::Resized(PhysicalSize {width, height}) => {
                        let physical_size = Vector2F::new(width as f32, height as f32);
                        ctx.window_size = physical_size;
                        ctx.check_bounds();
                        ctx.request_redraw();
                    }
                    WindowEvent::ModifiersChanged(new_modifiers) => {
                        modifiers = new_modifiers.state();
                    }
                    WindowEvent::KeyboardInput { event, ..  } => {
                        item.keyboard_input(&mut ctx, modifiers, event);
                    }
                    WindowEvent::CursorMoved { position: PhysicalPosition { x, y }, .. } => {
                        let new_pos = Vector2F::new(x as f32, y as f32);
                        let cursor_delta = new_pos - cursor_pos;
                        cursor_pos = new_pos;

                        if dragging {
                            ctx.move_by(cursor_delta * (-1.0 / ctx.scale));
                        } else {
                            item.cursor_moved(&mut ctx, new_pos);
                        }
                    },
                    WindowEvent::MouseInput { button: MouseButton::Left, state, .. } => {
                        match (state, modifiers.shift_key()) {
                            (WinitElementState::Pressed, true) if ctx.config.pan => dragging = true,
                            (WinitElementState::Released, _) if dragging => dragging = false,
                            _ => {
                                let page_nr = ctx.page_nr;
                                item.mouse_input(&mut ctx, page_nr, cursor_pos, state);
                            }
                        }
                    }
                    WindowEvent::MouseWheel { delta, .. } => {
                        let delta = match delta {
                            MouseScrollDelta::PixelDelta(PhysicalPosition { x: dx, y: dy }) => Vector2F::new(dx as f32, dy as f32) * ctx.pixel_scroll_factor,
                            MouseScrollDelta::LineDelta(dx, dy) => Vector2F::new(dx as f32, dy as f32) * ctx.line_scroll_factor,
                        };
                        if ctx.config.zoom && modifiers.control_key() {
                            ctx.zoom_by(-0.02 * delta.y());
                        } else if ctx.config.pan {
                            ctx.move_by(delta * (-1.0 / ctx.scale));
                        }
                    }
                    WindowEvent::CloseRequested => {
                        println!("The close button was pressed; stopping");
                        ctx.close();
                    },
                    _ => {}
                }
            }
            Event::LoopDestroyed => {
                item.exit(&mut ctx);
            }
            _ => {}
        }
        if ctx.redraw_requested {
            ctx.backend.window.request_redraw();
        }
        
        if let Some(dt) = ctx.update_interval {
            *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_secs_f32(dt));
        }
        if ctx.close {
            *control_flow = ControlFlow::Exit;
        }
    });
}