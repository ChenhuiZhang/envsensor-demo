use std::{io::Cursor, thread, time::Duration};

use anyhow::Result;
use binrw::BinRead;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serialport::SerialPort;

use crate::sensor::{SensorChannel, SensorData, SensorDriver, SensorModel, SensorType, Unit};

#[allow(dead_code)]
#[derive(BinRead)]
#[brw(big, magic = b"\xFF\x86")]
struct AutoReport {
    concentration2: u16,
    range: u16,
    concentration1: u16,
    checksum: u8,
}

#[allow(dead_code)]
#[derive(BinRead)]
#[brw(big)]
struct QueryParam1 {
    ty: u8,
    range: u16,
    unit: u8,
    reserved: [u8; 3],
    scale: u8,
    checksum: u8,
}

#[allow(dead_code)]
#[derive(BinRead)]
#[brw(big, magic = b"\xFF\xD7")]
struct QueryParam2 {
    ty: u8,
    range: u16,
    unit: u8,
    scale: u8,
    reserved: u8,
    checksum: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
enum ECType {
    CO = 0x19,
    NO2 = 0x21,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
enum ECUnit {
    PpmMg = 0x02,
    Ppbug = 0x04,
    Vol10g = 0x08,
}

impl From<ECType> for SensorType {
    fn from(value: ECType) -> Self {
        match value {
            ECType::CO => SensorType::CO,
            ECType::NO2 => SensorType::NO2,
        }
    }
}

pub struct TB600BC {
    dev: Box<dyn SerialPort>,
    scale: u32,
    channels: Vec<SensorChannel>,
}

fn simple_query(
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

impl TB600BC {
    pub fn new(port: &str) -> Result<Self> {
        let builder = serialport::new(port, 9600)
            .stop_bits(serialport::StopBits::One)
            .data_bits(serialport::DataBits::Eight)
            .timeout(Duration::from_secs(5));
        println!("{:?}", &builder);

        let mut port = builder.open().unwrap_or_else(|e| {
            eprintln!("Failed to open \"{}\". Error: {}", port, e);
            ::std::process::exit(1);
        });

        port.write_all(&[0xFF, 0x01, 0x78, 0x41, 0x00, 0x00, 0x00, 0x00, 0x46])?;

        thread::sleep(Duration::from_secs(1));

        let mut buffer = simple_query(&mut port, &[0xD7], 9)?;

        let param = QueryParam2::read(&mut buffer)?;

        let sensor_type = SensorType::from(ECType::try_from(param.ty)?);

        let sensor_unit = ECUnit::try_from(param.unit)?;

        // 0x30 >> 4 = 0x3 => 10^3
        let scale = 10_u32.pow((param.scale >> 4) as u32);

        // Build channel metadata
        let units = match sensor_unit {
            ECUnit::PpmMg => [Unit::PPM, Unit::MgPerM3],
            ECUnit::Ppbug => [Unit::PPB, Unit::UgPerM3],
            ECUnit::Vol10g => [Unit::PercentVol, Unit::TenGPerM3],
        };

        let channels = vec![
            SensorChannel::new(sensor_type, units[0]),
            SensorChannel::new(sensor_type, units[1]),
        ];

        Ok(TB600BC {
            dev: port,
            scale,
            channels,
        })
    }

    pub fn switch_mode(&mut self, auto: bool) -> Result<()> {
        if auto {
            self.dev
                .write_all(&[0xFF, 0x01, 0x78, 0x40, 0x00, 0x00, 0x00, 0x00, 0x47])?;
        } else {
            self.dev
                .write_all(&[0xFF, 0x01, 0x78, 0x41, 0x00, 0x00, 0x00, 0x00, 0x46])?;
        }

        Ok(())
    }

    pub fn read_auto_report_data(&mut self) -> Result<(f32, f32)> {
        //let mut buf: Vec<u8> = vec![0; 9];
        let mut buf = [0; 9];
        self.dev.read_exact(&mut buf)?;

        let data = AutoReport::read(&mut Cursor::new(&buf))?;

        let c1 = data.concentration1 as f32 / self.scale as f32;
        let c2 = data.concentration2 as f32 / self.scale as f32;

        Ok((c1, c2))
    }
}

impl SensorDriver for TB600BC {
    fn new(port: &str) -> Result<Self> {
        TB600BC::new(port)
    }

    fn initialize(&mut self) -> Result<()> {
        self.switch_mode(true)
    }

    fn get_metadata(&self) -> &[SensorChannel] {
        &self.channels
    }

    fn read_data(&mut self) -> Result<Vec<SensorData>> {
        let (c1, c2) = self.read_auto_report_data()?;

        Ok(vec![
            SensorData {
                ty: self.channels[0].sensor_type,
                value: c1,
                unit: self.channels[0].unit,
            },
            SensorData {
                ty: self.channels[1].sensor_type,
                value: c2,
                unit: self.channels[1].unit,
            },
        ])
    }

    fn model() -> SensorModel {
        SensorModel::EC_TB600BC
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_auto_report() {
        let mut data = Cursor::new(b"\xFF\x86\x25\xBC\x03\xE8\x20\xD0\xBE");
        let auto_report = AutoReport::read(&mut data).unwrap();

        assert_eq!(auto_report.concentration2, 0x25BC);
        assert_eq!(auto_report.range, 0x03E8);
        assert_eq!(auto_report.concentration1, 0x20D0);
    }
}
