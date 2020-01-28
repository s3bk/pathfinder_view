
use pathfinder_geometry::vector::{Vector2F, Vector2I};
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_renderer::scene::Scene;
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererOptions};
use pathfinder_renderer::options::{BuildOptions, RenderTransform};
use winit::{
    event::{Event, WindowEvent, DeviceEvent, KeyboardInput, ElementState, VirtualKeyCode, MouseButton, MouseScrollDelta, ModifiersState },
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    dpi::{LogicalSize, LogicalPosition, PhysicalSize, PhysicalPosition},
};

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Default)]
pub struct State {
    scale: f32,
    window_size: Option<(f32, f32)>,
    view_center: Option<(f32, f32)>
}

pub trait Interactive: 'static {
    fn scene(&mut self) -> Scene;
    fn char_input(&mut self, input: char) -> bool {
        false
    }
    fn keyboard_input(&mut self, state: ElementState, keycode: VirtualKeyCode) -> bool {
        false
    }
    fn mouse_input(&mut self, pos: Vector2F, state: ElementState) -> bool {
        false
    }
    fn exit(&mut self) {}
    fn title(&self) -> String { "A fantastic window!".into() }

    fn load_state(&self) -> Option<State> {
        None
    }
    fn save_state(&self, state: State) {}
}

pub fn show(mut item: impl Interactive) {
    info!("creating event loop");
    let event_loop = EventLoop::new();

    let mut scale = 96.0 / 25.4;
    // (150px / inch) * (1inch / 25.4mm) = 150px / 25.mm

    let scene = item.scene();
    let view_box = scene.view_box();
    
    let mut window_size = view_box.size().scale(scale);

    if let Some(state) = item.load_state() {
        scale = state.scale;
    }

    info!("creating window");

    #[cfg(target_arch="wasm32")]
    let mut window = crate::webgl::WebGlWindow::new(&event_loop, "canvas", window_size);

    #[cfg(target_os="linux")]
    let mut window = crate::gl::GlWindow::new(&event_loop, item.title(), window_size);

    let mut dpi = window.scale_factor();
    let mut cursor_pos = Vector2F::default();

    let mut modifiers = ModifiersState::empty();

    info!("entering the event loop");
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::RedrawRequested(_) => {
                let physical_size = window.framebuffer_size().to_f32();
                debug!("physical_size = {:?}", physical_size);
                let scene = item.scene();

                let tr = Transform2F::from_scale(Vector2F::splat(dpi * scale));
                let options = BuildOptions {
                    transform: RenderTransform::Transform2D(tr),
                    dilation: Vector2F::default(),
                    subpixel_aa_enabled: false
                };

                window.render(scene, options);
            },
            Event::DeviceEvent { event, .. } => match event {
                DeviceEvent::ModifiersChanged(new_modifiers) => {
                    modifiers = new_modifiers;
                },
                _ => {}
            }
            Event::WindowEvent { event, .. } =>  {
                match event {
                    WindowEvent::CursorMoved { .. } => {},
                    _ => info!("event: {:?}", event)
                }

                let mut needs_redraw = false;

                match event {
                    WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                        dpi = scale_factor as f32;
                        window_size = view_box.size().scale(scale);
                        window.resize(window_size.scale(dpi));
                        needs_redraw = true;
                    }
                    WindowEvent::KeyboardInput { input: KeyboardInput { state, virtual_keycode: Some(keycode), .. }, ..  } => {
                        needs_redraw |= item.keyboard_input(state, keycode);
                    }
                    WindowEvent::ReceivedCharacter(c) => needs_redraw |= item.char_input(c),
                    WindowEvent::CursorMoved { position: PhysicalPosition { x, y }, .. } => {
                        let new_pos = Vector2F::new(x as f32, y as f32);
                        let cursor_delta = new_pos - cursor_pos;
                        cursor_pos = new_pos;
                    },
                    WindowEvent::MouseInput { button: MouseButton::Left, state, .. } => {
                        let scene_pos = cursor_pos.scale(1.0 / (dpi * scale));
                        needs_redraw |= item.mouse_input(scene_pos, state);
                    }
                    WindowEvent::CloseRequested => {
                        println!("The close button was pressed; stopping");
                        *control_flow = ControlFlow::Exit
                    },
                    _ => {}
                }
                if needs_redraw {
                    window.request_redraw();
                }
            }
            Event::LoopDestroyed => {
                let state = State {
                    scale,
                    .. State::default()
                };
                item.save_state(state);
                item.exit();
            }
            _ => {}
        }
    });
}

#[cfg(feature="pan")]
pub fn show_pan(mut item: impl Interactive) {
    info!("creating event loop");
    let event_loop = EventLoop::new();

    let mut scale = 96.0 / 25.4;
    // (150px / inch) * (1inch / 25.4mm) = 150px / 25.mm

    let scene = item.scene();
    let view_box = scene.view_box();
    
    let mut view_center = view_box.origin() + view_box.size().scale(0.5);

    let mut window_size = view_box.size().scale(scale);

    if let Some(state) = item.load_state() {
        scale = state.scale;
        if let Some((w, h)) = state.window_size {
            window_size = Vector2F::new(w, h);
        }
        if let Some((x, y)) = state.view_center {
            view_center = Vector2F::new(x, y);
        }
    }

    info!("creating window");

    #[cfg(target_arch="wasm32")]
    let mut window = crate::webgl::WebGlWindow::new(&event_loop, "canvas", window_size);

    #[cfg(target_os="linux")]
    let mut window = crate::gl::GlWindow::new(&event_loop, item.title(), window_size);

    let mut dpi = window.scale_factor();
    let mut cursor_pos = Vector2F::default();
    let mut dragging = false;

    let mut modifiers = ModifiersState::empty();

    info!("entering the event loop");
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::RedrawRequested(_) => {
                let physical_size = window.framebuffer_size().to_f32();
                debug!("physical_size = {:?}", physical_size);
                let scene = item.scene();

                let tr = Transform2F::from_translation(physical_size.scale(0.5)) *
                    Transform2F::from_scale(Vector2F::splat(dpi * scale)) *
                    Transform2F::from_translation(-view_center);
                
                let options = BuildOptions {
                    transform: RenderTransform::Transform2D(tr),
                    dilation: Vector2F::default(),
                    subpixel_aa_enabled: false
                };

                window.render(scene, options);
            },
            Event::DeviceEvent { event, .. } => match event {
                DeviceEvent::ModifiersChanged(new_modifiers) => {
                    modifiers = new_modifiers;
                },
                _ => {}
            }
            Event::WindowEvent { event, .. } =>  {
                match event {
                    WindowEvent::CursorMoved { .. } => {},
                    _ => info!("event: {:?}", event)
                }

                let mut needs_redraw = false;

                match event {
                    WindowEvent::ScaleFactorChanged { scale_factor, new_inner_size: PhysicalSize { width, height } } => {
                        dpi = scale_factor as f32;
                        let physical_size = Vector2F::new(width as f32, height as f32);
                        window.resize(physical_size);
                        window_size = physical_size.scale(1.0 / dpi);
                        needs_redraw = true;
                    }
                    WindowEvent::Resized(PhysicalSize {width, height}) => {
                        let physical_size = Vector2F::new(width as f32, height as f32);
                        window.resize(physical_size);
                        window_size = physical_size.scale(1.0 / dpi);
                        needs_redraw = true;
                    }
                    WindowEvent::KeyboardInput { input: KeyboardInput { state, virtual_keycode: Some(keycode), .. }, ..  } => {
                        needs_redraw |= item.keyboard_input(state, keycode);
                    }
                    WindowEvent::ReceivedCharacter(c) => needs_redraw |= item.char_input(c),
                    WindowEvent::CursorMoved { position: PhysicalPosition { x, y }, .. } => {
                        let new_pos = Vector2F::new(x as f32, y as f32);
                        let cursor_delta = new_pos - cursor_pos;
                        cursor_pos = new_pos;

                        if dragging {
                                view_center = view_center - cursor_delta.scale(1.0 / (scale * dpi));
                                needs_redraw = true;
                        }
                    },
                    WindowEvent::MouseInput { button: MouseButton::Left, state, .. } => {
                        match (state, modifiers.shift()) {
                            (ElementState::Pressed, true) => dragging = true,
                            (ElementState::Released, _) if dragging => dragging = false,

                            _ => {
                                let scene_pos = 
                                Transform2F::from_translation(view_center) *
                                Transform2F::from_scale(Vector2F::splat(1.0 / (dpi * scale))) *
                                Transform2F::from_translation(window_size.scale(-0.5 * dpi)) *
                                cursor_pos;
                                needs_redraw |= item.mouse_input(scene_pos, state);
                            }
                        }
                    }
                    WindowEvent::MouseWheel { delta, modifiers, .. } => {
                        let delta = match delta {
                            MouseScrollDelta::PixelDelta(LogicalPosition { x: dx, y: dy }) => Vector2F::new(dx as f32, dy as f32),
                            MouseScrollDelta::LineDelta(dx, dy) => Vector2F::new(dx as f32, -dy as f32).scale(10.)
                        };
                        if modifiers.ctrl() {
                            scale *= (-0.02 * delta.y()).exp();
                            needs_redraw = true;
                        } else {
                            view_center = view_center - delta.scale(1.0 / scale);
                            needs_redraw = true;
                        }
                    }
                    WindowEvent::CloseRequested => {
                        println!("The close button was pressed; stopping");
                        *control_flow = ControlFlow::Exit
                    },
                    _ => {}
                }
                if needs_redraw {
                    window.request_redraw();
                }
            }
            Event::LoopDestroyed => {
                let state = State {
                    scale,
                    window_size: Some((window_size.x(), window_size.y())),
                    view_center: Some((view_center.x(), view_center.y()))
                };
                item.save_state(state);
                item.exit();
            }
            _ => {}
        }
    });
}

impl Interactive for Scene {
    fn scene(&mut self) -> Scene {
        self.clone()
    }
}
