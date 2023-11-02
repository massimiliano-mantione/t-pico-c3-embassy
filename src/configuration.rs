use crate::vision::LaserSidePosition;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
#[allow(dead_code)]
pub enum RaceConfigEntry {
    MaxSpeed,
    MinSpeed,
    BackSpeed,
    BackTime,
    AlertDistanceCenter,
    AlertDistanceSide30,
    AlertDistanceSide60,
    BackDistanceCenter,
    BackDistanceSide30,
    BackDistanceSide60,
    SteerKpN,
    SteerKpD,
    InterpolationKpN,
    InterpolationKpD,
    SlopeDistanceDelta,
    ClimbingSpeed,
    ClimbingAngle,
    StillnessDelta,
    StillnessTime,
    InversionTime,
    End,
}
pub const RACE_CONFIG_ENTRY_START: usize = 0;
pub const RACE_CONFIG_ENTRY_END: usize = RaceConfigEntry::InversionTime as usize + 1;

impl From<usize> for RaceConfigEntry {
    fn from(value: usize) -> RaceConfigEntry {
        unsafe {
            core::mem::transmute(if value < RACE_CONFIG_ENTRY_END {
                value
            } else {
                RACE_CONFIG_ENTRY_START
            })
        }
    }
}

impl Into<usize> for RaceConfigEntry {
    fn into(self) -> usize {
        unsafe { core::mem::transmute(self) }
    }
}

impl RaceConfigEntry {
    pub const fn start() -> Self {
        unsafe { core::mem::transmute(0) }
    }
    pub const fn end() -> Self {
        Self::End
    }

    pub fn index(self) -> usize {
        self.into()
    }

    pub fn prev(self) -> Self {
        let index = if self != Self::start() {
            self.index()
        } else {
            Self::end().index()
        };
        (index - 1).into()
    }

    pub fn next(self) -> Self {
        let next: Self = (self.index() + 1).into();
        if next == Self::end() {
            Self::start()
        } else {
            next
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            RaceConfigEntry::MaxSpeed => "MAX SPEED",
            RaceConfigEntry::MinSpeed => "MIN SPEED",
            RaceConfigEntry::BackSpeed => "BACK SPEED",
            RaceConfigEntry::BackTime => "BACK TIME",
            RaceConfigEntry::AlertDistanceCenter => "ALERT D  0",
            RaceConfigEntry::AlertDistanceSide30 => "ALERT D 30",
            RaceConfigEntry::AlertDistanceSide60 => "ALERT D 60",
            RaceConfigEntry::BackDistanceCenter => "BACK D  0",
            RaceConfigEntry::BackDistanceSide30 => "BACK D 30",
            RaceConfigEntry::BackDistanceSide60 => "BACK D 60",
            RaceConfigEntry::SteerKpN => "STEER KP N",
            RaceConfigEntry::SteerKpD => "STEER KP D",
            RaceConfigEntry::InterpolationKpN => "INTERP KP N",
            RaceConfigEntry::InterpolationKpD => "INTERP KP D",
            RaceConfigEntry::SlopeDistanceDelta => "SLOPE DELTA",
            RaceConfigEntry::ClimbingSpeed => "CLIMB SPD",
            RaceConfigEntry::ClimbingAngle => "CLIMB ANG",
            RaceConfigEntry::StillnessDelta => "STILL DELTA",
            RaceConfigEntry::StillnessTime => "STILL TIME",
            RaceConfigEntry::InversionTime => "INV TIME",
            RaceConfigEntry::End => "END",
        }
    }

    pub fn min(self) -> i16 {
        match self {
            RaceConfigEntry::MaxSpeed => 2000,
            RaceConfigEntry::MinSpeed => 1000,
            RaceConfigEntry::BackSpeed => 2000,
            RaceConfigEntry::BackTime => 100,
            RaceConfigEntry::AlertDistanceCenter => 200,
            RaceConfigEntry::AlertDistanceSide30 => 200,
            RaceConfigEntry::AlertDistanceSide60 => 200,
            RaceConfigEntry::BackDistanceCenter => 25,
            RaceConfigEntry::BackDistanceSide30 => 25,
            RaceConfigEntry::BackDistanceSide60 => 25,
            RaceConfigEntry::SteerKpN => 10,
            RaceConfigEntry::SteerKpD => 10,
            RaceConfigEntry::InterpolationKpN => 1,
            RaceConfigEntry::InterpolationKpD => 1,
            RaceConfigEntry::SlopeDistanceDelta => 50,
            RaceConfigEntry::ClimbingSpeed => 2000,
            RaceConfigEntry::ClimbingAngle => 5,
            RaceConfigEntry::StillnessDelta => 0,
            RaceConfigEntry::StillnessTime => 0,
            RaceConfigEntry::InversionTime => 100,
            RaceConfigEntry::End => 0,
        }
    }

    pub fn max(self) -> i16 {
        match self {
            RaceConfigEntry::MaxSpeed => 10000,
            RaceConfigEntry::MinSpeed => 9000,
            RaceConfigEntry::BackSpeed => 10000,
            RaceConfigEntry::BackTime => 1000,
            RaceConfigEntry::AlertDistanceCenter => 500,
            RaceConfigEntry::AlertDistanceSide30 => 500,
            RaceConfigEntry::AlertDistanceSide60 => 500,
            RaceConfigEntry::BackDistanceCenter => 400,
            RaceConfigEntry::BackDistanceSide30 => 400,
            RaceConfigEntry::BackDistanceSide60 => 400,
            RaceConfigEntry::SteerKpN => 1000,
            RaceConfigEntry::SteerKpD => 1000,
            RaceConfigEntry::InterpolationKpN => 1000,
            RaceConfigEntry::InterpolationKpD => 1000,
            RaceConfigEntry::SlopeDistanceDelta => 300,
            RaceConfigEntry::ClimbingSpeed => 10000,
            RaceConfigEntry::ClimbingAngle => 25,
            RaceConfigEntry::StillnessDelta => 100,
            RaceConfigEntry::StillnessTime => 100,
            RaceConfigEntry::InversionTime => 1000,
            RaceConfigEntry::End => 1,
        }
    }

    pub fn step(self) -> i16 {
        match self {
            RaceConfigEntry::MaxSpeed => 500,
            RaceConfigEntry::MinSpeed => 500,
            RaceConfigEntry::BackSpeed => 500,
            RaceConfigEntry::BackTime => 10,
            RaceConfigEntry::AlertDistanceCenter => 25,
            RaceConfigEntry::AlertDistanceSide30 => 25,
            RaceConfigEntry::AlertDistanceSide60 => 25,
            RaceConfigEntry::BackDistanceCenter => 25,
            RaceConfigEntry::BackDistanceSide30 => 25,
            RaceConfigEntry::BackDistanceSide60 => 25,
            RaceConfigEntry::SteerKpN => 10,
            RaceConfigEntry::SteerKpD => 10,
            RaceConfigEntry::InterpolationKpN => 10,
            RaceConfigEntry::InterpolationKpD => 10,
            RaceConfigEntry::SlopeDistanceDelta => 10,
            RaceConfigEntry::ClimbingSpeed => 500,
            RaceConfigEntry::ClimbingAngle => 1,
            RaceConfigEntry::StillnessDelta => 1,
            RaceConfigEntry::StillnessTime => 1,
            RaceConfigEntry::InversionTime => 10,
            RaceConfigEntry::End => 1,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RaceConfig {
    pub max_speed: i16,
    pub min_speed: i16,
    pub back_speed: i16,
    pub back_time: i16,
    pub alert_distance_center: i16,
    pub alert_distance_side_30: i16,
    pub alert_distance_side_60: i16,
    pub back_distance_center: i16,
    pub back_distance_side_30: i16,
    pub back_distance_side_60: i16,
    pub steer_kp_n: i16,
    pub steer_kp_d: i16,
    pub interpolation_kp_n: i16,
    pub interpolation_kp_d: i16,
    pub slope_distance_delta: i16,
    pub climbing_speed: i16,
    pub climbing_angle: i16,
    pub stillness_delta: i16,
    pub stillness_time: i16,
    pub inversion_time: i16,
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

    #[allow(unused)]
    pub fn reset(&mut self, entry: RaceConfigEntry) {
        match entry {
            RaceConfigEntry::MaxSpeed => self.max_speed = Self::init().max_speed,
            RaceConfigEntry::MinSpeed => self.min_speed = Self::init().min_speed,
            RaceConfigEntry::BackSpeed => self.back_speed = Self::init().back_speed,
            RaceConfigEntry::BackTime => self.back_time = Self::init().back_time,
            RaceConfigEntry::AlertDistanceCenter => {
                self.alert_distance_center = Self::init().alert_distance_center
            }
            RaceConfigEntry::AlertDistanceSide30 => {
                self.alert_distance_side_30 = Self::init().alert_distance_side_30
            }
            RaceConfigEntry::AlertDistanceSide60 => {
                self.alert_distance_side_60 = Self::init().alert_distance_side_60
            }
            RaceConfigEntry::BackDistanceCenter => {
                self.back_distance_center = Self::init().back_distance_center
            }
            RaceConfigEntry::BackDistanceSide30 => {
                self.back_distance_side_30 = Self::init().back_distance_side_30
            }
            RaceConfigEntry::BackDistanceSide60 => {
                self.back_distance_side_60 = Self::init().back_distance_side_60
            }
            RaceConfigEntry::SteerKpN => self.steer_kp_n = Self::init().steer_kp_n,
            RaceConfigEntry::SteerKpD => self.steer_kp_d = Self::init().steer_kp_d,
            RaceConfigEntry::InterpolationKpN => {
                self.interpolation_kp_n = Self::init().interpolation_kp_n
            }
            RaceConfigEntry::InterpolationKpD => {
                self.interpolation_kp_d = Self::init().interpolation_kp_d
            }
            RaceConfigEntry::SlopeDistanceDelta => {
                self.slope_distance_delta = Self::init().slope_distance_delta
            }
            RaceConfigEntry::ClimbingSpeed => self.climbing_speed = Self::init().climbing_speed,
            RaceConfigEntry::ClimbingAngle => self.climbing_angle = Self::init().climbing_angle,
            RaceConfigEntry::StillnessDelta => self.stillness_delta = Self::init().stillness_delta,
            RaceConfigEntry::StillnessTime => self.stillness_time = Self::init().stillness_time,
            RaceConfigEntry::InversionTime => self.inversion_time = Self::init().inversion_time,
            RaceConfigEntry::End => {}
        }
    }

    pub fn get(&self, entry: RaceConfigEntry) -> i16 {
        match entry {
            RaceConfigEntry::MaxSpeed => self.max_speed,
            RaceConfigEntry::MinSpeed => self.min_speed,
            RaceConfigEntry::BackSpeed => self.back_speed,
            RaceConfigEntry::BackTime => self.back_time,
            RaceConfigEntry::AlertDistanceCenter => self.alert_distance_center,
            RaceConfigEntry::AlertDistanceSide30 => self.alert_distance_side_30,
            RaceConfigEntry::AlertDistanceSide60 => self.alert_distance_side_60,
            RaceConfigEntry::BackDistanceCenter => self.back_distance_center,
            RaceConfigEntry::BackDistanceSide30 => self.back_distance_side_30,
            RaceConfigEntry::BackDistanceSide60 => self.back_distance_side_60,
            RaceConfigEntry::SteerKpN => self.steer_kp_n,
            RaceConfigEntry::SteerKpD => self.steer_kp_d,
            RaceConfigEntry::InterpolationKpN => self.interpolation_kp_n,
            RaceConfigEntry::InterpolationKpD => self.interpolation_kp_d,
            RaceConfigEntry::SlopeDistanceDelta => self.slope_distance_delta,
            RaceConfigEntry::ClimbingSpeed => self.climbing_speed,
            RaceConfigEntry::ClimbingAngle => self.climbing_angle,
            RaceConfigEntry::StillnessDelta => self.stillness_delta,
            RaceConfigEntry::StillnessTime => self.stillness_time,
            RaceConfigEntry::InversionTime => self.inversion_time,
            RaceConfigEntry::End => 0,
        }
    }

    pub fn set(&mut self, entry: RaceConfigEntry, value: i16) {
        match entry {
            RaceConfigEntry::MaxSpeed => self.max_speed = value,
            RaceConfigEntry::MinSpeed => self.min_speed = value,
            RaceConfigEntry::BackSpeed => self.back_speed = value,
            RaceConfigEntry::BackTime => self.back_time = value,
            RaceConfigEntry::AlertDistanceCenter => self.alert_distance_center = value,
            RaceConfigEntry::AlertDistanceSide30 => self.alert_distance_side_30 = value,
            RaceConfigEntry::AlertDistanceSide60 => self.alert_distance_side_60 = value,
            RaceConfigEntry::BackDistanceCenter => self.back_distance_center = value,
            RaceConfigEntry::BackDistanceSide30 => self.back_distance_side_30 = value,
            RaceConfigEntry::BackDistanceSide60 => self.back_distance_side_60 = value,
            RaceConfigEntry::SteerKpN => self.steer_kp_n = value,
            RaceConfigEntry::SteerKpD => self.steer_kp_d = value,
            RaceConfigEntry::InterpolationKpN => self.interpolation_kp_n = value,
            RaceConfigEntry::InterpolationKpD => self.interpolation_kp_d = value,
            RaceConfigEntry::SlopeDistanceDelta => self.slope_distance_delta = value,
            RaceConfigEntry::ClimbingSpeed => self.climbing_speed = value,
            RaceConfigEntry::ClimbingAngle => self.climbing_angle = value,
            RaceConfigEntry::StillnessDelta => self.stillness_delta = value,
            RaceConfigEntry::StillnessTime => self.stillness_time = value,
            RaceConfigEntry::InversionTime => self.inversion_time = value,
            RaceConfigEntry::End => {}
        }
    }

    pub fn inc(&mut self, entry: RaceConfigEntry) -> i16 {
        let value = (self.get(entry) + entry.step()).min(entry.max());
        self.set(entry, value);
        value
    }

    pub fn dec(&mut self, entry: RaceConfigEntry) -> i16 {
        let value = (self.get(entry) - entry.step()).max(entry.min());
        self.set(entry, value);
        value
    }
}
