use crate::Config;
use std::cmp;
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc, Mutex};

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
    redraw_channel: mpsc::Sender<(Config, epi::Frame)>,
    try_redraw: bool,
}
impl App {
    fn request_redraw(&mut self, frame: epi::Frame) {
        if self.working.load(std::sync::atomic::Ordering::SeqCst) {
            self.try_redraw = true;
            return;
        }
        self.try_redraw = false;
        self.working
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.redraw_channel
            .send((self.state.clone(), frame))
            .unwrap();
    }
    fn new() -> Self {
        let (redraw_channel, rx) = mpsc::channel::<(Config, epi::Frame)>();

        let image = Arc::new(Mutex::new(None));
        let image_handle = Arc::clone(&image);
        let working = Arc::new(AtomicBool::new(false));
        let working_handle = Arc::clone(&working);
        std::thread::spawn(move || {
            let thread_poll = rayon::ThreadPoolBuilder::new().build().unwrap();

            while let Ok((config, frame)) = rx.recv() {
                let contents = thread_poll.install(|| crate::get_image(&config));

                #[allow(clippy::unsound_collection_transmute)]
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
                frame.request_repaint();
            }

            println!("Shutting rendering down.");
        });

        Self {
            state: Config::default(),
            image,
            texture: None,
            working,
            redraw_channel,
            try_redraw: false,
        }
    }
}

impl epi::App for App {
    fn name(&self) -> &str {
        "Interact with fractals."
    }

    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        fn texture(
            app: &mut App,
            ctx: &egui::Context,
            frame: &epi::Frame,
        ) -> Option<(egui::TextureHandle, egui::Vec2)> {
            let img = { app.image.lock().unwrap().take() };
            if let Some(img) = img {
                let size = img.size;
                let handle = ctx.load_texture("main fractal", img);
                app.texture = Some((handle, egui::Vec2::new(size[0] as _, size[1] as _)));
                if app.try_redraw {
                    app.request_redraw(frame.clone());
                }
            }
            if let Some(texture) = &app.texture {
                return Some(texture.clone());
            }
            app.request_redraw(frame.clone());
            None
        }
        let texture = texture(self, ctx, frame);

        let previous_state = self.state.clone();

        egui::TopBottomPanel::bottom("controls").show(ctx, |ui| {
            // Iterations
            {
                let mut iterations = self.state.iterations().to_string();
                if ui.text_edit_singleline(&mut iterations).changed() {
                    if let Ok(i) = iterations.parse() {
                        self.state.iterations = Some(i)
                    }
                }
            }
        });
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::BLACK))
            .show(ctx, |ui| {
                if let Some((texture, size)) = texture {
                    let aspect_ratio = size.x / size.y;
                    let available_size = ui.available_size();
                    let mut space = available_size;
                    space.y = cmp::min(F32Ord(available_size.y), F32Ord(space.x / aspect_ratio)).0;
                    space.x = cmp::min(F32Ord(available_size.x), F32Ord(space.y * aspect_ratio)).0;
                    let margin = egui::Vec2::new(
                        (available_size.x - space.x) / 2.0,
                        (available_size.y - space.y) / 2.0,
                    );
                    egui::Frame::none().margin(margin).show(ui, |ui| {
                        ui.image(&texture, space);
                    });
                }
            });

        if self.state != previous_state {
            self.request_redraw(frame.clone());
        }
    }
}

pub fn start() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(App::new()), options);
}
