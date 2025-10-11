use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use anyhow::Result;
use strum::AsRefStr;

use crate::tb600b_c::TB600BC;

pub enum SensorType {
    CO,
}

#[allow(non_camel_case_types)]
#[derive(AsRefStr, Clone, Copy)]
pub enum SensorModel {
    EC_TB600BC,
}

impl SensorModel {
    pub fn all() -> &'static [SensorModel] {
        &[SensorModel::EC_TB600BC]
    }
}

/*
enum SensorHW {
    EC_TB600BC(TB600BC),
}

*/
pub struct Sensor {
    hw: SensorModel,
    port: String,
    flag: Arc<AtomicBool>,
}

impl Sensor {
    pub fn new(model: &SensorModel, port: &str) -> Result<Self> {
        Ok(Sensor {
            hw: *model,
            port: port.to_string(),
            flag: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn start_sample_thread(&self) -> Result<()> {
        let port = self.port.clone();
        self.flag.store(false, Ordering::SeqCst);
        let flag = self.flag.clone();

        thread::spawn(move || -> Result<()> {
            let mut sensor = TB600BC::new(&port)?;

            sensor.switch_mode(true)?;

            while !flag.load(Ordering::SeqCst) {
                // do work here
                let (c1, c2) = sensor.read_auto_report_data()?;
                thread::sleep(Duration::from_secs(1));
            }

            Ok(())
        });

        Ok(())
    }

    pub fn stop_sample_thread(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }
}
