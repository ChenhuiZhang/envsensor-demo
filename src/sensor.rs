use std::io::Write;
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

use crate::nextpm::NextPM;
use crate::rydason::Rydason;
use crate::tb600b_c::TB600BC;

/// Metadata for a single sensor channel (type and unit)
pub struct SensorChannel {
    pub sensor_type: SensorType,
    pub unit: Unit,
}

impl SensorChannel {
    pub fn new(sensor_type: SensorType, unit: Unit) -> Self {
        Self { sensor_type, unit }
    }
}

/// Trait that all sensor drivers must implement
pub trait SensorDriver: Send + 'static {
    /// Create a new sensor instance
    fn new(port: &str) -> Result<Self>
    where
        Self: Sized;

    /// Get sensor metadata (channels with types and units)
    fn get_metadata(&self) -> &[SensorChannel];

    /// Perform sensor-specific initialization
    fn initialize(&mut self) -> Result<()> {
        Ok(()) // Default: no initialization needed
    }

    /// Read sensor data
    fn read_data(&mut self) -> Result<Vec<SensorData>>;

    /// Get the sensor model this driver handles
    fn model() -> SensorModel
    where
        Self: Sized;
}

#[derive(AsRefStr, Clone, Copy, Debug, EnumIter)]
pub enum SensorType {
    CO,
    NO2,
    PM1,
    PM2_5,
    PM10,
}

#[derive(Clone, Copy, Debug, AsRefStr)]
pub enum Unit {
    #[strum(serialize = "ppm")]
    PPM,
    #[strum(serialize = "ppb")]
    PPB,
    #[strum(serialize = "mg/m3")]
    MgPerM3,
    #[strum(serialize = "µg/m3")]
    UgPerM3,
    #[strum(serialize = "%vol")]
    PercentVol,
    #[strum(serialize = "10g/m3")]
    TenGPerM3,
}

#[allow(non_camel_case_types)]
#[derive(AsRefStr, Clone, Copy, EnumIter)]
pub enum SensorModel {
    EC_TB600BC,
    RYDASON,
    TERA_NextPM,
}

impl SensorModel {
    pub fn all() -> Vec<SensorModel> {
        SensorModel::iter().collect()
    }
}

pub struct Sensor {
    model: SensorModel,
    port: String,
    stop_flag: Arc<AtomicBool>,
    rx: BusReader<AppMsg>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct SensorData {
    pub ty: SensorType,
    pub value: f32,
    pub unit: Unit,
}

#[derive(Clone, Debug)]
pub struct SampleData {
    pub timestamp: DateTime<Local>,
    pub data: Vec<SensorData>,
}

#[derive(Clone)]
pub enum AppMsg {
    Status(String),
    Sample(SampleData),
}

pub fn spawn_log_thread(
    model: SensorModel,
    flag: Arc<AtomicBool>,
    mut rx: BusReader<AppMsg>,
    channels: &[SensorChannel],
) {
    let csv_head = format!(
        "{},{}",
        "Timestamp",
        channels
            .iter()
            .map(|ch| format!("{}({})", ch.sensor_type.as_ref(), ch.unit.as_ref()))
            .collect::<Vec<_>>()
            .join(",")
    );

    thread::spawn(move || -> Result<()> {
        let filename = format!(
            "{}_{}.csv",
            chrono::Local::now().format("%Y-%m-%d-%H-%M-%S"),
            model.as_ref()
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

pub fn spawn_sensor_thread<T: SensorDriver>(
    port: String,
    mut bus: Bus<AppMsg>,
    flag: Arc<AtomicBool>,
) {
    thread::spawn(move || -> Result<()> {
        let model = T::model();

        let mut sensor = T::new(&port).inspect_err(|e| {
            bus.broadcast(AppMsg::Status(format!(
                "Failed to create {} sensor: {e}",
                model.as_ref()
            )));
        })?;

        bus.broadcast(AppMsg::Status(format!("{} init", model.as_ref())));

        sensor.initialize().inspect_err(|e| {
            bus.broadcast(AppMsg::Status(format!(
                "Failed to initialize {}: {e}",
                model.as_ref()
            )));
        })?;

        let metadata = sensor.get_metadata();

        spawn_log_thread(model, flag.clone(), bus.add_rx(), metadata);

        while !flag.load(Ordering::SeqCst) {
            let data = sensor.read_data().map_err(|e| {
                bus.broadcast(AppMsg::Status(format!("Failed to read data: {e}")));
                e
            })?;

            bus.broadcast(AppMsg::Sample(SampleData {
                timestamp: chrono::Local::now(),
                data,
            }));
        }

        Ok(())
    });
}

impl Sensor {
    pub fn new(model: &SensorModel, port: &str, rx: BusReader<AppMsg>) -> Result<Self> {
        Ok(Sensor {
            model: *model,
            port: port.to_string(),
            stop_flag: Arc::new(AtomicBool::new(false)),
            rx,
        })
    }

    pub fn start(&self, bus: Bus<AppMsg>) -> Result<()> {
        let port = self.port.clone();
        let flag = self.stop_flag.clone();

        match self.model {
            SensorModel::EC_TB600BC => spawn_sensor_thread::<TB600BC>(port, bus, flag),
            SensorModel::RYDASON => spawn_sensor_thread::<Rydason>(port, bus, flag),
            SensorModel::TERA_NextPM => spawn_sensor_thread::<NextPM>(port, bus, flag),
        }

        Ok(())
    }

    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
    }

    pub fn try_recv(&mut self) -> Option<AppMsg> {
        if let Ok(s) = self.rx.try_recv() {
            return Some(s);
        }

        None
    }
}
