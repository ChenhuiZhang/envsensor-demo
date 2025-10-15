use std::{io::Cursor, time::Duration};

use anyhow::{Result, anyhow};
use binrw::BinRead;
use binrw::BinWrite;
use binrw::binwrite;
use crc::Crc;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serialport::SerialPort;

use crate::sensor::SensorType;

const CRC_16_MODBUS: Crc<u16> = Crc::<u16>::new(&crc::CRC_16_MODBUS);

#[derive(Debug, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u16)]
enum RydasonType {
    CO = 1,
}

impl From<RydasonType> for SensorType {
    fn from(value: RydasonType) -> Self {
        match value {
            RydasonType::CO => SensorType::CO,
        }
    }
}

#[derive(Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u16)]
enum RydasonUnit {
    PPB = 1,
    PPM = 2,
}

#[binwrite]
#[brw(big)]
struct QueryReq {
    addr: u8,
    func: u8,
    reg: u16,
    value: u16,

    #[bw(calc = CRC_16_MODBUS.checksum(&[
        *addr,
	*func,
        (reg >> 8) as u8,
        (reg & 0xFF) as u8,
        (value >> 8) as u8,
        (value & 0xFF) as u8
    ]))]
    #[brw(little)]
    crc: u16,
}

#[derive(Debug, BinRead)]
#[br(import(len: u8))]
enum Value {
    #[br(pre_assert(len == 2))]
    U16(#[brw(big)] u16),

    #[br(pre_assert(len == 4))]
    U32(#[brw(big)] u32),
}

impl Value {
    pub fn as_u32(&self) -> Result<u32> {
        match self {
            Value::U16(_) => Err(anyhow!("Unexpected 16-bit value in this context")),
            Value::U32(v) => Ok(*v),
        }
    }

    pub fn as_u16(&self) -> Result<u16> {
        match self {
            Value::U16(v) => Ok(*v),
            Value::U32(_) => Err(anyhow!("Unexpected 32-bit value in this context")),
        }
    }
}

#[allow(dead_code)]
#[derive(BinRead)]
#[brw(big)]
struct QueryRsp {
    addr: u8,
    func: u8,
    len: u8,
    #[br(args(len))]
    value: Value,
    #[brw(little)]
    checksum: u16,
}

pub struct Rydason {
    dev: Box<dyn SerialPort>,
    addr: u8,
    sensor_type: SensorType,
    scale: u32,
}

fn query(port: &mut Box<dyn SerialPort>, req: &QueryReq, len: usize) -> Result<QueryRsp> {
    let mut buf = Cursor::new(Vec::new());
    req.write(&mut buf)?;
    port.write_all(buf.get_ref())?;

    let mut buf = vec![0u8; len];
    port.read_exact(&mut buf)?;

    Ok(QueryRsp::read(&mut Cursor::new(buf))?)
}

fn read_type(port: &mut Box<dyn SerialPort>, addr: u8) -> Result<SensorType> {
    let req = QueryReq {
        addr,
        func: 0x03,
        reg: 0x0101,
        value: 0x0001,
    };

    let rsp = query(port, &req, 7)?;

    Ok(SensorType::from(RydasonType::try_from(
        rsp.value.as_u16()?,
    )?))
}

fn read_unit(port: &mut Box<dyn SerialPort>, addr: u8) -> Result<RydasonUnit> {
    let req = QueryReq {
        addr,
        func: 0x03,
        reg: 0x0102,
        value: 0x0001,
    };

    let rsp = query(port, &req, 7)?;

    Ok(RydasonUnit::try_from(rsp.value.as_u16()?)?)
}

fn read_scale(port: &mut Box<dyn SerialPort>, addr: u8) -> Result<u32> {
    let req = QueryReq {
        addr,
        func: 0x03,
        reg: 0x0103,
        value: 0x0001,
    };

    let rsp = query(port, &req, 7)?;

    Ok(10_u32.pow(rsp.value.as_u16()? as u32))
}

impl Rydason {
    pub fn new(port: &str, addr: u8) -> Result<Self> {
        let builder = serialport::new(port, 9600)
            .stop_bits(serialport::StopBits::One)
            .data_bits(serialport::DataBits::Eight)
            .parity(serialport::Parity::Even)
            .timeout(Duration::from_secs(5));
        println!("{:?}", &builder);

        let mut port = builder.open().inspect_err(|e| {
            eprintln!("Failed to open \"{}\". Error: {}", port, e);
        })?;

        let sensor_type = read_type(&mut port, addr)?;

        let sensor_unit = read_unit(&mut port, addr)?;

        let scale = read_scale(&mut port, addr)?;

        Ok(Rydason {
            dev: port,
            addr,
            sensor_type,
            scale,
        })
    }

    pub fn read_measured_value(&mut self) -> Result<f32> {
        let req = QueryReq {
            addr: self.addr,
            func: 0x03,
            reg: 0x0108,
            value: 0x0002,
        };

        let rsp = query(&mut self.dev, &req, 9)?;

        Ok(rsp.value.as_u32()? as f32 / self.scale as f32)
    }
}
