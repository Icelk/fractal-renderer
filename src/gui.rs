use crate::{Algo, Config, Imaginary};
use std::cmp;
use std::str::FromStr;
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
    gui_on: bool,
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
            gui_on: true,
            image,
            texture: None,
            working,
            redraw_channel,
            try_redraw: false,
        }
    }
    fn number_input<T: ToString + FromStr>(value: &mut T, ui: &mut egui::Ui) -> Option<T> {
        let mut s = value.to_string();
        if ui
            .add_sized(
                egui::Vec2::new(60.0, ui.available_height()),
                egui::TextEdit::singleline(&mut s),
            )
            .changed()
        {
            if let Ok(i) = s.parse() {
                return Some(i);
            }
        }
        None
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

        if ctx.input().key_down(egui::Key::M) {
            self.gui_on = !self.gui_on;
        }

        if self.gui_on {
            // So the combo box works (needs to have space below)
            egui::TopBottomPanel::top("controls").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    {
                        let combo_box = egui::ComboBox::from_id_source("type")
                            .selected_text(match self.state.algo {
                                crate::Algo::Mandelbrot => "Mandelbrot",
                                crate::Algo::Julia(_) => "Julia",
                                crate::Algo::BarnsleyFern => "Fern",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.state.algo,
                                    Algo::Mandelbrot,
                                    "Mandelbrot",
                                );
                                ui.selectable_value(
                                    &mut self.state.algo,
                                    Algo::Julia(Imaginary::ZERO),
                                    "Julia",
                                );
                                ui.selectable_value(
                                    &mut self.state.algo,
                                    Algo::BarnsleyFern,
                                    "Fern",
                                );
                            });
                        if combo_box.response.changed() {
                            self.state.iterations = None;
                            self.state.pos = Imaginary::ZERO;
                            self.state.scale = Imaginary::ONE * 0.4;
                        }
                    }
                    // Resolution
                    {
                        let mut width = self.state.width;
                        if let Some(width) = Self::number_input(&mut width, ui) {
                            self.state.width = width;
                        }
                        let mut height = self.state.height;
                        if let Some(height) = Self::number_input(&mut height, ui) {
                            self.state.height = height;
                        }
                    }

                    // Iterations
                    {
                        let mut iters = self.state.iterations();
                        if let Some(iter) = Self::number_input(&mut iters, ui) {
                            self.state.iterations = Some(iter)
                        }
                    }
                    // julia pos
                    if let Algo::Julia(julia_c) = &mut self.state.algo {
                        use std::ops::RangeInclusive;
                        struct PointSelect<'a> {
                            size: egui::Vec2,
                            circle_radius: f32,
                            range: RangeInclusive<egui::Vec2>,
                            value: &'a mut egui::Vec2,
                        }
                        impl<'a> PointSelect<'a> {
                            fn new(
                                value: &'a mut egui::Vec2,
                                range: RangeInclusive<egui::Vec2>,
                                size: f32,
                            ) -> Self {
                                PointSelect {
                                    value,
                                    range,
                                    circle_radius: 4.0,
                                    size: egui::Vec2::new(size, size),
                                }
                            }

                            fn x_range(&self) -> RangeInclusive<f32> {
                                self.range.start().x..=self.range.end().x
                            }
                            fn y_range(&self) -> RangeInclusive<f32> {
                                self.range.end().y..=self.range.start().y
                            }

                            fn value_to_ui_pos(&self, rect: &egui::Rect) -> egui::Pos2 {
                                let x =
                                    egui::remap_clamp(self.value.x, self.x_range(), rect.x_range());
                                let y =
                                    egui::remap_clamp(self.value.y, self.y_range(), rect.y_range());
                                egui::Pos2::new(x, y)
                            }
                            fn ui_pos_to_value(
                                &self,
                                rect: &egui::Rect,
                                pos: egui::Pos2,
                            ) -> egui::Vec2 {
                                let x = egui::remap_clamp(pos.x, rect.x_range(), self.x_range());
                                let y = egui::remap_clamp(pos.y, rect.y_range(), self.y_range());

                                egui::Vec2::new(x, y)
                            }
                        }
                        impl egui::Widget for PointSelect<'_> {
                            fn ui(self, ui: &mut egui::Ui) -> egui::Response {
                                let (rect, mut response) = ui
                                    .allocate_exact_size(self.size, egui::Sense::click_and_drag());
                                let painter = ui.painter();

                                let visuals = ui.style().interact(&response);
                                let line_stroke = visuals.fg_stroke;

                                let circle_color =
                                    ui.style().visuals.widgets.active.fg_stroke.color;

                                let line = |from: egui::Pos2, to: egui::Pos2| {
                                    painter.line_segment([from, to], line_stroke);
                                };

                                line(rect.center_top(), rect.center_bottom());
                                line(rect.left_center(), rect.right_center());

                                let circle_pos = self.value_to_ui_pos(&rect);
                                painter.circle_filled(circle_pos, self.circle_radius, circle_color);

                                if response.dragged() {
                                    if let Some(mouse_pos) = ui.input().pointer.interact_pos() {
                                        *self.value = self.ui_pos_to_value(&rect, mouse_pos);
                                    }
                                    response.mark_changed();
                                }

                                response
                            }
                        }

                        let mut value = egui::Vec2::new(julia_c.re as f32, julia_c.im as f32);

                        let frame = egui::containers::Frame::dark_canvas(ui.style())
                            .margin(egui::Vec2::ZERO);

                        frame.show(ui, |ui| {
                            let widget = PointSelect::new(
                                &mut value,
                                egui::Vec2::new(-1.5, -1.5)..=egui::Vec2::new(1.5, 1.5),
                                80.0,
                            );
                            ui.add(widget).changed()
                        });

                        julia_c.re = value.x as f64;
                        julia_c.im = value.y as f64;
                    }
                    // Exposure
                    {
                        ui.add(
                            egui::Slider::new(&mut self.state.exposure, 0.01..=50.0)
                                .logarithmic(true),
                        );
                    }
                    // Color weight
                    if let Algo::BarnsleyFern = self.state.algo {
                        ui.add(
                            egui::Slider::new(&mut self.state.color_weight, 0.0001..=10.0)
                                .logarithmic(true),
                        );
                    }
                    // Flags
                    if let Algo::Mandelbrot | Algo::Julia(_) = self.state.algo {
                        ui.checkbox(&mut self.state.inside, "Coloured inside");
                        ui.checkbox(&mut self.state.smooth, "Smoothed");
                    }
                    // Movement
                    #[allow(unused_braces, clippy::blocks_in_if_conditions)]
                    if !ctx.wants_keyboard_input() {
                        let dt = { ctx.input().predicted_dt } as f64;

                        let scale_x = 1.0 / self.state.scale.re;
                        let scale_y = 1.0 / self.state.scale.im;
                        // move
                        if { ctx.input().key_down(egui::Key::ArrowLeft) } {
                            self.state.pos.re -= scale_x * dt * 0.5;
                        }
                        if { ctx.input().key_down(egui::Key::ArrowRight) } {
                            self.state.pos.re += scale_y * dt * 0.5;
                        }
                        if { ctx.input().key_down(egui::Key::ArrowUp) } {
                            self.state.pos.im -= scale_x * dt * 0.5;
                        }
                        if { ctx.input().key_down(egui::Key::ArrowDown) } {
                            self.state.pos.im += scale_y * dt * 0.5;
                        }
                        // scale
                        {
                            let delta = ctx.input().scroll_delta.y;
                            if delta >= 1.0 || delta <= -1.0 {
                                self.state.scale = self.state.scale
                                    * if delta < 0.0 {
                                        let delta = -delta;
                                        let scale_diff =
                                            (F32Ord((delta / 10.0 + 1.0).log10() / 2.0)
                                                .min(F32Ord(1.0)))
                                            .0;

                                        1.0 - scale_diff as f64
                                    } else {
                                        1.0 + (delta as f64 / 80.0)
                                    };
                            }
                        }
                        // screenshot
                        if { ctx.input().key_pressed(egui::Key::S) } {
                            let mut config = self.state.clone();
                            std::thread::spawn(move || {
                                config.width *= 2;
                                config.height *= 2;
                                let image = crate::get_image(&config);
                                crate::write_image(&config, image);
                            });
                        }
                        // info
                        ui.label(format!("{:.3}", self.state.scale.re));
                        if let Algo::Julia(value) = self.state.algo {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(format!(
                                    "{:.5}+{:.5}i",
                                    self.state.pos.re, self.state.pos.im
                                ));
                                ui.end_row();
                                ui.label(format!("{:.5}+{:.5}i", value.re, value.im,));
                            });
                        } else {
                            ui.label(format!(
                                "{:.5}+{:.5}i",
                                self.state.pos.re, self.state.pos.im
                            ));
                        }
                    }
                })
            });
            if self.state != previous_state {
                if self.state.algo.is_different(&previous_state.algo) {
                    let new_state = Config {
                        algo: self.state.algo.clone(),
                        ..Default::default()
                    };
                    self.state = new_state;
                }
                self.request_redraw(frame.clone());
            }
        }
        // Render this after controls to give that space. (even if it was below this on screen)
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
    }
}

pub fn start() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(App::new()), options);
}
