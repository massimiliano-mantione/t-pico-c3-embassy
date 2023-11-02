use crate::lasers::RawLaserReadings;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaceConfig {
    pub max_speed: u16,
    pub min_speed: u16,
    pub back_speed: u16,
    pub back_time: u16,
    pub alert_distance_center: u16,
    pub alert_distance_side_30: u16,
    pub alert_distance_side_60: u16,
    pub back_distance_center: u16,
    pub back_distance_side_30: u16,
    pub back_distance_side_60: u16,
    pub steer_kp_n: u16,
    pub steer_kp_d: u16,
    pub interpolation_kp_n: u16,
    pub interpolation_kp_d: u16,
    pub slope_distance_delta: u16,
    pub climbing_speed: u16,
    pub climbing_angle: u16,
    pub stillness_delta: u16,
    pub stillness_time: u16,
    pub correct_turn: u16,
    pub wrong_turn: u16,
    pub inversion_time: u16,
}

impl Default for RaceConfig {
    fn default() -> Self {
        Self::init()
    }
}

impl RaceConfig {
    pub const fn init() -> Self {
        Self {
            // max_speed: 0.6,
            max_speed: 4000,
            min_speed: 3500,
            back_speed: 8000,
            back_time: 400,
            alert_distance_center: 300,
            alert_distance_side_30: 350,
            alert_distance_side_60: 400,
            back_distance_center: 50,
            back_distance_side_30: 50,
            back_distance_side_60: 60,
            steer_kp_n: 100,
            steer_kp_d: 100,
            interpolation_kp_n: 130,
            interpolation_kp_d: 100,
            slope_distance_delta: 150,
            climbing_speed: 8000,
            climbing_angle: 10,
            stillness_delta: 0,
            stillness_time: 500,
            // correct_turn: -360.0,
            correct_turn: 0,
            wrong_turn: 270,
            inversion_time: 500,
        }
    }

    pub fn alert_distance(&self, position: LaserSidePosition) -> u16 {
        match position {
            LaserSidePosition::Center => self.alert_distance_center as u16,
            LaserSidePosition::Side30 => self.alert_distance_side_30 as u16,
            LaserSidePosition::Side60 => self.alert_distance_side_60 as u16,
        }
    }

    pub fn back_distance(&self, position: LaserSidePosition) -> u16 {
        match position {
            LaserSidePosition::Center => self.back_distance_center as u16,
            LaserSidePosition::Side30 => self.back_distance_side_30 as u16,
            LaserSidePosition::Side60 => self.back_distance_side_60 as u16,
        }
    }
}

pub const NUM_LASER_POSITIONS: usize = 5;
pub const CENTER_LASER: usize = 2;
pub const LASER_OVERFLOW: u16 = 600;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum LaserSidePosition {
    Center,
    Side30,
    Side60,
}

impl LaserSidePosition {
    pub fn index(&self, sign: u8) -> usize {
        match self {
            LaserSidePosition::Center => 2,
            LaserSidePosition::Side30 => {
                if sign > 0 {
                    3
                } else {
                    1
                }
            }
            LaserSidePosition::Side60 => {
                if sign > 0 {
                    4
                } else {
                    0
                }
            }
        }
    }

    pub fn offset(&self) -> usize {
        match self {
            LaserSidePosition::Center => 0,
            LaserSidePosition::Side30 => 1,
            LaserSidePosition::Side60 => 2,
        }
    }

    pub fn physical_index(&self, sign: i8, upper: bool) -> usize {
        if upper {
            match self {
                LaserSidePosition::Center => 2,
                LaserSidePosition::Side30 => {
                    if sign > 0 {
                        3
                    } else {
                        1
                    }
                }
                LaserSidePosition::Side60 => {
                    if sign > 0 {
                        5
                    } else {
                        4
                    }
                }
            }
        } else {
            match self {
                LaserSidePosition::Center => 7,
                LaserSidePosition::Side30 => {
                    if sign > 0 {
                        0
                    } else {
                        6
                    }
                }
                LaserSidePosition::Side60 => {
                    if sign > 0 {
                        5
                    } else {
                        4
                    }
                }
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum LaserStatus {
    Back,
    Alert,
    Regular,
    Overflow,
}

impl LaserStatus {
    pub fn from_value(value: u16, position: LaserSidePosition, config: &RaceConfig) -> Self {
        if value >= LASER_OVERFLOW {
            Self::Overflow
        } else if value > config.alert_distance(position) {
            Self::Regular
        } else if value > config.back_distance(position) {
            Self::Alert
        } else {
            Self::Back
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct LaserData {
    pub upper: u16,
    pub lower: u16,
    pub sign: i8,
    pub position: LaserSidePosition,
    pub status: LaserStatus,
    pub slope: bool,
}

impl LaserData {
    pub fn new(sign: i8, position: LaserSidePosition) -> Self {
        Self {
            upper: LASER_OVERFLOW,
            lower: LASER_OVERFLOW,
            sign,
            position,
            status: LaserStatus::Overflow,
            slope: false,
        }
    }

    pub fn value(&self) -> u16 {
        if self.slope {
            LASER_OVERFLOW
        } else {
            self.lower.min(self.upper)
        }
    }

    pub fn update(&mut self, raw_readings: &RawLaserReadings, config: &RaceConfig) {
        let lower = raw_readings.values[self.position.physical_index(self.sign, false)];
        let upper = raw_readings.values[self.position.physical_index(self.sign, true)];
        self.lower = lower;
        self.upper = upper;
        let slope_delta = config.slope_distance_delta as u16;
        self.slope = upper <= (lower + slope_delta) && upper >= (lower + slope_delta / 2);
        self.status = LaserStatus::from_value(self.value(), self.position, config);
    }

    #[allow(unused)]
    pub fn ch1(&self) -> char {
        if self.slope {
            'S'
        } else {
            match self.status {
                LaserStatus::Back => '!',
                LaserStatus::Alert => 'a',
                LaserStatus::Regular => ' ',
                LaserStatus::Overflow => '^',
            }
        }
    }
    #[allow(unused)]
    pub fn ch2(&self) -> char {
        unsafe { char::from_u32_unchecked(('0' as u32) + ((self.value() as u32 / 100) % 10)) }
    }
    #[allow(unused)]
    pub fn ch3(&self) -> char {
        unsafe { char::from_u32_unchecked(('0' as u32) + ((self.value() as u32 / 10) % 10)) }
    }
    #[allow(unused)]
    pub fn ch4(&self) -> char {
        unsafe { char::from_u32_unchecked(('0' as u32) + ((self.value() as u32 / 1) % 10)) }
    }
}

pub struct Vision {
    pub lasers: [LaserData; NUM_LASER_POSITIONS],
}

impl Vision {
    pub fn new() -> Self {
        Self {
            lasers: [
                LaserData::new(-1, LaserSidePosition::Side60),
                LaserData::new(-1, LaserSidePosition::Side30),
                LaserData::new(0, LaserSidePosition::Center),
                LaserData::new(1, LaserSidePosition::Side30),
                LaserData::new(1, LaserSidePosition::Side60),
            ],
        }
    }

    pub fn update(&mut self, raw_readings: &RawLaserReadings, config: &RaceConfig) {
        for laser in self.lasers.iter_mut() {
            laser.update(raw_readings, config);
        }
    }
}
