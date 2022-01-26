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
    finish: Arc<(Condvar, Mutex<bool>)>,
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
    fn wait_request(&self) {
        self.finish.0.wait(self.finish.1.lock().unwrap()).unwrap();
    }
    fn new() -> Self {
        let (redraw_channel, rx) = mpsc::channel();

        let image = Arc::new(Mutex::new(None));
        let image_handle = Arc::clone(&image);
        let working = Arc::new(AtomicBool::new(false));
        let working_handle = Arc::clone(&working);
        let finish = Arc::new((Condvar::new(), Mutex::new(false)));
        let finish_handle = Arc::clone(&finish);
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
                finish_handle.0.notify_all();
            }

            println!("Shutting rendering down.");
        });

        Self {
            state: Config::default(),
            image,
            texture: None,
            working,
            finish,
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
        fn texture<'a>(app: &mut App, ctx: &egui::Context) -> (egui::TextureHandle, egui::Vec2) {
            if app.working.load(std::sync::atomic::Ordering::SeqCst) {
                app.wait_request();
            }
            if let Some(img) = app.image.lock().unwrap().take() {
                let size = img.size;
                let handle = ctx.load_texture("main fractal", img);
                app.texture = Some((handle, egui::Vec2::new(size[0] as _, size[1] as _)));
            }
            if let Some(texture) = &app.texture {
                return texture.clone();
            }
            app.request_redraw();
            app.wait_request();
            texture(app, ctx)
        }
        let (texture, size) = texture(self, ctx);

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
                let aspect_ratio = size.x / size.y;
                let available_size = ui.available_size();
                let mut space = available_size;
                space.y = cmp::min(F32Ord(available_size.y), F32Ord(space.x / aspect_ratio)).0;
                space.x = cmp::min(F32Ord(available_size.x), F32Ord(space.y * aspect_ratio)).0;
                ui.allocate_space(egui::Vec2::new(
                    (available_size.x - space.x) / 2.0,
                    (available_size.y - space.y) / 2.0,
                ));
                ui.image(&texture, space);
            });

        if self.state != previous_state {
            self.request_redraw();
        }
    }
}

pub fn start() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(App::new()), options);
}
