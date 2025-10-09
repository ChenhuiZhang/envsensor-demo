use egui::ComboBox;
use egui_plot::{Line, Plot, PlotPoints};
use rand::Rng;
use std::time::{Duration, Instant};

struct App {
    data: Vec<(f64, f64)>,
    start_time: Instant,
    running: bool,
    last_update: Instant,
    dropdown_choice: usize,
}

fn main() -> eframe::Result<()> {
    let app = App {
        data: Vec::new(),
        start_time: Instant::now(),
        running: false,
        last_update: Instant::now(),
        dropdown_choice: 0,
    };

    let options = eframe::NativeOptions::default();
    eframe::run_native("EnvSensor Demo", options, Box::new(|_| Ok(Box::new(app))))
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top control panel
        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            egui::Frame::default()
                .inner_margin(egui::Margin {
                    left: 2,
                    right: 2,
                    top: 2,
                    bottom: 2,
                }) // top, bottom margins
                .show(ui, |ui| {
                    // Make text larger
                    ui.style_mut()
                        .text_styles
                        .get_mut(&egui::TextStyle::Button)
                        .unwrap()
                        .size = 20.0;
                    ui.style_mut()
                        .text_styles
                        .get_mut(&egui::TextStyle::Body)
                        .unwrap()
                        .size = 20.0;

                    // Make widgets taller/wider
                    ui.style_mut().spacing.interact_size.y = 30.0;

                    ui.horizontal(|ui| {
                        // Dropdown
                        ui.label("Sensor");
                        ComboBox::from_id_salt("sensor_dropdown")
                            .selected_text(format!("Choice {}", self.dropdown_choice))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.dropdown_choice, 0, "Choice 0");
                                ui.selectable_value(&mut self.dropdown_choice, 1, "Choice 1");
                                ui.selectable_value(&mut self.dropdown_choice, 2, "Choice 2");
                            });

                        ui.label("Port");
                        ComboBox::from_id_salt("port_dropdown")
                            .selected_text(format!("Choice {}", self.dropdown_choice))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.dropdown_choice, 0, "/dev/ttyS1");
                                ui.selectable_value(&mut self.dropdown_choice, 1, "/dev/ttyS2");
                                ui.selectable_value(&mut self.dropdown_choice, 2, "/dev/ttyUSB1");
                            });

                        // Start button
                        if ui.button("Start").clicked() {
                            self.running = true;
                            self.start_time = Instant::now();
                            self.data.clear(); // reset
                        }
                    });
                });
        });

        // If running, update data every 1 second
        if self.running && self.last_update.elapsed() >= Duration::from_secs(1) {
            let elapsed = self.start_time.elapsed().as_secs_f64();
            let y = rand::rng().random_range(0.0..100.0);
            self.data.push((elapsed, y));
            if self.data.len() > 100 {
                self.data.remove(0);
            }
            self.last_update = Instant::now();
        }

        // Chart in central panel
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::default()
                .inner_margin(egui::Margin {
                    left: 2,
                    right: 2,
                    top: 2,
                    bottom: 2,
                }) // top, bottom margins
                .show(ui, |ui| {
                    let points: PlotPoints = self.data.iter().map(|&(x, y)| [x, y]).collect();
                    Plot::new("random_line_chart")
                        .view_aspect(2.0)
                        .show(ui, |plot_ui| {
                            plot_ui.line(Line::new("", points));
                        });
                });
        });

        // request redraw
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}
