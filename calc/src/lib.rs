#![cfg_attr(
    feature = "spirv",
    feature(register_attr),
    register_attr(spirv),
    no_std
)]

#[cfg(feature = "spirv")]
use core::prelude::rust_2021::*;
#[cfg(feature = "spirv")]
use spirv_std::num_traits::Float;

#[cfg(not(feature = "spirv"))]
use core::fmt::Display;
use core::ops::{Add, Mul};
#[cfg(not(feature = "spirv"))]
use core::str::FromStr;

#[cfg_attr(not(feature = "spirv"), derive(Debug))]
#[derive(Clone, PartialEq)]
pub struct Config {
    pub algo: Algo,
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
    pub color_weight: f64,
    pub julia_set: Imaginary,
}
impl Config {
    pub fn new(algo: Algo) -> Self {
        Self {
            width: 2000,
            height: 1000,
            iterations: if let Algo::BarnsleyFern = algo {
                10_000_000
            } else {
                50
            },
            limit: 2.0_f64.powi(16),
            stable_limit: 2.0,
            pos: Imaginary::ZERO,
            scale: Imaginary::ONE * 0.4,
            exposure: 2.0,
            inside: true,
            smooth: true,
            primary_color: if let Algo::BarnsleyFern = algo {
                RGB::new(4, 100, 3)
            } else {
                RGB::new(40, 40, 255)
            },
            secondary_color: if let Algo::BarnsleyFern = algo {
                RGB::new(240, 240, 240)
            } else {
                RGB::new(240, 170, 0)
            },
            color_weight: 0.01,
            julia_set: Imaginary::ZERO,
            algo,
        }
    }
}
impl Default for Config {
    fn default() -> Self {
        Self::new(Algo::Mandelbrot)
    }
}

#[cfg_attr(not(feature = "spirv"), derive(Debug))]
#[derive(Clone, Copy, PartialEq)]
pub struct Imaginary {
    pub re: f64,
    pub im: f64,
}
impl Imaginary {
    pub const ZERO: Self = Self { re: 0.0, im: 0.0 };
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
    #[inline(always)]
    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            re: self.re * rhs,
            im: self.im * rhs,
        }
    }
}

#[cfg_attr(not(feature = "spirv"), derive(Debug))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RGB {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
impl RGB {
    const BLACK: Self = Self::new(0, 0, 0);
    #[inline(always)]
    pub const fn new(r: u8, b: u8, g: u8) -> Self {
        Self { r, g, b }
    }
}
fn color_multiply(color: RGB, mult: f64) -> RGB {
    RGB::new(
        (color.r as f64 * mult) as u8,
        (color.g as f64 * mult) as u8,
        (color.b as f64 * mult) as u8,
    )
}

#[cfg_attr(not(feature = "spirv"), derive(Debug))]
#[derive(Clone, PartialEq)]
pub enum Algo {
    Mandelbrot,
    BarnsleyFern,
    Julia,
}
pub enum AlgoParseError {
    /// Use one of the variants.
    Incorrect,
}
#[cfg(not(feature = "spirv"))]
impl Display for AlgoParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "invalid algorithm name")
    }
}
#[cfg(not(feature = "spirv"))]
impl FromStr for Algo {
    type Err = AlgoParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(if s.eq_ignore_ascii_case("mandelbrot") {
            Self::Mandelbrot
        } else if s.eq_ignore_ascii_case("fern") || s.eq_ignore_ascii_case("barnsleyfern") {
            Self::BarnsleyFern
        } else if s.eq_ignore_ascii_case("julia") {
            Self::Julia
        } else {
            return Err(AlgoParseError::Incorrect);
        })
    }
}

#[inline(always)]
fn coord_to_space(coord: f64, max: f64, offset: f64, pos: f64, scale: f64) -> f64 {
    ((coord / max) - offset) / scale + pos
}
#[inline(always)]
fn xy_to_imaginary(
    x: u32,
    y: u32,
    width: f64,
    height: f64,
    pos: &Imaginary,
    scale: &Imaginary,
) -> Imaginary {
    let re = coord_to_space(x as f64, height, (width / height) / 2.0, pos.re, scale.re);
    let im = coord_to_space(y as f64, height, 0.5, pos.im, scale.im);
    Imaginary { re, im }
}

pub fn get_recursive_pixel(config: &Config, x: u32, y: u32) -> RGB {
    let start = xy_to_imaginary(
        x,
        y,
        config.width as f64,
        config.height as f64,
        &config.pos,
        &config.scale,
    );
    let (pos, iters) = match config.algo {
        Algo::Mandelbrot => recursive(config.iterations, start, start, config.limit),
        Algo::Julia => recursive(config.iterations, start, config.julia_set, config.limit),
        _ => return RGB::BLACK,
    };

    let dist = pos.squared_distance();

    if dist > config.stable_limit {
        let mut iters = iters as f64;

        if config.smooth {
            // https://en.wikipedia.org/wiki/Plotting_algorithms_for_the_Mandelbrot_set#Continuous_(smooth)_coloring

            let log_zn = f64::log2(dist.sqrt()) / 2.0;
            let nu = f64::log2(log_zn);

            iters += 1.0 - nu;
        }

        let mult = iters as f64 / config.iterations as f64 * config.exposure;
        color_multiply(config.primary_color, mult)
    } else if config.inside {
        color_multiply(config.secondary_color, dist)
    } else {
        RGB::BLACK
    }
}

/// `limit` is distance from center considered out of bounds.
///
/// If `c == start`, this is a Mandelbrot set. If `c` is constant, it's a Julia set.
///
/// # Return
///
/// Returns the final position and the number of iterations to get there.
#[inline(always)]
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
