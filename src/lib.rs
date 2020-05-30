#![feature(associated_type_defaults)]

#[macro_use] extern crate log;
pub mod view;

pub use view::Interactive;

#[cfg(target_os="linux")]
pub mod gl;

#[cfg(not(target_arch="wasm32"))]
mod show;

#[cfg(not(target_arch="wasm32"))]
pub use show::show;

#[cfg(target_arch="wasm32")]
pub mod wasm;

#[cfg(target_arch="wasm32")]
pub use wasm::WasmView;

use pathfinder_geometry::{
    vector::{Vector2F},
    rect::RectF
};
use pathfinder_color::ColorF;
use pathfinder_renderer::scene::Scene;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ElementState {
    Pressed,
    Released
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool,
}

#[derive(Default, Debug)]
pub struct Config {
    pub zoom: bool,
    pub pan:  bool
}

pub type Emitter = Box<dyn Fn(Vec<u8>) + Send>;

pub struct Context {
    // we need to keep two different redraws apart:
    // - the scene needs to be regenerated
    pub (crate) update_scene: bool,
    // - the window needs a repaint
    pub (crate) redraw_requested: bool,
    pub (crate) page_nr: usize,
    pub (crate) num_pages: usize,
    pub (crate) scale: f32, // device independend
    pub (crate) view_center: Vector2F,
    pub (crate) window_size: Vector2F,
    pub (crate) background_color: ColorF,
    pub (crate) scale_factor: f32, // device dependend
    pub (crate) config: Config,
    pub (crate) emitter: Option<Emitter>
}

const DEFAULT_SCALE: f32 = 96.0 / 25.4;
impl Context {
    pub fn new(config: Config) -> Self {
        Context {
            redraw_requested: true,
            update_scene: true,
            num_pages: 1,
            page_nr: 0,
            scale: DEFAULT_SCALE,
            background_color: ColorF::new(0.0, 0.0, 0.0, 0.0),
            scale_factor: 1.0,
            config,
            view_center: Vector2F::default(),
            window_size: Vector2F::default(),
            emitter: None
        }
    }
    pub (crate) fn request_redraw(&mut self) {
        self.redraw_requested = true;
    }
    pub fn update_scene(&mut self) {
        self.update_scene = true;
        self.redraw_requested = true;
    }
    pub fn goto_page(&mut self, page: usize) {
        let page = page.min(self.num_pages - 1);
        if page != self.page_nr {
            self.page_nr = page;
            self.update_scene();
        }
    }
    pub fn next_page(&mut self) {
        self.goto_page(self.page_nr.saturating_add(1));
    }
    pub fn prev_page(&mut self) {
        self.goto_page(self.page_nr.saturating_sub(1));
    }
    pub fn page_nr(&self) -> usize {
        self.page_nr
    }
    pub fn zoom_by(&mut self, log2_factor: f32) {
        self.scale *= 2f32.powf(log2_factor);
        self.request_redraw();
    }
    pub fn set_zoom(&mut self, factor: f32) {
        if factor != self.scale {
            self.scale = factor;
            self.request_redraw();
        }
    }

    pub fn move_by(&mut self, delta: Vector2F) {
        self.move_to(self.view_center + delta);
    }

    pub fn move_to(&mut self, point: Vector2F) {
        self.view_center = point;
        self.request_redraw();
    }

    pub (crate) fn set_scale_factor(&mut self, factor: f32) {
        self.scale_factor = factor;
        self.request_redraw();
    }

    #[cfg(target_arch = "wasm32")]
    pub fn send(&mut self, data: Vec<u8>) {}

    /// can only be called once. will return None afterwards
    pub fn take_emitter(&mut self) -> Option<Emitter> {
        self.emitter.take()
    }
}

pub struct KeyEvent {
    pub (crate) cancelled: bool,
    pub state: ElementState,
    pub keycode: KeyCode,
    pub modifiers: Modifiers
}
impl KeyEvent {
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
}
macro_rules! keycodes {
    ($( $(#[$meta:meta])? $key:ident,)*) => (
        #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
        pub enum KeyCode { $( $(#[$meta])? $key,)* }

        #[cfg(not(target_arch="wasm32"))]
        impl From<winit::event::VirtualKeyCode> for KeyCode {
            fn from(c: winit::event::VirtualKeyCode) -> Self {
                match c {
                    $(winit::event::VirtualKeyCode::$key => KeyCode::$key,)*
                }
            }
        }
    )
}

// borrowed from winit…
keycodes!{
    /// The '1' key over the letters.
    Key1,
    /// The '2' key over the letters.
    Key2,
    /// The '3' key over the letters.
    Key3,
    /// The '4' key over the letters.
    Key4,
    /// The '5' key over the letters.
    Key5,
    /// The '6' key over the letters.
    Key6,
    /// The '7' key over the letters.
    Key7,
    /// The '8' key over the letters.
    Key8,
    /// The '9' key over the letters.
    Key9,
    /// The '0' key over the 'O' and 'P' keys.
    Key0,

    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    /// The Escape key, next to F1.
    Escape,

    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,

    /// Print Screen/SysRq.
    Snapshot,
    /// Scroll Lock.
    Scroll,
    /// Pause/Break key, next to Scroll lock.
    Pause,

    /// `Insert`, next to Backspace.
    Insert,
    Home,
    Delete,
    End,
    PageDown,
    PageUp,

    Left,
    Up,
    Right,
    Down,

    /// The Backspace key, right over Enter.
    // TODO: rename
    Back,
    /// The Enter key.
    Return,
    /// The space bar.
    Space,

    /// The "Compose" key on Linux.
    Compose,

    Caret,

    Numlock,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,

    AbntC1,
    AbntC2,
    Add,
    Apostrophe,
    Apps,
    At,
    Ax,
    Backslash,
    Calculator,
    Capital,
    Colon,
    Comma,
    Convert,
    Decimal,
    Divide,
    Equals,
    Grave,
    Kana,
    Kanji,
    LAlt,
    LBracket,
    LControl,
    LShift,
    LWin,
    Mail,
    MediaSelect,
    MediaStop,
    Minus,
    Multiply,
    Mute,
    MyComputer,
    NavigateForward,  // also called "Prior"
    NavigateBackward, // also called "Next"
    NextTrack,
    NoConvert,
    NumpadComma,
    NumpadEnter,
    NumpadEquals,
    OEM102,
    Period,
    PlayPause,
    Power,
    PrevTrack,
    RAlt,
    RBracket,
    RControl,
    RShift,
    RWin,
    Semicolon,
    Slash,
    Sleep,
    Stop,
    Subtract,
    Sysrq,
    Tab,
    Underline,
    Unlabeled,
    VolumeDown,
    VolumeUp,
    Wake,
    WebBack,
    WebFavorites,
    WebForward,
    WebHome,
    WebRefresh,
    WebSearch,
    WebStop,
    Yen,
    Copy,
    Paste,
    Cut,
}

fn view_box(scene: &Scene) -> RectF {
    let view_box = scene.view_box();
    if view_box == RectF::default() {
        scene.bounds()
    } else {
        view_box
    }
}
