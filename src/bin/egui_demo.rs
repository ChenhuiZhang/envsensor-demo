use egui::ComboBox;
use egui_plot::{Line, Plot, PlotPoints};
use std::time::Duration;

use envsensor_demo::{
    sensor::{Sensor, SensorModel},
    serial_port_list,
};

struct App {
    data: Vec<(f64, f64)>,
    running: Option<Sensor>,
    sensor_choice: usize,
    sensors: Vec<SensorModel>,
    port_choice: usize,
    ports: Vec<String>,
}

fn main() -> eframe::Result<()> {
    let app = App {
        data: Vec::new(),
        running: None,
        sensor_choice: 0,
        sensors: SensorModel::all(),
        port_choice: 0,
        ports: serial_port_list(),
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
                        ui.add_enabled_ui(self.running.is_none(), |ui| {
                            ui.label("Sensor");
                            ComboBox::from_id_salt("sensor_dropdown")
                                .selected_text(self.sensors[self.sensor_choice].as_ref())
                                .show_ui(ui, |ui| {
                                    for (idx, sensor) in self.sensors.iter().enumerate() {
                                        ui.selectable_value(
                                            &mut self.sensor_choice,
                                            idx,
                                            sensor.as_ref(),
                                        );
                                    }
                                });

                            ui.label("Port");
                            ComboBox::from_id_salt("port_dropdown")
                                .selected_text(
                                    self.ports
                                        .get(self.port_choice)
                                        .unwrap_or(&"No available port".to_string()),
                                )
                                .show_ui(ui, |ui| {
                                    for (idx, port) in self.ports.iter().enumerate() {
                                        ui.selectable_value(&mut self.port_choice, idx, port);
                                    }
                                });
                        });

                        // Start button
                        if ui
                            .button(match self.running {
                                Some(_) => "Stop",
                                None => "Start",
                            })
                            .clicked()
                        {
                            match &self.running {
                                Some(s) => {
                                    s.stop();

                                    self.running = None;
                                }
                                None => {
                                    let s = Sensor::new(
                                        &self.sensors[self.sensor_choice],
                                        &self.ports[self.port_choice],
                                    )
                                    .unwrap();

                                    if s.start().is_ok() {
                                        self.running = Some(s);
                                    }
                                }
                            }
                        }
                    });
                });
        });

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
