use anyhow::Result;
use binrw::BinRead;
use std::io::Cursor;
use std::time::Duration;

use serialport::SerialPort;

use crate::sensor::{SensorChannel, SensorData, SensorDriver, SensorModel, SensorType, Unit};

#[allow(dead_code)]
#[derive(BinRead)]
#[brw(big)]
struct ReadingReply {
    addr: u8,
    cmd: u8,
    state: u8,
    unused_pm: [u8; 6],
    pm1: u16,
    pm2_5: u16,
    pm10: u16,
    checksum: u8,
}

pub struct NextPM {
    dev: Box<dyn SerialPort>,
    channels: Vec<SensorChannel>,
}

fn simple_read(
    port: &mut Box<dyn SerialPort>,
    query: &[u8],
    resp_len: usize,
) -> Result<Cursor<Vec<u8>>> {
    // Write the query command
    port.write_all(query)?;

    // Prepare a buffer for the response
    let mut serial_buf = vec![0u8; resp_len];

    // Read the exact number of bytes expected
    port.read_exact(&mut serial_buf)?;

    // Return a Cursor over the buffer
    Ok(Cursor::new(serial_buf))
}

impl NextPM {
    pub fn new(port: &str) -> Result<Self> {
        let builder = serialport::new(port, 115200)
            .data_bits(serialport::DataBits::Eight)
            .parity(serialport::Parity::Even)
            .stop_bits(serialport::StopBits::One)
            .timeout(Duration::from_secs(5));
        println!("{:?}", &builder);

        let port = builder.open().unwrap_or_else(|e| {
            eprintln!("Failed to open \"{}\". Error: {}", port, e);
            ::std::process::exit(1);
        });

        // Build channel metadata
        let channels = vec![
            SensorChannel::new(SensorType::PM1, Unit::UgPerM3),
            SensorChannel::new(SensorType::PM2_5, Unit::UgPerM3),
            SensorChannel::new(SensorType::PM10, Unit::UgPerM3),
        ];

        Ok(NextPM {
            dev: port,
            channels,
        })
    }

    pub fn read_measured_value(&mut self) -> Result<(f32, f32, f32)> {
        let mut buffer = simple_read(&mut self.dev, &[0x81, 0x11, 0x6E], 16)?;

        //TODO verify the checksum
        let value = ReadingReply::read(&mut buffer)?;

        let pm1 = value.pm1 as f32 / 10.0;
        let pm2_5 = value.pm2_5 as f32 / 10.0;
        let pm10 = value.pm10 as f32 / 10.0;

        Ok((pm1, pm2_5, pm10))
    }
}

impl SensorDriver for NextPM {
    fn new(port: &str) -> Result<Self> {
        NextPM::new(port)
    }

    fn get_metadata(&self) -> &[SensorChannel] {
        &self.channels
    }

    fn read_data(&mut self) -> Result<Vec<SensorData>> {
        let (pm1, pm2_5, pm10) = self.read_measured_value()?;

        // NextPM needs polling delay
        std::thread::sleep(std::time::Duration::from_secs(1));

        Ok(vec![
            SensorData {
                ty: self.channels[0].sensor_type,
                value: pm1,
                unit: self.channels[0].unit,
            },
            SensorData {
                ty: self.channels[1].sensor_type,
                value: pm2_5,
                unit: self.channels[1].unit,
            },
            SensorData {
                ty: self.channels[2].sensor_type,
                value: pm10,
                unit: self.channels[2].unit,
            },
        ])
    }

    fn model() -> SensorModel {
        SensorModel::TERA_NextPM
    }
}
