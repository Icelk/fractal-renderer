pub use calc::{get_recursive_pixel, Algo, Config, Imaginary, RGB};
use std::io::Write;

use clap::{Arg, ArgGroup};
use rand::{Rng, SeedableRng};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

#[cfg(feature = "gpu")]
#[path = "compute.rs"]
pub mod compute;
#[cfg(feature = "gui")]
#[path = "gui.rs"]
pub mod gui;

#[cfg(feature = "avif")]
pub const fn transmute_rgb_slice(me: &[RGB]) -> &[ravif::RGB8] {
    unsafe { std::mem::transmute(me) }
}

#[cfg(feature = "avif")]
pub fn rgb_convert(rgb: RGB) -> ravif::RGB8 {
    ravif::RGB8::new(rgb.r, rgb.g, rgb.b)
}

fn parse_hex_rgb(s: &str) -> RGB {
    let (p1, p2) = s.split_at(2);
    let (p2, p3) = p2.split_at(2);
    let r = u8::from_str_radix(p1, 16).expect("failed to parse hex color");
    let g = u8::from_str_radix(p2, 16).expect("failed to parse hex color");
    let b = u8::from_str_radix(p3, 16).expect("failed to parse hex color");
    RGB::new(r, g, b)
}

pub fn get_options() -> Options {
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
    let algo = matches.value_of_t("algo").unwrap();
    let mut julia_set = Imaginary::ZERO;
    if let Algo::Julia = &algo {
        julia_set.re = matches.value_of_t("julia_re").unwrap();
        julia_set.im = matches.value_of_t("julia_im").unwrap();
    }
    let color_weight = matches.value_of_t("color_weight").unwrap();
    let gui = matches.is_present("gui");
    if gui && cfg!(not(feature = "gui")) {
        eprintln!("The gui feature isn't enabled! Remove the GUI argument.");
    }

    let reference = Config::new(algo.clone());
    let config = Config {
        width,
        height,
        iterations: iterations.unwrap_or(reference.iterations),
        limit,
        stable_limit,
        pos,
        scale,
        exposure,
        inside: !inside_disabled,
        smooth: !unsmooth,
        primary_color: primary_color.unwrap_or(reference.primary_color),
        secondary_color: secondary_color.unwrap_or(reference.secondary_color),
        color_weight,
        julia_set,
        algo,
    };

    Options {
        config,
        filename,
        open,
        gui,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    pub config: Config,

    pub filename: String,
    pub open: bool,
    pub gui: bool,
}

#[cfg(feature = "avif")]
pub fn image_to_data(image: Image, image_config: &ravif::Config, options: &Options) -> Vec<u8> {
    println!("Starting encode.");
    let (data, _) = ravif::encode_rgb(image.into(), image_config).expect("encoding failed");
    println!("Finished encode. Writing file {:?}.", options.filename);
    data
}

pub fn get_image(config: &Config) -> Vec<RGB> {
    match config.algo {
        Algo::Mandelbrot | Algo::Julia => {
            #[cfg(feature = "gpu")]
            {
                compute::start();
            }

            #[cfg(not(feature = "gpu"))]
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
                vec![config.secondary_color; (config.width * config.height) as usize];

            let mut image =
                Image::new(&mut contents, config.width as usize, config.height as usize);

            fern(config, &mut image);

            contents
        }
    }
}

#[cfg(feature = "avif")]
pub fn write_image(options: &Options, mut contents: Vec<RGB>) {
    let config = &options.config;
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

    let data = image_to_data(img, &img_config, options);
    let mut file =
        std::fs::File::create(&options.filename).expect("failed to create output image file");
    file.write_all(&data).expect("failed to write image data");
    file.flush().expect("failed to flush file");

    if options.open {
        fn start_shell(cmd: &str, command_arg: &str, exec: &str) {
            std::process::Command::new(cmd)
                .arg(command_arg)
                .arg(exec)
                .spawn()
                .expect("failed to open image");
        }
        #[cfg(windows)]
        {
            start_shell("cmd", "/C", &format!("start {}", options.filename));
        }
        #[cfg(target_os = "macos")]
        {
            start_shell("sh", "-c", &format!("open {:?}", options.filename));
        };
        #[cfg(all(not(target_os = "macos"), unix))]
        {
            start_shell("sh", "-c", &format!("xdg-open {:?}", options.filename));
        };
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
        ravif::Img::new(transmute_rgb_slice(me.contents), me.width, me.height)
    }
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

    let color = config.primary_color;

    for _ in 0..config.iterations {
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
