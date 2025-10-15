use std::io::Write;
use std::time::Duration;
use std::{
    fs::File,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

use anyhow::Result;
use bus::{Bus, BusReader};
use chrono::DateTime;
use chrono::Local;
use strum::{AsRefStr, IntoEnumIterator};
use strum_macros::EnumIter;

use crate::rydason::Rydason;
use crate::tb600b_c::TB600BC;

#[derive(Clone, Copy, Debug)]
pub enum SensorType {
    CO,
}

#[allow(non_camel_case_types)]
#[derive(AsRefStr, Clone, Copy, EnumIter)]
pub enum SensorModel {
    EC_TB600BC,
    RYDASON,
}

impl SensorModel {
    pub fn all() -> Vec<SensorModel> {
        SensorModel::iter().collect()
    }
}

pub struct Sensor {
    hw: SensorModel,
    port: String,
    flag: Arc<AtomicBool>,
    rx: BusReader<AppMsg>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct SensorData {
    ty: SensorType,
    value: f32,
    unit: &'static str,
}

#[derive(Clone, Debug)]
pub struct SampleData {
    timestamp: DateTime<Local>,
    data: Vec<SensorData>,
}

#[derive(Clone)]
pub enum AppMsg {
    Status(String),
    Sample(SampleData),
}

impl Sensor {
    pub fn new(model: &SensorModel, port: &str, rx: BusReader<AppMsg>) -> Result<Self> {
        Ok(Sensor {
            hw: *model,
            port: port.to_string(),
            flag: Arc::new(AtomicBool::new(false)),
            rx,
        })
    }

    pub fn start(&self, mut bus: Bus<AppMsg>) -> Result<()> {
        let port = self.port.clone();
        self.flag.store(false, Ordering::SeqCst);
        let flag = self.flag.clone();

        let mut rx = bus.add_rx();

        match self.hw {
            SensorModel::EC_TB600BC => {
                thread::spawn(move || -> Result<()> {
                    let mut sensor = TB600BC::new(&port).inspect_err(|e| {
                        bus.broadcast(AppMsg::Status(format!(
                            "Failed to create TB600BC sensor: {e}"
                        )));
                    })?;

                    bus.broadcast(AppMsg::Status("TB600BC init".to_string()));

                    sensor.switch_mode(true).inspect_err(|e| {
                        bus.broadcast(AppMsg::Status(format!(
                            "Failed to switch to auto report mode: {e}"
                        )));
                    })?;

                    bus.broadcast(AppMsg::Status("TB600BC switch to auto report".to_string()));

                    while !flag.load(Ordering::SeqCst) {
                        let (c1, c2) = sensor.read_auto_report_data().map_err(|e| {
                            bus.broadcast(AppMsg::Status(format!(
                                "Failed to read auto report data: {e}"
                            )));
                            e
                        })?;

                        let now = chrono::Local::now();
                        let v = vec![
                            SensorData {
                                ty: SensorType::CO,
                                value: c1,
                                unit: "ppm",
                            },
                            SensorData {
                                ty: SensorType::CO,
                                value: c2,
                                unit: "mg/m3",
                            },
                        ];

                        bus.broadcast(AppMsg::Sample(SampleData {
                            timestamp: now,
                            data: v,
                        }));
                    }

                    Ok(())
                });
            }
            SensorModel::RYDASON => {
                thread::spawn(move || -> Result<()> {
                    let mut sensor = Rydason::new(&port, 1).inspect_err(|e| {
                        eprintln!("Failed to create Rydason sensor: {e}");
                        bus.broadcast(AppMsg::Status(format!(
                            "Failed to create Rydason sensor: {e}"
                        )));
                    })?;

                    bus.broadcast(AppMsg::Status("Rydason init".to_string()));

                    while !flag.load(Ordering::SeqCst) {
                        let now = chrono::Local::now();
                        let v = vec![SensorData {
                            ty: SensorType::CO,
                            value: sensor.read_measured_value().map_err(|e| {
                                eprintln!("Failed to read measured value: {e}");
                                e
                            })?,
                            unit: "ppm",
                        }];

                        bus.broadcast(AppMsg::Sample(SampleData {
                            timestamp: now,
                            data: v,
                        }));

                        thread::sleep(Duration::from_secs(1));
                    }
                    Ok(())
                });
            }
        }

        let flag = self.flag.clone();
        let device = self.hw;

        thread::spawn(move || -> Result<()> {
            let filename = format!(
                "{}_{}.csv",
                chrono::Local::now().format("%Y-%m-%d-%H-%M-%S"),
                device.as_ref()
            );

            let mut csv = File::create(filename)?;
            // Write CSV header
            match device {
                SensorModel::EC_TB600BC => {
                    writeln!(csv, "Timestamp,CO(ppm),CO(mg/m3)")?;
                }
                SensorModel::RYDASON => {
                    writeln!(csv, "Timestamp,CO(ppm)")?;
                }
            }

            while !flag.load(Ordering::SeqCst) {
                if let Ok(AppMsg::Sample(sample)) = rx.recv() {
                    writeln!(
                        csv,
                        "{},{}",
                        sample.timestamp.format("%Y-%m-%d %H:%M:%S"),
                        sample
                            .data
                            .iter()
                            .map(|d| d.value.to_string())
                            .collect::<Vec<_>>()
                            .join(",")
                    )?;

                    csv.flush()?;
                }
            }

            Ok(())
        });

        Ok(())
    }

    pub fn stop(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    pub fn try_recv(&mut self) -> Option<AppMsg> {
        if let Ok(s) = self.rx.try_recv() {
            return Some(s);
        }

        None
    }
}
