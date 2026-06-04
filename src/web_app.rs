pub mod app;
pub mod components;
pub mod editor;
pub mod actions;

pub fn run() {
    yew::Renderer::<app::App>::new().render();
}
