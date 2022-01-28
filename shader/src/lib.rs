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

use spirv_std::num_traits::Float;
use core::ops::{Mul, Add};

#[spirv(compute(threads(64)))]
pub fn main_cs(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] buffer: &mut [Vec3],
) {
}

pub enum Algo {
    Mandelbrot,
    Julia(Imaginary),
}
pub struct Config {
    pub width: u32,
    pub height: u32,
    pub iterations: u32,
    pub limit: f64,
    pub stable_limit: f64,
    pub pos: Imaginary,
    pub scale: Imaginary,
    pub exposure: f64,
    pub inside: bool,
    pub smooth: bool,
    pub primary_color: RGB,
    pub secondary_color: RGB,
    pub algo: Algo,
    pub color_weight: f64,
}

const BLACK: RGB = RGB::new(0, 0, 0);

pub struct RGB {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
impl RGB {
    pub const fn new(r: u8, b: u8, g: u8) -> Self {
        Self { r,g,b }
    }
}
pub struct Imaginary {
    pub re: f64,
    pub im: f64,
}
impl Imaginary {
    const ZERO: Self = Self { re: 0.0, im: 0.0 };
    const ONE: Self = Self { re: 1.0, im: 1.0 };
    #[inline(always)]
    pub fn square(self) -> Self {
        let re = (self.re * self.re) - (self.im * self.im);
        let im = 2.0 * self.re * self.im;

        Self { re, im }
    }
    #[inline(always)]
    pub fn squared_distance(self) -> f64 {
        self.re * self.re + self.im * self.im
    }
}
impl Add for Imaginary {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            re: self.re + rhs.re,
            im: self.im + rhs.im,
        }
    }
}
impl Mul<f64> for Imaginary {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            re: self.re * rhs,
            im: self.im * rhs,
        }
    }
}

pub fn get_recursive_pixel(config: &Config, x: u32, y: u32) -> RGB {
    fn unreachable() -> ! {
        panic!("called get_recursive_pixel when algo wasn't a recursive pixel one.")
    }

    let start = xy_to_imaginary(
        x,
        y,
        config.width as f64,
        config.height as f64,
        &config.pos,
        &config.scale,
    );
    let (mandelbrot, iters) = match config.algo {
        Algo::Mandelbrot => recursive(config.iterations, start, start, config.limit),
        Algo::Julia(c) => recursive(config.iterations, start, c, config.limit),
        _ => unreachable(),
    };

    let dist = mandelbrot.squared_distance();

    if dist > config.stable_limit {
        let mut iters = iters as f64;

        if config.smooth {
            // https://en.wikipedia.org/wiki/Plotting_algorithms_for_the_Mandelbrot_set#Continuous_(smooth)_coloring

            let log_zn = (f64::log2(dist.sqrt())) / 2.0;
            let nu = f64::log2(log_zn);

            iters += 1.0 - nu;
        }

        let mult = iters as f64 / config.iterations as f64 * config.exposure;
        color_multiply(config.primary_color, mult)
    } else if config.inside {
        color_multiply(config.secondary_color, dist)
    } else {
        BLACK
    }
}

fn color_multiply(color: RGB, mult: f64) -> RGB {
    RGB::new(
        (color.r as f64 * mult) as u8,
        (color.g as f64 * mult) as u8,
        (color.b as f64 * mult) as u8,
    )
}
pub fn recursive(iterations: u32, start: Imaginary, c: Imaginary, limit: f64) -> (Imaginary, u32) {
    let squared = limit * limit;
    let mut previous = start;
    for i in 0..iterations {
        let next = previous.square() + c;
        let dist = next.squared_distance();
        if dist > squared {
            return (next, i);
        }
        previous = next;
    }
    (previous, iterations)
}
