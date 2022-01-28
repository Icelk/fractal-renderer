#[cfg(feature = "bin")]
use std::fmt::Display;
#[cfg(feature = "bin")]
use std::io::Write;
use core::ops::{Add, Mul};
use core::str::FromStr;
use core::prelude::rust_2021::*;

#[cfg(feature = "bin")]
use clap::{Arg, ArgGroup};
#[cfg(feature = "fern")]
use rand::{Rng, SeedableRng};
#[cfg(feature = "bin")]
use rayon::iter::{IntoParallelIterator, ParallelIterator};

#[cfg(feature = "gpu")]
#[path ="compute.rs"]
pub mod compute;
#[cfg(feature = "gui")]
#[path ="gui.rs"]
pub mod gui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RGB {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
impl RGB {
    pub const fn new(r: u8, b: u8, g: u8) -> Self {
        Self { r,g,b }
    }
    #[cfg(feature = "avif")]
    pub const fn transmute(me: &[Self]) -> &[ravif::RGB8] {
        unsafe { std::mem::transmute(me) }
    }
}
#[cfg(feature = "avif")]
impl From<RGB> for ravif::RGB8 {
    fn from(rgb: RGB) -> Self {
        Self::new(rgb.r, rgb.g, rgb.b)
    }
}

const BLACK: RGB = RGB::new(0, 0, 0);

#[derive(Debug, Clone, PartialEq)]
pub enum Algo {
    Mandelbrot,
    BarnsleyFern,
    Julia(Imaginary),
}
impl Algo {
    fn is_different(&self, other: &Self) -> bool {
        if let Self::Julia(_) = self {
            if let Self::Julia(_) = other {
                return false;
            }
        }
        !self.eq(other)
    }
}
pub enum AlgoParseError {
    /// Use one of the variants.
    Incorrect,
}
#[cfg(feature = "bin")]
impl Display for AlgoParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid algorithm name")
    }
}
impl FromStr for Algo {
    type Err = AlgoParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(if s.eq_ignore_ascii_case("mandelbrot") {
            Self::Mandelbrot
        } else if s.eq_ignore_ascii_case("fern") || s.eq_ignore_ascii_case("barnsleyfern") {
            Self::BarnsleyFern
        } else if s.eq_ignore_ascii_case("julia") {
            Self::Julia(Imaginary { re: 0.0, im: 0.0 })
        } else {
            return Err(AlgoParseError::Incorrect);
        })
    }
}

fn parse_hex_rgb(s: &str) -> RGB {
    let (p1, p2) = s.split_at(2);
    let (p2, p3) = p2.split_at(2);
    let r = u8::from_str_radix(p1, 16).expect("failed to parse hex color");
    let g = u8::from_str_radix(p2, 16).expect("failed to parse hex color");
    let b = u8::from_str_radix(p3, 16).expect("failed to parse hex color");
    RGB::new(r, g, b)
}

#[cfg(feature = "bin")]
pub fn get_config() -> Config {
    let app = clap::App::new("fractal-renderer")
        .about("Set `-d` for a more traditional look.")
        .arg(
            Arg::new("width")
                .help("Easily handles 100MP images.")
                .default_value("750"),
        )
        .arg(
            Arg::new("height")
                .help("Easily handles 100MP images.")
                .default_value("500"),
        )
        .arg(
            Arg::new("iterations")
                .long("iterations")
                .short('i')
                .takes_value(true)
                .help("Limit of iterations. Default is 50 for Mandelbrot & Julia and 10_000_000 for Fern.")
        )
        .arg(
            Arg::new("limit")
                .long("limit")
                .short('l')
                .help("Limit where vaules are treated to escape. Only applicable to Mandelbrot & Julia.")
                .takes_value(true)
                .default_value("65536"),
        )
        .arg(
            Arg::new("stable_limit")
                .long("stable-limit")
                .help("The limit of points considered inside the fractal. Only applicable to Mandelbrot & Julia.")
                .default_value("2"),
        )
        .arg(
            Arg::new("pos_x")
                .short('x')
                .takes_value(true)
                .default_value_if("algo", Some("julia"), Some("0"))
                .default_value("-0.6")
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new("pos_y")
                .short('y')
                .takes_value(true)
                .default_value("0")
                .allow_hyphen_values(true),
        )
        .arg(Arg::new("scale_y").long("scale-y").takes_value(true))
        .arg(Arg::new("scale_x").long("scale-x").takes_value(true))
        .group(
            ArgGroup::new("scale_individual")
                .arg("scale_x")
                .arg("scale_y"),
        )
        .arg(
            Arg::new("scale")
                .conflicts_with("scale_individual")
                .long("scale")
                .short('s')
                .takes_value(true)
                .default_value("0.4"),
        )
        .arg(
            Arg::new("exposure")
                .long("exposure")
                .short('e')
                .takes_value(true)
                .default_value("5"),
        )
        .arg(Arg::new("primary_color").long("primary-color").takes_value(true).help("The main color of output."))
        .arg(Arg::new("secondary_color").long("secondary-color").takes_value(true).help("The secondary color of output. Defaults to orange for Mandelbrot and Julia. Acts as the background color for the Fern."))
        .arg(
            Arg::new("disable_inside")
                .long("disable-inside")
                .short('d')
                .help("Makes the inside of fractals black."),
        )
        .arg(
            Arg::new("unsmooth")
                .long("unsmooth")
                .short('u')
                .help("Don't smooth the aliasing of the borders."),
        )
        .arg(
            Arg::new("filename")
                .long("output")
                .short('o')
                .takes_value(true)
                .default_value("output"),
        )
        .arg(
            Arg::new("open")
                .long("open")
                .help("Open the image after generation."),
        )
        .arg(
            Arg::new("algo")
                .long("algorithm")
                .short('a')
                .help("The algorithm to use.")
                .default_value("mandelbrot")
                .possible_value("mandelbrot")
                .possible_value("fern")
                .possible_value("julia").requires_if("julia", "julia_re").requires_if("julia", "julia_im"),
        )
        .arg(
            Arg::new("julia_re")
            .long("julia-real")
            .help("Real part of start point for Julia set.")
            .takes_value(true)
            .allow_hyphen_values(true),
        )
        .arg(
            Arg::new("julia_im")
            .long("julia-imaginary")
            .help("Imaginary part of start point for Julia set.")
            .takes_value(true)
            .allow_hyphen_values(true),
        )
        .arg(
            Arg::new("color_weight")
            .long("color-weight")
            .short('w')
            .help("How much 'opacity' each hit on the Fern has. Increase to get a darker fern.").default_value("0.01")
        )
        .arg(
            Arg::new("gui")
            .long("gui")
            .short('g')
            .help("Start the GUI. Requires the `gui` cargo feature.")
            .long_help("Use `s` to take a 2x screenshot. `m` hides the menybar. Use the arrow keys and scroll to move around the image.")
        );

    let matches = app.get_matches();

    let width = matches.value_of_t("width").unwrap();
    let height = matches.value_of_t("height").unwrap();
    let iterations = matches.value_of_t("iterations").ok();
    let pos = Imaginary {
        re: matches.value_of_t("pos_x").unwrap(),
        im: matches.value_of_t("pos_y").unwrap(),
    };
    let scale = Imaginary {
        re: matches
            .value_of_t("scale_x")
            .or_else(|_| matches.value_of_t("scale"))
            .unwrap(),
        im: matches
            .value_of_t("scale_y")
            .or_else(|_| matches.value_of_t("scale"))
            .unwrap(),
    };
    let limit = matches.value_of_t("limit").unwrap();
    let stable_limit = matches.value_of_t("stable_limit").unwrap();
    let exposure: f64 = matches.value_of_t("exposure").unwrap();
    let primary_color = matches.value_of("primary_color").map(parse_hex_rgb);
    let secondary_color = matches.value_of("secondary_color").map(parse_hex_rgb);
    let inside_disabled = matches.is_present("disable_inside");
    let unsmooth = matches.is_present("unsmooth");
    let filename = matches
        .value_of("filename")
        .map(|f| format!("{}.avif", f))
        .unwrap();
    let open = matches.is_present("open");
    let mut algo = matches.value_of_t("algo").unwrap();
    if let Algo::Julia(start) = &mut algo {
        start.re = matches.value_of_t("julia_re").unwrap();
        start.im = matches.value_of_t("julia_im").unwrap();
    }
    let color_weight = matches.value_of_t("color_weight").unwrap();
    let gui = matches.is_present("gui");
    if gui && cfg!(not(feature = "gui")) {
        eprintln!("The gui feature isn't enabled! Remove the GUI argument.");
    }

    Config {
        width,
        height,
        iterations,
        limit,
        stable_limit,
        pos,
        scale,
        exposure,
        inside: !inside_disabled,
        smooth: !unsmooth,
        primary_color,
        secondary_color,
        open,
        filename,
        algo,
        color_weight,

        gui,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub width: u32,
    pub height: u32,
    pub iterations: Option<u32>,
    pub limit: f64,
    pub stable_limit: f64,
    pub pos: Imaginary,
    pub scale: Imaginary,
    pub exposure: f64,
    pub inside: bool,
    pub smooth: bool,
    pub primary_color: Option<RGB>,
    pub secondary_color: Option<RGB>,
    pub filename: String,
    pub open: bool,
    pub algo: Algo,
    pub color_weight: f64,

    pub gui: bool,
}
impl Config {
    fn iterations(&self) -> u32 {
        if let Some(iters) = self.iterations {
            return iters;
        }
        match self.algo {
            Algo::Mandelbrot | Algo::Julia(_) => 50,
            Algo::BarnsleyFern => 10_000_000,
        }
    }
    fn primary_color(&self) -> RGB {
        if let Some(color) = self.primary_color {
            return color;
        }

        match self.algo {
            Algo::Mandelbrot | Algo::Julia(_) => RGB::new(40, 40, 255),
            Algo::BarnsleyFern => RGB::new(4, 100, 3),
        }
    }
    fn secondary_color(&self) -> RGB {
        if let Some(color) = self.secondary_color {
            return color;
        }

        match self.algo {
            Algo::Mandelbrot | Algo::Julia(_) => RGB::new(240, 170, 0),
            Algo::BarnsleyFern => RGB::new(240, 240, 240),
        }
    }
}
impl Default for Config {
    fn default() -> Self {
        Self {
            width: 2000,
            height: 1000,
            iterations: None,
            limit: 2.0_f64.powi(16),
            stable_limit: 2.0,
            pos: Imaginary::ZERO,
            scale: Imaginary::ONE * 0.4,
            exposure: 2.0,
            inside: true,
            smooth: true,
            primary_color: None,
            secondary_color: None,
            filename: "output.avif".to_owned(),
            open: false,
            algo: Algo::Mandelbrot,
            color_weight: 0.01,

            gui: false,
        }
    }
}

#[cfg(feature = "avif")]
pub fn image_to_data(image: Image, image_config: &ravif::Config, config: &Config) -> Vec<u8> {
    println!("Starting encode.");
    let (data, _) = ravif::encode_rgb(image.into(), image_config).expect("encoding failed");
    println!("Finished encode. Writing file {:?}.", config.filename);
    data
}

pub fn get_image(config: &Config) -> Vec<RGB> {
    match config.algo {
        Algo::Mandelbrot | Algo::Julia(_) => {
            #[cfg(feature = "gpu")]
            {
                compute::start();
            }

            #[cfg(all(not(feature = "gpu"), feature = "bin"))]
            {
            let image: Vec<_> = (0..config.height)
                // Only one parallell iter, else, it'd be less efficient.
                .into_par_iter()
                .map(|y| {
                    let mut row = Vec::with_capacity(config.width as usize);
                    for x in 0..config.width {
                        row.push(get_recursive_pixel(config, x, y))
                    }
                    row
                })
                .flatten()
                .collect();

            image
            }
        }
        Algo::BarnsleyFern => {
            let mut contents =
                vec![config.secondary_color(); (config.width * config.height) as usize];

            let mut image =
                Image::new(&mut contents, config.width as usize, config.height as usize);

            fern(config, &mut image);

            contents
        }
    }
}

#[cfg(feature = "avif")]
pub fn write_image(config: &Config, mut contents: Vec<RGB>) {
    let img_config = ravif::Config {
        speed: 8,
        quality: 100.0,
        threads: 0,
        color_space: ravif::ColorSpace::YCbCr,
        alpha_quality: 0.0,
        premultiplied_alpha: false,
    };
    let img = Image::new(
        contents.as_mut_slice(),
        config.width as usize,
        config.height as usize,
    );

    let data = image_to_data(img, &img_config, config);
    let mut file =
        std::fs::File::create(&config.filename).expect("failed to create output image file");
    file.write_all(&data).expect("failed to write image data");
    file.flush().expect("failed to flush file");

    if config.open {
        fn start_shell(cmd: &str, command_arg: &str, exec: &str) {
            std::process::Command::new(cmd)
                .arg(command_arg)
                .arg(exec)
                .spawn()
                .expect("failed to open image");
        }
        #[cfg(windows)]
        {
            start_shell("cmd", "/C", &format!("start {}", config.filename));
        }
        #[cfg(target_os = "macos")]
        {
            start_shell("sh", "-c", &format!("open {:?}", config.filename));
        };
        #[cfg(all(not(target_os = "macos"), unix))]
        {
            start_shell("sh", "-c", &format!("xdg-open {:?}", config.filename));
        };
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
        Algo::Mandelbrot => recursive(config.iterations(), start, start, config.limit),
        Algo::Julia(c) => recursive(config.iterations(), start, c, config.limit),
        _ => unreachable(),
    };

    let dist = mandelbrot.squared_distance();

    if dist > config.stable_limit {
        let mut iters = iters as f64;

        if config.smooth {
            // https://en.wikipedia.org/wiki/Plotting_algorithms_for_the_Mandelbrot_set#Continuous_(smooth)_coloring

            let log_zn = f64::log2(dist.sqrt()) / 2.0;
            let nu = f64::log2(log_zn);

            iters += 1.0 - nu;
        }

        let mult = iters as f64 / config.iterations() as f64 * config.exposure;
        color_multiply(config.primary_color(), mult)
    } else if config.inside {
        color_multiply(config.secondary_color(), dist)
    } else {
        BLACK
    }
}

pub struct Image<'a> {
    contents: &'a mut [RGB],
    width: usize,
    height: usize,
}
impl<'a> Image<'a> {
    pub fn new(contents: &'a mut [RGB], width: usize, height: usize) -> Self {
        Self {
            contents,
            width,
            height,
        }
    }
    pub fn pixel_mut(&mut self, x: usize, y: usize) -> Option<&mut RGB> {
        if x > self.width {
            return None;
        }
        let index = y * self.width + x;
        if self.contents.len() < index {
            return None;
        }
        self.contents.get_mut(index)
    }
    fn subtract_pixel(&mut self, x: usize, y: usize, value: RGB, amount: f64) {
        let pixel = if let Some(p) = self.pixel_mut(x, y) {
            p
        } else {
            return;
        };

        let new = RGB::new(
            (pixel.r as f64 * 1.0 / ((((1.0 / (value.r as f64 / 255.0)) - 1.0) * amount) + 1.0))
                as u8,
            (pixel.g as f64 * 1.0 / ((((1.0 / (value.g as f64 / 255.0)) - 1.0) * amount) + 1.0))
                as u8,
            (pixel.b as f64 * 1.0 / ((((1.0 / (value.b as f64 / 255.0)) - 1.0) * amount) + 1.0))
                as u8,
        );
        *pixel = new;
    }
}
#[cfg(feature = "avif")]
impl<'a> From<Image<'a>> for ravif::Img<&'a [ravif::RGB8]> {
    fn from(me: Image<'a>) -> Self {
        ravif::Img::new(RGB::transmute(me.contents), me.width, me.height)
    }
}
fn color_multiply(color: RGB, mult: f64) -> RGB {
    RGB::new(
        (color.r as f64 * mult) as u8,
        (color.g as f64 * mult) as u8,
        (color.b as f64 * mult) as u8,
    )
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
#[inline(always)]
pub fn fern(config: &Config, image: &mut Image) {
    let width = config.width as f64;
    let height = config.height as f64;
    let mut x = (config.pos.re) * width;
    let mut y = (config.pos.im) * height;

    // 0.006 just works fine, to get the scale in line with the other algos
    let effective_scale_x = 65.0 * config.scale.re * config.height as f64 * 0.006;
    let effective_scale_y = 37.0 * config.scale.im * config.height as f64 * 0.006;

    let mut rng = rand::rngs::SmallRng::from_entropy();

    let color = config.primary_color();

    for _ in 0..config.iterations() {
        image.subtract_pixel(
            (((x - config.pos.re) * effective_scale_x) + width / 2.0) as usize,
            // 5.0 seems to work fine
            (height - ((y + (config.pos.im - 5.0) - 0.5) * effective_scale_y + height / 2.0))
                as usize,
            color,
            config.color_weight,
        );

        let r: f64 = rng.gen();

        // https://en.wikipedia.org/wiki/Barnsley_fern#Python
        if r < 0.01 {
            let old_x = x;
            x = 0.00 * x + 0.00 * y;
            y = 0.00 * old_x + 0.16 * y + 0.00;
        } else if r < 0.86 {
            let old_x = x;
            x = 0.85 * x + 0.04 * y;
            y = -0.04 * old_x + 0.85 * y + 1.60;
        } else if r < 0.93 {
            let old_x = x;
            x = 0.20 * x - 0.26 * y;
            y = 0.23 * old_x + 0.22 * y + 1.60;
        } else {
            let old_x = x;
            x = -0.15 * x + 0.28 * y;
            y = 0.26 * old_x + 0.24 * y + 0.44;
        }
    }
}