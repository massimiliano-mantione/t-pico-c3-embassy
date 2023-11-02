use crate::{configuration::RaceConfig, lasers::RawLaserReadings};

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
