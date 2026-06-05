pub mod app;
pub mod components;
pub mod editor;
pub mod actions;
pub mod indexed_db;

pub fn run() {
    yew::Renderer::<app::App>::new().render();
}
