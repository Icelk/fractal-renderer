#![cfg_attr(
    target_arch = "spirv",
    feature(register_attr),
    register_attr(spirv),
    no_std
)]
extern crate spirv_std;

use glam::{UVec3, Vec3};
use spirv_std::glam;
#[cfg(not(target_arch = "spirv"))]
use spirv_std::macros::spirv;

use core::ops::{Add, Mul};
use spirv_std::num_traits::Float;

#[spirv(compute(threads(64)))]
pub fn main_cs(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] buffer: &mut [fractal_renderer_calc::RGBF],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)]
    config: &fractal_renderer_calc::InnerConfig,
) {
    let index = id.x;
    // let y = index / config.width;
    // let x = index - (y * config.width);
    let x = buffer[index as usize].r;
    let y = buffer[index as usize].g;
    let pixel = fractal_renderer_calc::get_recursive_pixel(config, x, y);
    buffer[index as usize] = pixel;
}
