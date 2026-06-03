#[cfg(target_arch = "wasm32")]
fn main() {
    lcs_editor::run();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("Run `trunk build --release`, then serve the generated static files.");
}
