use web_sys::{
    Window, MouseEvent, WheelEvent, KeyboardEvent, UiEvent,
    HtmlCanvasElement, WebGl2RenderingContext, Event,
    InputEvent,
};
use js_sys::{Function, Uint8Array};
use wasm_bindgen::{prelude::*, JsCast};
use crate::*;
use pathfinder_geometry::vector::{Vector2F, Vector2I, vec2f};
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_renderer::{
    scene::{Scene, SceneSink},
    gpu::{
        renderer::Renderer,
        options::{DestFramebuffer, RendererOptions, RendererMode},
    },
    concurrent::executor::SequentialExecutor,
    options::{BuildOptions, RenderTransform, RenderCommandListener},
};
use pathfinder_webgl::WebGlDevice;
use std::marker::PhantomData;
use crate::util::round_v_to_16;

pub struct Emitter<T>(PhantomData<T>);

pub struct Backend {}
impl Backend {
    pub fn resize(&mut self, size: Vector2F) {}
    pub fn get_scroll_factors(&self) -> (Vector2F, Vector2F) {
        (
            Vector2F::new(1.0, 1.0),
            Vector2F::new(10.0, -10.0),
        )
    }
    pub fn set_icon(&mut self, icon: Icon) {}
}

#[wasm_bindgen]
pub struct WasmView {
    item: Box<dyn Interactive<Event=Vec<u8>>>,
    ctx: Context,
    window: Window,
    renderer: Renderer<WebGlDevice>,
    framebuffer_size: Vector2F,
    canvas: HtmlCanvasElement,
}

impl WasmView {
    pub fn new(canvas: HtmlCanvasElement, context: WebGl2RenderingContext, config: Config, mut item: Box<dyn Interactive<Event=Vec<u8>>>) -> Self {
        canvas.set_attribute("tabindex", "0").unwrap();
        canvas.set_attribute("contenteditable", "true").unwrap();

        let window = web_sys::window().unwrap();
        let scale_factor = scale_factor(&window);
        let backend = Backend {};
        let mut ctx = Context::new(config, backend);
        ctx.set_scale_factor(scale_factor);

        // figure out the framebuffer, as that can only be integer values
        let framebuffer_size = v_ceil(item.window_size_hint().unwrap_or(vec2f(100., 100.)));
        
        // then figure out the css size
        ctx.window_size = framebuffer_size * (1.0 / ctx.scale_factor);

        set_canvas_size(&canvas, ctx.window_size, framebuffer_size.to_i32());

        let render_mode = RendererMode { level: ctx.config.render_level };
        let render_options = RendererOptions {
            dest:  DestFramebuffer::full_window(framebuffer_size.to_i32()),
            background_color: Some(ctx.config.background),
            show_debug_ui: false,
        };

        let renderer = Renderer::new(WebGlDevice::new(context),
            &*ctx.config.resource_loader,
            render_mode,
            render_options,
        );

        item.init(&mut ctx, Emitter(PhantomData));

        WasmView {
            item,
            ctx,
            window,
            renderer,
            canvas,
            framebuffer_size,
        }
    }
}

fn v_ceil(v: Vector2F) -> Vector2F {
    Vector2F::new(v.x().ceil(), v.y().ceil())
}

#[wasm_bindgen]
impl WasmView {
    pub fn render(&mut self) {
        let mut scene = self.item.scene(&mut self.ctx);
        let scene_view_box = view_box(&scene);

        // figure out the framebuffer, as that can only be integer values
        let framebuffer_size = v_ceil(scene_view_box.size());
        
        // then figure out the css size
        self.ctx.window_size = framebuffer_size * (1.0 / self.ctx.scale_factor);
        
        if framebuffer_size != self.framebuffer_size {
            set_canvas_size(&self.canvas, self.ctx.window_size, framebuffer_size.to_i32());
            self.renderer.options_mut().dest = DestFramebuffer::full_window(framebuffer_size.to_i32());
            self.framebuffer_size = framebuffer_size;
        }

        // temp fix
        scene.set_view_box(RectF::new(Vector2F::default(), round_v_to_16(framebuffer_size.to_i32()).to_f32()));
        
        let tr = if self.ctx.config.pan {
            Transform2F::from_translation(self.ctx.window_size * 0.5) *
            Transform2F::from_translation(-self.ctx.view_center)
        } else {
            Transform2F::from_translation(-scene_view_box.origin())
        };
        let options = BuildOptions {
            transform: RenderTransform::Transform2D(tr),
            dilation: Vector2F::default(),
            subpixel_aa_enabled: false
        };

        scene.build_and_render(&mut self.renderer, options, SequentialExecutor);
        self.ctx.redraw_requested = false;
    }
    pub fn animation_frame(&mut self, timestamp: f64) {
        self.render();
    }

    pub fn mouse_move(&mut self, event: &MouseEvent) -> bool {
        false
    }

    pub fn mouse_down(&mut self, event: &MouseEvent) -> bool {
        self.mouse_input(event, ElementState::Pressed);
        self.ctx.redraw_requested
    }
    pub fn mouse_up(&mut self, event: &MouseEvent) -> bool {
        self.mouse_input(event, ElementState::Released);
        self.ctx.redraw_requested
    }

    fn mouse_input(&mut self, event: &MouseEvent, state: ElementState) {
        let css_pos = Vector2F::new(event.offset_x() as f32, event.offset_y() as f32);

        let scale = 1.0 / self.ctx.scale;
        let tr = if self.ctx.config.pan {
            Transform2F::from_translation(self.ctx.view_center) *
            Transform2F::from_scale(Vector2F::splat(scale)) *
            Transform2F::from_translation(self.ctx.window_size * (-0.5 * self.ctx.scale_factor))
        } else {
            Transform2F::from_scale(Vector2F::splat(scale))
        };

        let scene_pos = tr * css_pos;
        let page = self.ctx.page_nr;
        self.item.mouse_input(&mut self.ctx, page, scene_pos, state);
    }

    pub fn wheel(&mut self, event: &WheelEvent) -> bool {
        self.ctx.redraw_requested
    }

    pub fn key_down(&mut self, event: &KeyboardEvent) -> bool {
        self.keyboard_input(event, ElementState::Pressed);
        self.ctx.redraw_requested

    }
    pub fn key_up(&mut self, event: &KeyboardEvent) -> bool {
        self.keyboard_input(event, ElementState::Released);
        self.ctx.redraw_requested
    }

    fn keyboard_input(&mut self, event: &KeyboardEvent, state: ElementState) {
        let keycode = match virtual_key_code(&event) {
            Some(keycode) => keycode,
            None => return,
        };
        let mut key_event = KeyEvent {
            cancelled: false,
            modifiers: keyboard_modifiers(&event),
            state,
            keycode
        };

        self.item.keyboard_input(&mut self.ctx, &mut key_event);

        if key_event.cancelled {
            cancel(&event);
        }
    }

    pub fn resize(&mut self, event: &UiEvent) -> bool {
        self.ctx.set_scale_factor(scale_factor(&self.window));
        self.ctx.request_redraw();
        self.ctx.redraw_requested
    }

    pub fn data(&mut self, data: &Uint8Array) -> bool {
        self.item.event(&mut self.ctx, data.to_vec());
        self.ctx.redraw_requested
    }
    pub fn idle(&mut self) -> bool {
        self.item.idle(&mut self.ctx);
        self.ctx.redraw_requested
    }
    pub fn input(&mut self, text: String) -> bool {
        self.item.text_input(&mut self.ctx, text);
        self.ctx.redraw_requested
    }
}

fn cancel(event: impl AsRef<Event>) {
    event.as_ref().prevent_default();
}

fn set_canvas_size(canvas: &HtmlCanvasElement, css_size: Vector2F, framebuffer_size: Vector2I) {
    canvas.set_width(framebuffer_size.x() as u32);
    canvas.set_height(framebuffer_size.y() as u32);

    let style = canvas.style();
    style
        .set_property("width", &format!("{}px", css_size.x()))
        .expect("Failed to set canvas width");
    style
        .set_property("height", &format!("{}px", css_size.y()))
        .expect("Failed to set canvas height");
}

pub fn scale_factor(window: &Window) -> f32 {
    window.device_pixel_ratio() as f32
}

pub fn window_size(window: &Window) -> Vector2F {
    let width = window
        .inner_width().unwrap()
        .as_f64().unwrap();
    
    let height = window
        .inner_height().unwrap()
        .as_f64().unwrap();

    Vector2F::new(width as f32, height as f32)
}

pub fn mouse_modifiers(event: &MouseEvent) -> Modifiers {
    Modifiers {
        shift: event.shift_key(),
        ctrl: event.ctrl_key(),
        alt: event.alt_key(),
        meta: event.meta_key()
    }
}

pub fn virtual_key_code(event: &KeyboardEvent) -> Option<KeyCode> {
    Some(match &event.code()[..] {
        "Digit1" => KeyCode::Key1,
        "Digit2" => KeyCode::Key2,
        "Digit3" => KeyCode::Key3,
        "Digit4" => KeyCode::Key4,
        "Digit5" => KeyCode::Key5,
        "Digit6" => KeyCode::Key6,
        "Digit7" => KeyCode::Key7,
        "Digit8" => KeyCode::Key8,
        "Digit9" => KeyCode::Key9,
        "Digit0" => KeyCode::Key0,
        "KeyA" => KeyCode::A,
        "KeyB" => KeyCode::B,
        "KeyC" => KeyCode::C,
        "KeyD" => KeyCode::D,
        "KeyE" => KeyCode::E,
        "KeyF" => KeyCode::F,
        "KeyG" => KeyCode::G,
        "KeyH" => KeyCode::H,
        "KeyI" => KeyCode::I,
        "KeyJ" => KeyCode::J,
        "KeyK" => KeyCode::K,
        "KeyL" => KeyCode::L,
        "KeyM" => KeyCode::M,
        "KeyN" => KeyCode::N,
        "KeyO" => KeyCode::O,
        "KeyP" => KeyCode::P,
        "KeyQ" => KeyCode::Q,
        "KeyR" => KeyCode::R,
        "KeyS" => KeyCode::S,
        "KeyT" => KeyCode::T,
        "KeyU" => KeyCode::U,
        "KeyV" => KeyCode::V,
        "KeyW" => KeyCode::W,
        "KeyX" => KeyCode::X,
        "KeyY" => KeyCode::Y,
        "KeyZ" => KeyCode::Z,
        "Escape" => KeyCode::Escape,
        "F1" => KeyCode::F1,
        "F2" => KeyCode::F2,
        "F3" => KeyCode::F3,
        "F4" => KeyCode::F4,
        "F5" => KeyCode::F5,
        "F6" => KeyCode::F6,
        "F7" => KeyCode::F7,
        "F8" => KeyCode::F8,
        "F9" => KeyCode::F9,
        "F10" => KeyCode::F10,
        "F11" => KeyCode::F11,
        "F12" => KeyCode::F12,
        "F13" => KeyCode::F13,
        "F14" => KeyCode::F14,
        "F15" => KeyCode::F15,
        "F16" => KeyCode::F16,
        "F17" => KeyCode::F17,
        "F18" => KeyCode::F18,
        "F19" => KeyCode::F19,
        "F20" => KeyCode::F20,
        "F21" => KeyCode::F21,
        "F22" => KeyCode::F22,
        "F23" => KeyCode::F23,
        "F24" => KeyCode::F24,
        "PrintScreen" => KeyCode::Snapshot,
        "ScrollLock" => KeyCode::Scroll,
        "Pause" => KeyCode::Pause,
        "Insert" => KeyCode::Insert,
        "Home" => KeyCode::Home,
        "Delete" => KeyCode::Delete,
        "End" => KeyCode::End,
        "PageDown" => KeyCode::PageDown,
        "PageUp" => KeyCode::PageUp,
        "ArrowLeft" => KeyCode::Left,
        "ArrowUp" => KeyCode::Up,
        "ArrowRight" => KeyCode::Right,
        "ArrowDown" => KeyCode::Down,
        "Backspace" => KeyCode::Back,
        "Enter" => KeyCode::Return,
        "Space" => KeyCode::Space,
        "Compose" => KeyCode::Compose,
        "Caret" => KeyCode::Caret,
        "NumLock" => KeyCode::Numlock,
        "Numpad0" => KeyCode::Numpad0,
        "Numpad1" => KeyCode::Numpad1,
        "Numpad2" => KeyCode::Numpad2,
        "Numpad3" => KeyCode::Numpad3,
        "Numpad4" => KeyCode::Numpad4,
        "Numpad5" => KeyCode::Numpad5,
        "Numpad6" => KeyCode::Numpad6,
        "Numpad7" => KeyCode::Numpad7,
        "Numpad8" => KeyCode::Numpad8,
        "Numpad9" => KeyCode::Numpad9,
        "AbntC1" => KeyCode::AbntC1,
        "AbntC2" => KeyCode::AbntC2,
        "NumpadAdd" => KeyCode::NumpadAdd,
        "Quote" => KeyCode::Apostrophe,
        "Apps" => KeyCode::Apps,
        "At" => KeyCode::At,
        "Ax" => KeyCode::Ax,
        "Backslash" => KeyCode::Backslash,
        "Calculator" => KeyCode::Calculator,
        "Capital" => KeyCode::Capital,
        "Semicolon" => KeyCode::Semicolon,
        "Comma" => KeyCode::Comma,
        "Convert" => KeyCode::Convert,
        "NumpadDecimal" => KeyCode::NumpadDecimal,
        "NumpadDivide" => KeyCode::NumpadDivide,
        "Equal" => KeyCode::Equals,
        "Backquote" => KeyCode::Grave,
        "Kana" => KeyCode::Kana,
        "Kanji" => KeyCode::Kanji,
        "AltLeft" => KeyCode::LAlt,
        "BracketLeft" => KeyCode::LBracket,
        "ControlLeft" => KeyCode::LControl,
        "ShiftLeft" => KeyCode::LShift,
        "MetaLeft" => KeyCode::LWin,
        "Mail" => KeyCode::Mail,
        "MediaSelect" => KeyCode::MediaSelect,
        "MediaStop" => KeyCode::MediaStop,
        "Minus" => KeyCode::Minus,
        "NumpadMultiply" => KeyCode::NumpadMultiply,
        "Mute" => KeyCode::Mute,
        "LaunchMyComputer" => KeyCode::MyComputer,
        "NavigateForward" => KeyCode::NavigateForward,
        "NavigateBackward" => KeyCode::NavigateBackward,
        "NextTrack" => KeyCode::NextTrack,
        "NoConvert" => KeyCode::NoConvert,
        "NumpadComma" => KeyCode::NumpadComma,
        "NumpadEnter" => KeyCode::NumpadEnter,
        "NumpadEquals" => KeyCode::NumpadEquals,
        "OEM102" => KeyCode::OEM102,
        "Period" => KeyCode::Period,
        "PlayPause" => KeyCode::PlayPause,
        "Power" => KeyCode::Power,
        "PrevTrack" => KeyCode::PrevTrack,
        "AltRight" => KeyCode::RAlt,
        "BracketRight" => KeyCode::RBracket,
        "ControlRight" => KeyCode::RControl,
        "ShiftRight" => KeyCode::RShift,
        "MetaRight" => KeyCode::RWin,
        "Slash" => KeyCode::Slash,
        "Sleep" => KeyCode::Sleep,
        "Stop" => KeyCode::Stop,
        "NumpadSubtract" => KeyCode::NumpadSubtract,
        "Sysrq" => KeyCode::Sysrq,
        "Tab" => KeyCode::Tab,
        "Underline" => KeyCode::Underline,
        "Unlabeled" => KeyCode::Unlabeled,
        "AudioVolumeDown" => KeyCode::VolumeDown,
        "AudioVolumeUp" => KeyCode::VolumeUp,
        "Wake" => KeyCode::Wake,
        "WebBack" => KeyCode::WebBack,
        "WebFavorites" => KeyCode::WebFavorites,
        "WebForward" => KeyCode::WebForward,
        "WebHome" => KeyCode::WebHome,
        "WebRefresh" => KeyCode::WebRefresh,
        "WebSearch" => KeyCode::WebSearch,
        "WebStop" => KeyCode::WebStop,
        "Yen" => KeyCode::Yen,
        _ => return None,
    })
}

pub fn keyboard_modifiers(event: &KeyboardEvent) -> Modifiers {
    Modifiers {
        shift: event.shift_key(),
        ctrl: event.ctrl_key(),
        alt: event.alt_key(),
        meta: event.meta_key()
    }
}
