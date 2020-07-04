
use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_renderer::{
    concurrent::{
        rayon::RayonExecutor,
        scene_proxy::SceneProxy
    },
    gpu::{
        options::{DestFramebuffer, RendererOptions, RendererMode},
        renderer::Renderer
    },
    scene::Scene,
    options::{BuildOptions}
};
use pathfinder_resources::embedded::EmbeddedResourceLoader;
use pathfinder_geometry::{
    vector::{Vector2F, Vector2I},
    rect::RectF
};
use pathfinder_color::ColorF;

use glutin::{GlRequest, Api, WindowedContext, PossiblyCurrent};
use winit::{
    event_loop::EventLoop,
    window::WindowBuilder,
    dpi::{LogicalSize, PhysicalSize},
};
use gl;
use crate::Config;

pub fn scroll_factors() -> (Vector2F, Vector2F) {
    // pixel factor           line delta factor
    (Vector2F::new(1.0, 1.0), Vector2F::new(10.0, -10.0))
}

pub struct GlWindow {
    windowed_context: WindowedContext<PossiblyCurrent>,
    proxy: SceneProxy,
    renderer: Renderer<GLDevice>,
    framebuffer_size: Vector2I,
    window_size: Vector2F,
}
impl GlWindow {
    pub fn new<T>(event_loop: &EventLoop<T>, title: String, window_size: Vector2F, config: &Config) -> Self {
        let window_builder = WindowBuilder::new()
            .with_title(title)
            .with_decorations(config.borders)
            .with_inner_size(PhysicalSize::new(window_size.x() as f64, window_size.y() as f64))
            .with_transparent(config.transparent);

        let windowed_context = glutin::ContextBuilder::new()
            .with_gl(GlRequest::Specific(Api::OpenGlEs, (3, 2)))
            .build_windowed(window_builder, &event_loop)
            .unwrap();
        
        let windowed_context = unsafe {
            windowed_context.make_current().unwrap()
        };

        gl::load_with(|ptr| windowed_context.get_proc_address(ptr));
        
        let dpi = windowed_context.window().scale_factor() as f32;
        let proxy = SceneProxy::new(config.render_level, RayonExecutor);
        let framebuffer_size = (window_size * dpi).to_i32();
        // Create a Pathfinder renderer.
        let render_mode = RendererMode { level: config.render_level };
        let render_options = RendererOptions {
            dest:  DestFramebuffer::full_window(framebuffer_size),
            background_color: Some(config.background),
            show_debug_ui: true,
        };

        let renderer = Renderer::new(GLDevice::new(GLVersion::GLES3, 0),
            &EmbeddedResourceLoader,
            render_mode,
            render_options,
        );

        GlWindow {
            windowed_context,
            proxy,
            renderer,
            framebuffer_size,
            window_size,
        }
    }
    pub fn render(&mut self, scene: Scene, options: BuildOptions) {
        self.proxy.replace_scene(scene);
        self.proxy.set_view_box(RectF::new(Vector2F::default(), self.framebuffer_size().to_f32()));

        self.proxy.build_and_render(&mut self.renderer, options);
        self.windowed_context.swap_buffers().unwrap();
    }
    
    pub fn resize(&mut self, size: Vector2F) {
        if size != self.window_size {
            let window = self.windowed_context.window();
            window.set_inner_size(PhysicalSize::new(size.x() as u32, size.y() as u32));
            window.request_redraw();
            self.window_size = size;
        }
    }
    // size changed, update GL context
    pub fn resized(&mut self, size: Vector2F) {
        let new_framebuffer_size = size.to_i32();
        if new_framebuffer_size != self.framebuffer_size {
            self.framebuffer_size = new_framebuffer_size;
            self.windowed_context.resize(PhysicalSize::new(self.framebuffer_size.x() as u32, self.framebuffer_size.y() as u32));
            self.renderer.set_main_framebuffer_size(self.framebuffer_size);
        }
    }
    pub fn scale_factor(&self) -> f32 {
        self.windowed_context.window().scale_factor() as f32
    }
    pub fn request_redraw(&self) {
        self.windowed_context.window().request_redraw();
    }
    pub fn framebuffer_size(&self) -> Vector2I {
        self.framebuffer_size
    }
}