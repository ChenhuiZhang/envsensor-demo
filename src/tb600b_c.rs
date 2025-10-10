use crate::sensor::SensorType;
use anyhow::Result;
use binrw::BinRead;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serialport::SerialPort;
use std::io::Cursor;

#[derive(BinRead)]
#[brw(big, magic = b"\xFF\x86")]
struct AutoReport {
    concentration2: u16,
    range: u16,
    concentration1: u16,
    checksum: u8,
}

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
}

impl From<ECType> for SensorType {
    fn from(value: ECType) -> Self {
        match value {
            ECType::CO => SensorType::CO,
        }
    }
}

struct TB600BC {
    dev: Box<dyn SerialPort>,
    sensor_type: SensorType,
    scale: u32,
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
    fn new(port: &str) -> Result<Self> {
        let builder = serialport::new(port, 9600)
            .stop_bits(serialport::StopBits::One)
            .data_bits(serialport::DataBits::Eight);
        println!("{:?}", &builder);

        let mut port = builder.open().unwrap_or_else(|e| {
            eprintln!("Failed to open \"{}\". Error: {}", port, e);
            ::std::process::exit(1);
        });

        let mut buffer = simple_query(&mut port, &[0xD7], 9)?;

        let param = QueryParam2::read(&mut buffer)?;

        let sensor_type = SensorType::from(ECType::try_from(param.ty)?);

        // 0x30 >> 4 = 0x3 => 10^3
        let scale = 10_u32.pow((param.scale >> 4) as u32);

        Ok(TB600BC {
            dev: port,
            sensor_type,
            scale,
        })
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
