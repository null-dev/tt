use eframe::{egui, Frame, glow};
use eframe::glow::HasContext;
use egui::{Color32, Event, pos2, Pos2, Rect, Rounding, Stroke, TouchPhase, Vec2, vec2};

fn main() {
    let mut native_options = eframe::NativeOptions::default();
    native_options.initial_window_size = Some(vec2(800 as f32, 480 as f32));
    eframe::run_native("My egui App", native_options, Box::new(|cc| Box::new(MyEguiApp::new(cc))));
}

#[derive(Default)]
struct MyEguiApp {}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_pixels_per_point(1.5);
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Self::default()
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Debug touch
            let events: Vec<Pos2> = ui.input().events.iter().filter_map(|e| {
                match e {
                    Event::Touch { pos, .. } => Some(*pos),
                    _ => None
                }
            }).collect();
            for pos in events {
                ui.painter().circle(pos, 5.0, Color32::YELLOW, Stroke::none());
            }

            /*egui::ScrollArea::both().show(ui, |ui| {
                ui.allocate_space(vec2(2000.0, 2000.0));
            });*/

            egui::ScrollArea::both().show(ui, |ui| {
                ctx.settings_ui(ui);
                // ctx.inspection_ui(ui);
            });

            /*ui.heading("Hello World!");

            // Clipping test
            egui::ScrollArea::both()
                .max_width(100.0)
                .max_height(100.0)
                .min_scrolled_height(100.0)
                .min_scrolled_width(100.0)
                .always_show_scroll(true)
                .auto_shrink([false, false])
                .show_viewport(ui, |ui, rect| {
                    let (_, r) = ui.allocate_space(vec2(200.0, 200.0));
                    ui.painter().rect_filled(
                        Rect::from([
                            r.min,
                            r.min + vec2(200.0, 200.0)
                        ]),
                        Rounding::none(),
                        Color32::RED
                    );
                    ui.painter().rect_filled(
                        Rect::from([
                            r.min,
                            r.min + vec2(100.0, 100.0)
                        ]),
                        Rounding::none(),
                        Color32::GREEN
                    );
                    ui.painter().rect_filled(
                        Rect::from([
                            r.min - vec2(50.0, 50.0),
                            r.min + vec2(50.0, 50.0)
                        ]),
                        Rounding::none(),
                        Color32::BLUE
                    );
                });*/
        });
    }
}