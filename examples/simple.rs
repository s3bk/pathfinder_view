use pathfinder_renderer::{
    scene::{Scene, DrawPath},
    paint::Paint,
};
use pathfinder_content::{
    outline::{Outline, Contour},
};
use pathfinder_geometry::{
    rect::RectF,
    vector::{Vector2F, vec2f},
};
use pathfinder_color::ColorU;
use pathfinder_view::{show, Config};
use pathfinder_resources::embedded::EmbeddedResourceLoader;

fn main() {
    env_logger::init();

    let mut scene = Scene::new();
    scene.set_view_box(RectF::from_points(vec2f(0., 0.), vec2f(100., 100.)));

    let mut outline = Outline::new();
    let contour = Contour::from_rect_rounded(
        RectF::from_points(vec2f(10., 10.), vec2f(90., 90.)),
        vec2f(10., 10.)
    );
    outline.push_contour(contour);

    let paint = Paint::from_color(ColorU::new(200, 100, 200, 255));
    let paint_id = scene.push_paint(&paint);
    scene.push_draw_path(DrawPath::new(outline, paint_id));

    let mut config = Config::new(Box::new(EmbeddedResourceLoader));
    config.pan = true;
    show(scene, config);
}
