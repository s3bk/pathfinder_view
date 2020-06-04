struct App;

impl pathfinder_view::Interactive for App {
    fn scene(&mut self, nr: usize) -> pathfinder_renderer::scene::Scene {
        pathfinder_renderer::scene::Scene::new()
    }

    fn num_pages(&self) -> usize {
        1
    }
}

fn main() {
    pathfinder_view::show(App, pathfinder_view::Config {
        zoom: true,
        pan: true,
    });
}
