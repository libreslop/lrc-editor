pub mod app;
pub mod audio;
pub mod components;

pub fn run() {
    yew::Renderer::<app::App>::new().render();
}
