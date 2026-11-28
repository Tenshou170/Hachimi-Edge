pub mod render_hook;
pub mod input;
mod d3d11_backup;
mod d3d11_painter;
mod smaa;

pub fn init() {
    render_hook::init();
}