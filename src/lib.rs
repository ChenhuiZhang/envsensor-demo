mod nextpm;
mod rydason;
pub mod sensor;
mod tb600b_c;

pub fn serial_port_list() -> Vec<String> {
    let ports = serialport::available_ports().unwrap_or_default();
    ports.into_iter().map(|p| p.port_name).collect()
}
