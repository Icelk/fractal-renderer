use std::io::Write;
use std::ops::Add;

use clap::Arg;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

fn main() {
    let app = clap::App::new("fractal-renderer")
        .arg(Arg::new("width").required(true))
        .arg(Arg::new("height").required(true))
        .arg(
            Arg::new("iterations")
                .long("iterations")
                .short('i')
                .takes_value(true),
        )
        .arg(
            Arg::new("top_falloff")
                .long("falloff")
                .short('f')
                .takes_value(true),
        )
        .arg(Arg::new("pos_x").short('x').takes_value(true))
        .arg(Arg::new("pos_y").short('y').takes_value(true))
        .arg(Arg::new("scale_y").long("scale-y").takes_value(true))
        .arg(Arg::new("scale_x").long("scale-x").takes_value(true))
        .arg(
            Arg::new("filename")
                .long("output")
                .short('o')
                .takes_value(true),
        );

    let matches = app.get_matches();

    let width = matches.value_of_t("width").unwrap_or(500);
    let width_f = width as f64;
    let height = matches.value_of_t("height").unwrap_or(500);
    let height_f = height as f64;
    let iterations = matches.value_of_t("iterations").unwrap_or(30);
    let pos = Imaginary {
        re: matches.value_of_t("pos_x").unwrap_or(-0.3),
        im: matches.value_of_t("pos_y").unwrap_or(0.0),
    };
    let scale = Imaginary {
        re: matches.value_of_t("scale_x").unwrap_or(0.5),
        im: matches.value_of_t("scale_y").unwrap_or(0.5),
    };
    let col = ravif::RGB8::new(40, 40, 255);
    let top_falloff = matches.value_of_t("top_falloff").unwrap_or(100.0);
    let filename = matches
        .value_of("filename")
        .map(|f| format!("{}.avif", f))
        .unwrap_or_else(|| "output.avif".to_owned());

    let image: Vec<_> = (0..height)
        .into_par_iter()
        .map(|y| {
            let mut row = Vec::with_capacity(width as usize);
            for x in 0..width {
                let start = xy_to_imaginary(x, y, width_f, height_f, &pos, &scale);
                let mandlebrot = mandelbrot(iterations, start);

                let dist = mandlebrot.squared_distance();
                let weight = if dist > 1.0 {
                    top_falloff / (dist - 1.0)
                } else {
                    dist
                };
                let pixel = ravif::RGB8::new(
                    (col.r as f64 * weight) as u8,
                    (col.g as f64 * weight) as u8,
                    (col.b as f64 * weight) as u8,
                );
                row.push(pixel)
            }
            row
        })
        .flatten()
        .collect();

    let img = ravif::Img::new(image.as_slice(), width as usize, height as usize);
    println!("Starting encode");
    let (data, _) = ravif::encode_rgb(
        img,
        &ravif::Config {
            speed: 10,
            quality: 80.0,
            threads: 0,
            color_space: ravif::ColorSpace::YCbCr,
            alpha_quality: 0.0,
            premultiplied_alpha: false,
        },
    )
    .expect("encoding failed");

    let mut file = std::fs::File::create(filename).expect("failed to create output image file");
    file.write_all(&data).expect("failed to write image data");
    file.flush().expect("failed to flush file");
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
fn coord_to_space(coord: f64, max: f64, offset: f64, pos: &f64, scale: &f64) -> f64 {
    ((coord / max) - offset + pos) / scale
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
    let re = coord_to_space(x as f64, height, (width / height) / 2.0, &pos.re, &scale.re);
    let im = coord_to_space(y as f64, height, 0.5, &pos.im, &scale.im);
    Imaginary { re, im }
}

#[inline(always)]
fn mandelbrot(iterations: u32, start: Imaginary) -> Imaginary {
    let mut previous = start;
    for _ in 0..iterations {
        previous = previous.square() + start;
    }
    previous
}
