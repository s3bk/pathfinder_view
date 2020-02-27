
use winit::event::{Event, ElementState as WinitElementState, VirtualKeyCode, ModifiersState,
    DeviceEvent, WindowEvent, KeyboardInput, MouseButton, MouseScrollDelta};
use winit::event_loop::{EventLoop, ControlFlow};
use winit::dpi::{PhysicalSize, PhysicalPosition, LogicalPosition};
use crate::view::Interactive;
use crate::{ElementState, KeyEvent, KeyCode, Config, Modifiers, Context};
use pathfinder_geometry::vector::Vector2F;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_renderer::{
    options::{BuildOptions, RenderTransform},
};

impl From<WinitElementState> for ElementState {
    fn from(s: WinitElementState) -> ElementState {
        match s {
            WinitElementState::Pressed => ElementState::Pressed,
            WinitElementState::Released => ElementState::Released
        }
    }
}
impl From<ModifiersState> for Modifiers {
    fn from(m: ModifiersState) -> Modifiers {
        Modifiers {
            shift: m.shift(),
            ctrl: m.ctrl(),
            alt: m.alt(),
            meta: m.logo()
        }
    }
}

#[cfg(not(target_arch="wasm32"))]
pub fn show(mut item: impl Interactive, config: Config) {
    info!("creating event loop");
    let event_loop = EventLoop::with_user_event();


    let scroll_factors = crate::gl::scroll_factors();


    let mut cursor_pos = Vector2F::default();
    let mut dragging = false;

    let mut modifiers = ModifiersState::empty();

    let mut ctx = Context::new(config);
    ctx.request_redraw();
    ctx.num_pages = item.num_pages();

    let scene = item.scene(ctx.page_nr);
    let view_box = scene.view_box();
    ctx.view_center = view_box.origin() + view_box.size().scale(0.5);
    ctx.window_size = view_box.size().scale(ctx.scale);

    info!("creating window with {:?}", ctx.window_size);

    let mut window = crate::gl::GlWindow::new(&event_loop, item.title(), ctx.window_size);
    ctx.scale_factor = window.scale_factor();

    let proxy = event_loop.create_proxy();
    item.init(&mut ctx);

    info!("entering the event loop");
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::RedrawRequested(_) => {
                // clamp page, just in case
                let scene = item.scene(ctx.page_nr.min(item.num_pages() - 1));
                let physical_size = if ctx.config.pan {
                    window.framebuffer_size().to_f32()
                } else {
                    scene.view_box().size().scale(ctx.scale * ctx.scale_factor)
                };
                window.resize(physical_size);

                let tr = Transform2F::from_translation(physical_size.scale(0.5)) *
                    Transform2F::from_scale(Vector2F::splat(ctx.scale * ctx.scale_factor)) *
                    Transform2F::from_translation(-ctx.view_center);
                
                let options = BuildOptions {
                    transform: RenderTransform::Transform2D(tr),
                    dilation: Vector2F::default(),
                    subpixel_aa_enabled: false
                };

                window.render(scene, options);
                ctx.redraw_requested = false;
                ctx.update_scene = false;
            },
            Event::UserEvent(e) => {
                item.event(&mut ctx, e);
            }
            Event::MainEventsCleared => item.idle(&mut ctx),
            Event::DeviceEvent { event, .. } => match event {
                DeviceEvent::ModifiersChanged(new_modifiers) => {
                    modifiers = new_modifiers;
                },
                _ => {}
            }
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::ScaleFactorChanged { scale_factor, new_inner_size: &mut PhysicalSize { width, height } } => {
                        ctx.scale_factor = scale_factor as f32;
                        if ctx.config.pan {
                            let physical_size = Vector2F::new(width as f32, height as f32);
                            window.resize(physical_size);
                            ctx.window_size = physical_size.scale(1.0 / ctx.scale_factor);
                        } else {
                            let physical_size = ctx.window_size.scale(ctx.scale * ctx.scale_factor);
                            window.resize(physical_size);
                        }
                        ctx.request_redraw();
                    }
                    WindowEvent::Focused { ..} => ctx.request_redraw(),
                    WindowEvent::Resized(PhysicalSize {width, height}) if ctx.config.pan => {
                        let physical_size = Vector2F::new(width as f32, height as f32);
                        window.resize(physical_size);
                        ctx.window_size = physical_size.scale(1.0 / ctx.scale_factor);
                        ctx.request_redraw();
                    }
                    WindowEvent::KeyboardInput { input: KeyboardInput { state, virtual_keycode: Some(keycode), .. }, ..  } => {
                        let mut event = KeyEvent {
                            state: state.into(),
                            modifiers: modifiers.into(),
                            keycode: keycode.into(),
                            cancelled: false
                        };
                        item.keyboard_input(&mut ctx, &mut event);
                    }
                    WindowEvent::ReceivedCharacter(c) => item.char_input(&mut ctx, c),
                    WindowEvent::CursorMoved { position: PhysicalPosition { x, y }, .. } => {
                        let new_pos = Vector2F::new(x as f32, y as f32);
                        let cursor_delta = new_pos - cursor_pos;
                        cursor_pos = new_pos;

                        if dragging {
                            ctx.move_by(cursor_delta.scale(-1.0 / (ctx.scale * ctx.scale_factor)));
                        }
                    },
                    WindowEvent::MouseInput { button: MouseButton::Left, state, .. } => {
                        match (state, modifiers.shift()) {
                            (WinitElementState::Pressed, true) if ctx.config.pan => dragging = true,
                            (WinitElementState::Released, _) if dragging => dragging = false,
                            _ => {
                                let scene_pos = ctx.device_to_scene() * cursor_pos;
                                let page_nr = ctx.page_nr;
                                item.mouse_input(&mut ctx, page_nr, scene_pos, state.into());
                            }
                        }
                    }
                    WindowEvent::MouseWheel { delta, .. } => {
                        let (pixel_factor, line_factor) = scroll_factors;
                        let delta = match delta {
                            MouseScrollDelta::PixelDelta(LogicalPosition { x: dx, y: dy }) => Vector2F::new(dx as f32, dy as f32) * pixel_factor,
                            MouseScrollDelta::LineDelta(dx, dy) => Vector2F::new(dx as f32, dy as f32) * line_factor,
                        };
                        if ctx.config.zoom && modifiers.ctrl() {
                            ctx.zoom_by(-0.02 * delta.y());
                        } else if ctx.config.pan {
                            ctx.move_by(delta.scale(-1.0 / ctx.scale));
                        }
                    }
                    WindowEvent::CloseRequested => {
                        println!("The close button was pressed; stopping");
                        *control_flow = ControlFlow::Exit
                    },
                    _ => {}
                }
                if ctx.redraw_requested || ctx.update_scene {
                    window.request_redraw();
                }
            }
            Event::LoopDestroyed => {
                item.exit(&mut ctx);
            }
            _ => {}
        }
    });
}