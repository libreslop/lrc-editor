pub mod app;
pub mod audio;
pub mod components;
pub mod editor;

pub fn run() {
    yew::Renderer::<app::App>::new().render();
}
