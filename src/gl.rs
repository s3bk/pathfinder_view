
use std::{ffi::CStr, num::NonZeroU32};

use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_renderer::{
    concurrent::{
        rayon::RayonExecutor,
        scene_proxy::SceneProxy,
        executor::SequentialExecutor,
    },
    gpu::{
        options::{DestFramebuffer, RendererOptions, RendererMode, RendererLevel},
        renderer::Renderer
    },
    scene::Scene,
    options::{BuildOptions}
};
use pathfinder_geometry::{
    vector::{Vector2F, Vector2I},
    rect::RectF
};

use glutin::{context::{ContextApi, Version, PossiblyCurrentContext}, config::{ConfigTemplate, ConfigTemplateBuilder, Api}, prelude::{GlConfig, GlDisplay, NotCurrentGlContextSurfaceAccessor}, display::{GetGlDisplay, Display}, surface::{GlSurface, Surface, WindowSurface}};
use winit::{
    event_loop::EventLoop,
    window::{WindowBuilder, Window},
    dpi::{PhysicalSize},
};
use gl;
use crate::Config;
use crate::util::round_v_to_16;
use glutin_winit::{DisplayBuilder, GlWindow as GlutinGlWindow};
use raw_window_handle::HasRawWindowHandle;

pub struct GlWindow {
    gl_context: PossiblyCurrentContext,
    gl_surface: Surface<WindowSurface>,
    proxy: SceneProxy,
    renderer: Renderer<GLDevice>,
    framebuffer_size: Vector2I,
    window_size: Vector2F,
    window: Window,
}
impl GlWindow {
    pub fn new<T>(event_loop: &EventLoop<T>, title: String, window_size: Vector2F, config: &Config) -> Self {
        let window_builder = WindowBuilder::new()
            .with_title(title)
            .with_decorations(config.borders)
            .with_inner_size(PhysicalSize::new(window_size.x() as f64, window_size.y() as f64))
            .with_transparent(config.transparent);

        let (glutin_gl_version, renderer_gl_version, api) = match config.render_level {
            RendererLevel::D3D9 => (Version::new(3, 0), GLVersion::GLES3, Api::GLES3),
            RendererLevel::D3D11 => (Version::new(4, 3), GLVersion::GL4, Api::OPENGL),
        };
        let template_builder = ConfigTemplateBuilder::new().with_alpha_size(8).with_api(api);
        let display_builder = DisplayBuilder::new().with_window_builder(Some(window_builder));
        let (mut window, gl_config) = display_builder.build(event_loop, template_builder, |configs| {
            configs
            .reduce(|accum, config| {
                let transparency_check = config.supports_transparency().unwrap_or(false)
                    & !accum.supports_transparency().unwrap_or(false);

                if transparency_check || config.num_samples() > accum.num_samples() {
                    config
                } else {
                    accum
                }
            })
            .unwrap()
        }).unwrap();
        let mut window = window.unwrap();

        let raw_window_handle = window.raw_window_handle();

        let gl_display = gl_config.display();
        
        let context_attributes = glutin::context::ContextAttributesBuilder::new()
            .build(Some(raw_window_handle));
        
        let attrs = window.build_surface_attributes(<_>::default());
        let gl_surface = unsafe {
            gl_config.display().create_window_surface(&gl_config, &attrs).unwrap()
        };

        let mut windowed_context = unsafe {
            gl_display.create_context(&gl_config, &context_attributes).expect("failed to create context")
        };
        let current_context = unsafe {
            windowed_context
            .make_current(&gl_surface)
            .unwrap()
        };

        gl::load_with(|ptr: &str| gl_display.get_proc_address(unsafe { CStr::from_ptr(ptr.as_ptr().cast()) }));
        
        let dpi = window.scale_factor() as f32;
        let proxy = match config.threads {
            true => SceneProxy::new(config.render_level, RayonExecutor),
            false => SceneProxy::new(config.render_level, SequentialExecutor)
        };
        let framebuffer_size = (window_size * dpi).to_i32();
        // Create a Pathfinder renderer.
        let render_mode = RendererMode { level: config.render_level };
        let render_options = RendererOptions {
            dest:  DestFramebuffer::full_window(framebuffer_size),
            background_color: Some(config.background),
            show_debug_ui: false,
        };


        let renderer = Renderer::new(GLDevice::new(renderer_gl_version, 0),
            &*config.resource_loader,
            render_mode,
            render_options,
        );

        GlWindow {
            gl_context: current_context,
            gl_surface,
            proxy,
            renderer,
            framebuffer_size,
            window_size,
            window,
        }
    }
    pub fn render(&mut self, mut scene: Scene, options: BuildOptions) {
        scene.set_view_box(RectF::new(Vector2F::default(), self.framebuffer_size.to_f32()));
        self.proxy.replace_scene(scene);

        self.proxy.build_and_render(&mut self.renderer, options);
        self.gl_surface.swap_buffers(&self.gl_context).unwrap();
    }
    
    pub fn resize(&mut self, size: Vector2F) {
        if size != self.window_size {
            self.window.set_inner_size(PhysicalSize::new(size.x() as u32, size.y() as u32));
            self.window.request_redraw();
            self.window_size = size;
        }
    }
    // size changed, update GL context
    pub fn resized(&mut self, size: Vector2F) {
        // pathfinder does not like scene sizes that are now a multiple of the tile size (16).
        let new_framebuffer_size = round_v_to_16(size.to_i32());
        if new_framebuffer_size != self.framebuffer_size {
            self.framebuffer_size = new_framebuffer_size;
            self.gl_surface.resize(&self.gl_context, NonZeroU32::new(self.framebuffer_size.x() as u32).unwrap(), NonZeroU32::new(self.framebuffer_size.y() as u32).unwrap());
            self.renderer.options_mut().dest = DestFramebuffer::full_window(new_framebuffer_size);
        }
    }
    pub fn scale_factor(&self) -> f32 {
        self.window.scale_factor() as f32
    }
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }
    pub fn framebuffer_size(&self) -> Vector2I {
        self.framebuffer_size
    }
    pub fn window(&self) -> &Window {
        &self.window
    }
}
