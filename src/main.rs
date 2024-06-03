pub mod core;
pub mod image;
pub mod math;
pub mod color_mapping;
pub mod stitch_images;

fn main() {
    stitch_images::make();
}
