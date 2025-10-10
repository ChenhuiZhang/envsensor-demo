use strum::AsRefStr;

pub enum SensorType {
    CO,
}

#[allow(non_camel_case_types)]
#[derive(AsRefStr)]
pub enum SensorModel {
    EC_TB600BC,
}

impl SensorModel {
    pub fn all() -> &'static [SensorModel] {
        &[SensorModel::EC_TB600BC]
    }
}
