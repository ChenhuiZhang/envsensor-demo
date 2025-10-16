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

#[derive(Clone, Copy, Debug, AsRefStr, EnumIter)]
pub enum SensorType {
    CO,
    NO2,
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

fn start_log(
    device: SensorModel,
    flag: Arc<AtomicBool>,
    mut rx: BusReader<AppMsg>,
    sensor_type: &[SensorType],
    sensor_unit: &[&'static str],
) {
    let csv_head = format!(
        "{},{}",
        "Timestamp",
        sensor_type
            .iter()
            .zip(sensor_unit.iter())
            .map(|(t, u)| format!("{}({})", t.as_ref(), u))
            .collect::<Vec<_>>()
            .join(",")
    );

    thread::spawn(move || -> Result<()> {
        let filename = format!(
            "{}_{}.csv",
            chrono::Local::now().format("%Y-%m-%d-%H-%M-%S"),
            device.as_ref()
        );

        let mut csv = File::create(filename)?;
        // Write CSV header
        writeln!(csv, "{csv_head}")?;

        while !flag.load(Ordering::SeqCst) {
            if let Ok(AppMsg::Sample(sample)) = rx.recv() {
                writeln!(
                    csv,
                    "{},{}",
                    sample.timestamp.format("%m/%d/%Y %H:%M:%S"),
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

        let device = self.hw;

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

                    let sensor_type = sensor.get_sensor_type();
                    let sensor_unit = sensor.get_sensor_unit();

                    let f = flag.clone();
                    start_log(device, f, bus.add_rx(), &sensor_type, &sensor_unit);

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
                                ty: sensor_type[0],
                                value: c1,
                                unit: sensor_unit[0],
                            },
                            SensorData {
                                ty: sensor_type[1],
                                value: c2,
                                unit: sensor_unit[1],
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

                    let sensor_type = sensor.get_sensor_type();
                    let sensor_unit = sensor.get_sensor_unit();

                    let f = flag.clone();
                    start_log(device, f, bus.add_rx(), &sensor_type, &sensor_unit);

                    while !flag.load(Ordering::SeqCst) {
                        let now = chrono::Local::now();
                        let v = vec![SensorData {
                            ty: sensor_type[0],
                            value: sensor.read_measured_value().map_err(|e| {
                                eprintln!("Failed to read measured value: {e}");
                                e
                            })?,
                            unit: sensor_unit[0],
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
