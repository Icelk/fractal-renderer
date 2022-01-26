use crate::Config;
use std::cmp;
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc, Condvar, Mutex};

use eframe::{egui, epi};

/// Panics if a `NaN` is used.
struct F32Ord(f32);
impl PartialEq for F32Ord {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl Eq for F32Ord {}
impl PartialOrd for F32Ord {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}
impl Ord for F32Ord {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

struct App {
    state: Config,
    image: Arc<Mutex<Option<egui::ColorImage>>>,
    texture: Option<(egui::TextureHandle, eframe::egui::Vec2)>,
    working: Arc<AtomicBool>,
    finish: Condvar,
    redraw_channel: mpsc::Sender<Config>,
    try_redraw: bool,
}
impl App {
    fn request_redraw(&mut self) {
        if self.working.load(std::sync::atomic::Ordering::SeqCst) {
            self.try_redraw = true;
            return;
        }
        self.try_redraw = false;
        self.working
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.redraw_channel.send(self.state.clone()).unwrap();
    }
    fn new() -> Self {
        let (redraw_channel, rx) = mpsc::channel();

        let image = Arc::new(Mutex::new(None));
        let image_handle = Arc::clone(&image);
        let working = Arc::new(AtomicBool::new(false));
        let working_handle = Arc::clone(&working);
        std::thread::spawn(move || {
            let thread_poll = rayon::ThreadPoolBuilder::new().build().unwrap();

            while let Ok(config) = rx.recv() {
                let mut contents = thread_poll.install(|| crate::get_image(&config));

                let mut image_rgb_contents: Vec<u8> = unsafe { std::mem::transmute(contents) };
                unsafe { image_rgb_contents.set_len(image_rgb_contents.len() * 3) };

                let image_buffer: image::RgbImage =
                    image::ImageBuffer::from_raw(config.width, config.height, image_rgb_contents)
                        .unwrap();

                let size = [image_buffer.width() as _, image_buffer.height() as _];
                let image_buffer = image::DynamicImage::ImageRgb8(image_buffer);
                let image_buffer = image_buffer.to_rgba8();
                let pixels = image_buffer.as_flat_samples();

                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                {
                    let mut lock = image_handle.lock().unwrap();
                    *lock = Some(color_image);
                }
                working_handle.store(false, std::sync::atomic::Ordering::SeqCst);
            }

            println!("Shutting rendering down.");
        });

        Self {
            state: Config::default(),
            image,
            texture: None,
            working,
            finish: Condvar::new(),
            redraw_channel,
            try_redraw: false,
        }
    }
}

impl epi::App for App {
    fn name(&self) -> &str {
        "Interact with fractals."
    }

    fn update(&mut self, ctx: &egui::Context, frame: &eframe::epi::Frame) {
        fn texture<'a>(
            app: & mut App,
            ctx: &egui::Context,
        ) -> (egui::TextureHandle, egui::Vec2) {
            if let Some(img) = app.image.lock().unwrap().take() {
                let size = img.size;
                let handle = ctx.load_texture("main fractal", img);
                app.texture = Some((handle, egui::Vec2::new(size[0] as _, size[1] as _)));
            }
            if let Some(texture) = &app.texture {
                return texture.clone();
            }
            app.request_redraw();
            texture(app, ctx)
        }
        let (texture, size) = texture(self, ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            let aspect_ratio = size.x / size.y;
            let mut available_size = ui.available_size();
            available_size.y = cmp::min(
                F32Ord(available_size.y),
                F32Ord(available_size.x / aspect_ratio),
            )
            .0;
            available_size.x = cmp::min(
                F32Ord(available_size.x),
                F32Ord(available_size.y * aspect_ratio),
            )
            .0;
            ui.image(&texture, available_size);
        });
    }
}

pub fn start() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(App::new()), options);
}
