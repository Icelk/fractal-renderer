use std::fmt::Display;
use std::io::Write;
use std::ops::Add;
use std::process::Command;
use std::str::FromStr;

use clap::{Arg, ArgGroup};
use rand::{Rng, SeedableRng};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

#[derive(Debug, Clone)]
enum Algo {
    Mandelbrot,
    BarnsleyFern,
    Julia,
}
enum AlgoParseError {
    /// Use one of the variants.
    Incorrect,
}
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
            Self::Julia
        } else {
            return Err(AlgoParseError::Incorrect);
        })
    }
}

fn get_config() -> Config {
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
                .help("Default is 50 for Mandelbrot and 50_000 for Fern.")
        )
        .arg(
            Arg::new("limit")
                .long("limit")
                .short('l')
                .help("Limit where vaules are treated to escape. Only applicable to Mandelbrot.")
                .takes_value(true)
                .default_value("65536"),
        )
        .arg(
            Arg::new("stable_limit")
                .long("stable-limit")
                .help("The limit of points considered inside the fractal. Only applicable to Mandelbrot.")
                .default_value("2"),
        )
        .arg(
            Arg::new("pos_x")
                .short('x')
                .takes_value(true)
                .default_value("0.6"),
        )
        .arg(
            Arg::new("pos_y")
                .short('y')
                .takes_value(true)
                .default_value("0"),
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
                .possible_value("julia"),
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
    let inside_disabled = matches.is_present("disable_inside");
    let unsmooth = matches.is_present("unsmooth");
    let filename = matches
        .value_of("filename")
        .map(|f| format!("{}.avif", f))
        .unwrap();
    let open = matches.is_present("open");
    let algo = matches.value_of_t("algo").unwrap();
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
        color: None,
        open,
        filename,
        algo,
    }
}

#[derive(Debug, Clone)]
struct Config {
    width: u32,
    height: u32,
    iterations: Option<u32>,
    limit: f64,
    stable_limit: f64,
    pos: Imaginary,
    scale: Imaginary,
    exposure: f64,
    inside: bool,
    smooth: bool,
    color: Option<ravif::RGB8>,
    filename: String,
    open: bool,
    algo: Algo,
}
impl Config {
    fn iterations(&self) -> u32 {
        if let Some(iters) = self.iterations {
            return iters;
        }
        match self.algo {
            Algo::Mandelbrot => 50,
            Algo::BarnsleyFern => 50_000,
            Algo::Julia => unimplemented!(),
        }
    }
    fn color(&self) -> ravif::RGB8 {
        if let Some(color) = self.color {
            return color;
        }

        match self.algo {
            Algo::Mandelbrot => ravif::RGB8::new(40, 40, 255),
            Algo::BarnsleyFern => ravif::RGB8::new(20, 150, 30),
            Algo::Julia => unimplemented!(),
        }
    }
}

fn image_to_data(image: Image, image_config: &ravif::Config, config: &Config) -> Vec<u8> {
    println!("Starting encode.");
    let (data, _) = ravif::encode_rgb(image.into(), image_config).expect("encoding failed");
    println!("Finished encode. Writing file {:?}.", config.filename);
    data
}

fn main() {
    let config = get_config();

    let img_config = ravif::Config {
        speed: 8,
        quality: 100.0,
        threads: 0,
        color_space: ravif::ColorSpace::YCbCr,
        alpha_quality: 0.0,
        premultiplied_alpha: false,
    };

    let data = match config.algo {
        Algo::Mandelbrot => {
            let mut image: Vec<_> = (0..config.height)
                // Only one parallell iter, else, it'd be less efficient.
                .into_par_iter()
                .map(|y| {
                    let mut row = Vec::with_capacity(config.width as usize);
                    for x in 0..config.width {
                        row.push(get_mandelbrot_pixel(&config, x, y))
                    }
                    row
                })
                .flatten()
                .collect();

            let img = Image::new(
                image.as_mut_slice(),
                config.width as usize,
                config.height as usize,
            );
            image_to_data(img, &img_config, &config)
        }
        Algo::BarnsleyFern => {
            let mut contents =
                vec![ravif::RGB8::new(0, 0, 0); (config.width * config.height) as usize];

            let mut image =
                Image::new(&mut contents, config.width as usize, config.height as usize);

            fern(&config, &mut image);

            image_to_data(image, &img_config, &config)
        }
        Algo::Julia => unimplemented!(),
    };

    let mut file =
        std::fs::File::create(&config.filename).expect("failed to create output image file");
    file.write_all(&data).expect("failed to write image data");
    file.flush().expect("failed to flush file");

    if config.open {
        fn start_shell(cmd: &str, command_arg: &str, exec: &str) {
            Command::new(cmd)
                .arg(command_arg)
                .arg(exec)
                .spawn()
                .and_then(|mut c| c.wait())
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

#[derive(Debug, Clone, Copy)]
struct Imaginary {
    re: f64,
    im: f64,
}
impl Imaginary {
    #[inline(always)]
    fn square(self) -> Self {
        let re = (self.re * self.re) - (self.im * self.im);
        let im = 2.0 * self.re * self.im;

        Self { re, im }
    }
    #[inline(always)]
    fn squared_distance(self) -> f64 {
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

#[inline(always)]
fn coord_to_space(coord: f64, max: f64, offset: f64, pos: f64, scale: f64) -> f64 {
    ((coord / max) - offset) / scale - pos
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

fn get_mandelbrot_pixel(config: &Config, x: u32, y: u32) -> ravif::RGB8 {
    let start = xy_to_imaginary(
        x,
        y,
        config.width as f64,
        config.height as f64,
        &config.pos,
        &config.scale,
    );
    let (mandelbrot, iters) = mandelbrot(config.iterations(), start, config.limit);

    let dist = mandelbrot.squared_distance();
    let weight = if dist > config.stable_limit {
        let mut iters = iters as f64;

        if config.smooth {
            // https://en.wikipedia.org/wiki/Plotting_algorithms_for_the_Mandelbrot_set#Continuous_(smooth)_coloring

            let log_zn = f64::log2(dist) / 2.0;
            let nu = f64::log2(log_zn);

            iters += 1.0 - nu;
        }

        iters as f64 / config.iterations() as f64 * config.exposure
    } else if config.inside {
        dist
    } else {
        0.0
    };
    let color = config.color();
    color_multiply(color, weight)
}

struct Image<'a> {
    contents: &'a mut [ravif::RGB8],
    width: usize,
    height: usize,
}
impl<'a> Image<'a> {
    fn new(contents: &'a mut [ravif::RGB8], width: usize, height: usize) -> Self {
        Self {
            contents,
            width,
            height,
        }
    }
    fn set_pixel(&mut self, x: usize, y: usize, value: ravif::RGB8) {
        let index = y * self.width + x;
        if self.contents.len() < index {
            return;
        }
        self.contents[index] = value;
    }
}
impl<'a> From<Image<'a>> for ravif::Img<&'a [ravif::RGB8]> {
    fn from(me: Image<'a>) -> Self {
        ravif::Img::new(me.contents, me.width, me.height)
    }
}
fn color_multiply(color: ravif::RGB8, mult: f64) -> ravif::RGB8 {
    ravif::RGB8::new(
        (color.r as f64 * mult) as u8,
        (color.g as f64 * mult) as u8,
        (color.b as f64 * mult) as u8,
    )
}

/// `limit` is distance from center considered out of bounds.
///
/// # Less banding
///
/// Increase limit and iterations.
///
/// # Return
///
/// Returns the final position and the number of iterations to get there.
#[inline(always)]
fn mandelbrot(iterations: u32, start: Imaginary, limit: f64) -> (Imaginary, u32) {
    let squared = limit * limit;
    let mut previous = start;
    for i in 0..iterations {
        let next = previous.square() + start;
        let dist = next.squared_distance();
        if dist > squared {
            return (next, i);
        }
        // optimization
        if dist < 0.0001 {
            return (next, i);
        }
        previous = next;
    }
    (previous, iterations)
}
fn fern(config: &Config, image: &mut Image) {
    let width = config.width as f64;
    let height = config.height as f64;
    let mut x = (config.pos.re) * width;
    let mut y = (config.pos.im) * height;

    let mut rng = rand::rngs::SmallRng::from_entropy();

    let mut i_without_reset = 0;

    let color = config.color();

    for _ in 0..config.iterations() {
        let pixel_x = x as f64 * 65.0 * config.scale.re;
        let pixel_y = y as f64 * 37.0 * config.scale.re;

        image.set_pixel(
            (pixel_x + width / 2.0) as usize,
            (height - (pixel_y )) as usize,
            color_multiply(color, (config.exposure * 20.0) / i_without_reset as f64),
        );

        let r: f64 = rng.gen();

        i_without_reset += 1;

        // https://en.wikipedia.org/wiki/Barnsley_fern#Python
        if r < 0.01 {
            x = 0.00 * x + 0.00 * y;
            y = 0.00 * x + 0.16 * y + 0.00;
            i_without_reset = 0;
        } else if r < 0.86 {
            x = 0.85 * x + 0.04 * y;
            y = -0.04 * x + 0.85 * y + 1.60;
        } else if r < 0.93 {
            x = 0.20 * x - 0.26 * y;
            y = 0.23 * x + 0.22 * y + 1.60;
        } else {
            x = -0.15 * x + 0.28 * y;
            y = 0.26 * x + 0.24 * y + 0.44;
        }
    }
}
