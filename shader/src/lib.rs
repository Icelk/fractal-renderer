#![cfg_attr(
    target_arch = "spirv",
    feature(register_attr),
    register_attr(spirv),
    no_std
)]
extern crate spirv_std;

use glam::UVec3;
use spirv_std::glam;
#[cfg(not(target_arch = "spirv"))]
use spirv_std::macros::spirv;

#[spirv(compute(threads(64)))]
pub fn main_cs(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)]
    buffer: &mut [fractal_renderer_calc::RGBF],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)]
    config: &fractal_renderer_calc::InnerConfig,
) {
    let index = id.x as usize;
    // let y = index / config.width;
    // let x = index - (y * config.width);
    let x = buffer[index].r;
    let y = buffer[index].g;
    let pixel = fractal_renderer_calc::get_recursive_pixel(config, x, y);
    buffer[index] = pixel;
}
