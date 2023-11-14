use embassy_time::Duration;

use crate::{race::Angle, vision::LaserSidePosition};

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
#[allow(dead_code)]
pub enum RaceConfigEntry {
    MaxSpeed,
    MinSpeed,
    SteerBias,
    SafeAngle,
    BackSpeed,
    BackTime,
    SprintSpeed,
    SprintTime,
    AlertDistanceCenter,
    AlertDistanceSide30,
    AlertDistanceSide60,
    BackDistanceCenter,
    BackDistanceSide30,
    BackDistanceSide60,
    SlopeDistanceDelta,
    ClimbingSpeed,
    ClimbingAngle,
    ClimbingIgnore,
    StillnessDelta,
    StillnessTime,
    UseStillness,
    InversionTime,
    ClimbDirection,
    UseClimbDirection,
    UseColorInversion,
    End,
}
pub const RACE_CONFIG_ENTRY_START: usize = 0;
pub const RACE_CONFIG_ENTRY_END: usize = RaceConfigEntry::End as usize;

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
            RaceConfigEntry::SteerBias => "STEER BIAS",
            RaceConfigEntry::SafeAngle => "SAFE ANGLE",
            RaceConfigEntry::BackSpeed => "BACK SPEED",
            RaceConfigEntry::BackTime => "BACK TIME",
            RaceConfigEntry::SprintSpeed => "SPRINT SPEED",
            RaceConfigEntry::SprintTime => "SPRINT TIME",
            RaceConfigEntry::AlertDistanceCenter => "ALERT D  0",
            RaceConfigEntry::AlertDistanceSide30 => "ALERT D 30",
            RaceConfigEntry::AlertDistanceSide60 => "ALERT D 60",
            RaceConfigEntry::BackDistanceCenter => "BACK D  0",
            RaceConfigEntry::BackDistanceSide30 => "BACK D 30",
            RaceConfigEntry::BackDistanceSide60 => "BACK D 60",
            RaceConfigEntry::SlopeDistanceDelta => "SLOPE DELTA",
            RaceConfigEntry::ClimbingSpeed => "CLIMB SPD",
            RaceConfigEntry::ClimbingAngle => "CLIMB ANG",
            RaceConfigEntry::ClimbingIgnore => "CLIMB IGN",
            RaceConfigEntry::StillnessDelta => "STILL DELTA",
            RaceConfigEntry::StillnessTime => "STILL TIME",
            RaceConfigEntry::UseStillness => "USE STILL",
            RaceConfigEntry::InversionTime => "INV TIME",
            RaceConfigEntry::ClimbDirection => "CLIMB DIR",
            RaceConfigEntry::UseClimbDirection => "USE CLIMB DIR",
            RaceConfigEntry::UseColorInversion => "USE COLOR INV",
            RaceConfigEntry::End => "END",
        }
    }

    pub fn min(self) -> i16 {
        match self {
            RaceConfigEntry::MaxSpeed => 2000,
            RaceConfigEntry::MinSpeed => 1000,
            RaceConfigEntry::SteerBias => 0,
            RaceConfigEntry::SafeAngle => 0,
            RaceConfigEntry::BackSpeed => 2000,
            RaceConfigEntry::BackTime => 100,
            RaceConfigEntry::SprintSpeed => 2000,
            RaceConfigEntry::SprintTime => 0,
            RaceConfigEntry::AlertDistanceCenter => 200,
            RaceConfigEntry::AlertDistanceSide30 => 200,
            RaceConfigEntry::AlertDistanceSide60 => 200,
            RaceConfigEntry::BackDistanceCenter => 25,
            RaceConfigEntry::BackDistanceSide30 => 25,
            RaceConfigEntry::BackDistanceSide60 => 25,
            RaceConfigEntry::SlopeDistanceDelta => 50,
            RaceConfigEntry::ClimbingSpeed => 2000,
            RaceConfigEntry::ClimbingAngle => 5,
            RaceConfigEntry::ClimbingIgnore => 0,
            RaceConfigEntry::StillnessDelta => 0,
            RaceConfigEntry::StillnessTime => 0,
            RaceConfigEntry::UseStillness => 0,
            RaceConfigEntry::InversionTime => 100,
            RaceConfigEntry::ClimbDirection => -180,
            RaceConfigEntry::UseClimbDirection => 0,
            RaceConfigEntry::UseColorInversion => 0,
            RaceConfigEntry::End => 0,
        }
    }

    pub fn max(self) -> i16 {
        match self {
            RaceConfigEntry::MaxSpeed => 10000,
            RaceConfigEntry::MinSpeed => 9000,
            RaceConfigEntry::SteerBias => 20,
            RaceConfigEntry::SafeAngle => 35,
            RaceConfigEntry::BackSpeed => 10000,
            RaceConfigEntry::BackTime => 1000,
            RaceConfigEntry::SprintSpeed => 10000,
            RaceConfigEntry::SprintTime => 3000,
            RaceConfigEntry::AlertDistanceCenter => 500,
            RaceConfigEntry::AlertDistanceSide30 => 500,
            RaceConfigEntry::AlertDistanceSide60 => 500,
            RaceConfigEntry::BackDistanceCenter => 400,
            RaceConfigEntry::BackDistanceSide30 => 400,
            RaceConfigEntry::BackDistanceSide60 => 400,
            RaceConfigEntry::SlopeDistanceDelta => 300,
            RaceConfigEntry::ClimbingSpeed => 10000,
            RaceConfigEntry::ClimbingAngle => 25,
            RaceConfigEntry::ClimbingIgnore => 10,
            RaceConfigEntry::StillnessDelta => 100,
            RaceConfigEntry::StillnessTime => 100,
            RaceConfigEntry::UseStillness => 1,
            RaceConfigEntry::InversionTime => 1000,
            RaceConfigEntry::ClimbDirection => 180,
            RaceConfigEntry::UseClimbDirection => 1,
            RaceConfigEntry::UseColorInversion => 1,
            RaceConfigEntry::End => 1,
        }
    }

    pub fn step(self) -> i16 {
        match self {
            RaceConfigEntry::MaxSpeed => 500,
            RaceConfigEntry::MinSpeed => 500,
            RaceConfigEntry::SteerBias => 1,
            RaceConfigEntry::SafeAngle => 1,
            RaceConfigEntry::BackSpeed => 500,
            RaceConfigEntry::BackTime => 10,
            RaceConfigEntry::SprintSpeed => 500,
            RaceConfigEntry::SprintTime => 10,
            RaceConfigEntry::AlertDistanceCenter => 25,
            RaceConfigEntry::AlertDistanceSide30 => 25,
            RaceConfigEntry::AlertDistanceSide60 => 25,
            RaceConfigEntry::BackDistanceCenter => 25,
            RaceConfigEntry::BackDistanceSide30 => 25,
            RaceConfigEntry::BackDistanceSide60 => 25,
            RaceConfigEntry::SlopeDistanceDelta => 10,
            RaceConfigEntry::ClimbingSpeed => 500,
            RaceConfigEntry::ClimbingAngle => 1,
            RaceConfigEntry::ClimbingIgnore => 1,
            RaceConfigEntry::StillnessDelta => 1,
            RaceConfigEntry::StillnessTime => 1,
            RaceConfigEntry::UseStillness => 1,
            RaceConfigEntry::InversionTime => 10,
            RaceConfigEntry::ClimbDirection => 90,
            RaceConfigEntry::UseClimbDirection => 1,
            RaceConfigEntry::UseColorInversion => 1,
            RaceConfigEntry::End => 1,
        }
    }

    pub fn value_name(self, value: i16) -> Option<&'static str> {
        match self {
            RaceConfigEntry::MaxSpeed => None,
            RaceConfigEntry::MinSpeed => None,
            RaceConfigEntry::SteerBias => None,
            RaceConfigEntry::SafeAngle => None,
            RaceConfigEntry::BackSpeed => None,
            RaceConfigEntry::BackTime => None,
            RaceConfigEntry::SprintSpeed => None,
            RaceConfigEntry::SprintTime => None,
            RaceConfigEntry::AlertDistanceCenter => None,
            RaceConfigEntry::AlertDistanceSide30 => None,
            RaceConfigEntry::AlertDistanceSide60 => None,
            RaceConfigEntry::BackDistanceCenter => None,
            RaceConfigEntry::BackDistanceSide30 => None,
            RaceConfigEntry::BackDistanceSide60 => None,
            RaceConfigEntry::SlopeDistanceDelta => None,
            RaceConfigEntry::ClimbingSpeed => None,
            RaceConfigEntry::ClimbingAngle => None,
            RaceConfigEntry::ClimbingIgnore => None,
            RaceConfigEntry::StillnessDelta => None,
            RaceConfigEntry::StillnessTime => None,
            RaceConfigEntry::UseStillness => match value {
                0 => Some("NO"),
                1 => Some("YES"),
                _ => None,
            },
            RaceConfigEntry::InversionTime => None,
            RaceConfigEntry::ClimbDirection => None,
            RaceConfigEntry::UseClimbDirection => match value {
                0 => Some("NO"),
                1 => Some("YES"),
                _ => None,
            },
            RaceConfigEntry::UseColorInversion => match value {
                0 => Some("NO"),
                1 => Some("YES"),
                _ => None,
            },
            RaceConfigEntry::End => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RaceConfig {
    pub max_speed: i16,
    pub min_speed: i16,
    pub steer_bias: i16,
    pub safe_angle: i16,
    pub back_speed: i16,
    pub back_time: i16,
    pub sprint_speed: i16,
    pub sprint_time: i16,
    pub alert_distance_center: i16,
    pub alert_distance_side_30: i16,
    pub alert_distance_side_60: i16,
    pub back_distance_center: i16,
    pub back_distance_side_30: i16,
    pub back_distance_side_60: i16,
    pub slope_distance_delta: i16,
    pub climbing_speed: i16,
    pub climbing_angle: i16,
    pub climbing_ignore: i16,
    pub stillness_delta: i16,
    pub stillness_time: i16,
    pub use_stillness: i16,
    pub inversion_time: i16,
    pub climb_direction: i16,
    pub use_climb_direction: i16,
    pub use_color_inversion: i16,
}

impl Default for RaceConfig {
    fn default() -> Self {
        Self::init()
    }
}

impl RaceConfig {
    pub const fn init() -> Self {
        Self {
            max_speed: 3900,
            min_speed: 3600,
            steer_bias: 0,
            safe_angle: 10,
            back_speed: 5000,
            back_time: 100,
            sprint_speed: 6000,
            sprint_time: 100,
            alert_distance_center: 380,
            alert_distance_side_30: 190,
            alert_distance_side_60: 150,
            back_distance_center: 80,
            back_distance_side_30: 60,
            back_distance_side_60: 50,
            slope_distance_delta: 150,
            climbing_speed: 8000,
            climbing_angle: 15,
            climbing_ignore: 3,
            stillness_delta: 0,
            stillness_time: 500,
            use_stillness: 1,
            inversion_time: 750,
            climb_direction: 0,
            use_climb_direction: 1,
            use_color_inversion: 1,
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

    pub fn climb_power_boost(&self, pitch: Angle) -> i16 {
        if pitch <= Angle::ZERO {
            0
        } else {
            let boost_range = (self.climbing_speed - self.max_speed) as i32;
            let pitch_range = (self.climbing_angle - self.climbing_ignore) as i32;
            let pitch_delta = (pitch.value() - (self.climbing_ignore as i32))
                .min(pitch_range)
                .max(0);
            let boost = pitch_delta * boost_range / pitch_range;
            boost.min(boost_range) as i16
        }
    }

    pub fn turn_speed(&self, steer: Angle) -> i16 {
        let safe_angle = self.safe_angle as i32;
        let steer = steer
            .value()
            .max(-Angle::MAX_STEER.value())
            .min(Angle::MAX_STEER.value());
        let speed_range = (self.max_speed - self.min_speed) as i32;
        let steer_range = Angle::MAX_STEER.value() - safe_angle;
        let steer_delta = if steer < -safe_angle {
            -(steer + safe_angle)
        } else if steer > safe_angle {
            steer - safe_angle
        } else {
            0
        };
        let speed_delta = steer_delta * speed_range / steer_range;
        let speed_delta = speed_delta as i16;
        self.max_speed - speed_delta
    }

    pub fn detect_climb(&self, pitch: Angle) -> bool {
        let climbing_threshold: Angle = (self.climbing_angle as i32 * 2 / 3).into();
        pitch >= climbing_threshold
    }

    pub fn detect_downhill(&self, pitch: Angle) -> bool {
        let climbing_threshold: Angle = (self.climbing_angle as i32 * 2 / 3).into();
        pitch <= -climbing_threshold
    }

    pub fn detect_stillness(&self, stillness_time: Option<Duration>) -> bool {
        stillness_time
            .map(|time| time >= Duration::from_millis(self.stillness_time as u64))
            .unwrap_or(false)
    }

    pub fn climb_direction(&self) -> Angle {
        (self.climb_direction as i32).into()
    }

    pub fn use_climb_direction(&self) -> bool {
        self.use_climb_direction != 0
    }

    #[allow(unused)]
    pub fn reset(&mut self, entry: RaceConfigEntry) {
        match entry {
            RaceConfigEntry::MaxSpeed => self.max_speed = Self::init().max_speed,
            RaceConfigEntry::MinSpeed => self.min_speed = Self::init().min_speed,
            RaceConfigEntry::SteerBias => self.steer_bias = Self::init().steer_bias,
            RaceConfigEntry::SafeAngle => self.safe_angle = Self::init().safe_angle,
            RaceConfigEntry::BackSpeed => self.back_speed = Self::init().back_speed,
            RaceConfigEntry::BackTime => self.back_time = Self::init().back_time,
            RaceConfigEntry::SprintSpeed => self.sprint_speed = Self::init().sprint_speed,
            RaceConfigEntry::SprintTime => self.sprint_time = Self::init().sprint_time,
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
            RaceConfigEntry::SlopeDistanceDelta => {
                self.slope_distance_delta = Self::init().slope_distance_delta
            }
            RaceConfigEntry::ClimbingSpeed => self.climbing_speed = Self::init().climbing_speed,
            RaceConfigEntry::ClimbingAngle => self.climbing_angle = Self::init().climbing_angle,
            RaceConfigEntry::ClimbingIgnore => self.climbing_ignore = Self::init().climbing_ignore,
            RaceConfigEntry::StillnessDelta => self.stillness_delta = Self::init().stillness_delta,
            RaceConfigEntry::StillnessTime => self.stillness_time = Self::init().stillness_time,
            RaceConfigEntry::UseStillness => self.use_stillness = Self::init().use_stillness,
            RaceConfigEntry::InversionTime => self.inversion_time = Self::init().inversion_time,
            RaceConfigEntry::ClimbDirection => self.climb_direction = Self::init().climb_direction,
            RaceConfigEntry::UseClimbDirection => {
                self.use_climb_direction = Self::init().use_climb_direction
            }
            RaceConfigEntry::UseColorInversion => {
                self.use_color_inversion = Self::init().use_color_inversion
            }
            RaceConfigEntry::End => {}
        }
    }

    pub fn get(&self, entry: RaceConfigEntry) -> i16 {
        match entry {
            RaceConfigEntry::MaxSpeed => self.max_speed,
            RaceConfigEntry::MinSpeed => self.min_speed,
            RaceConfigEntry::SteerBias => self.steer_bias,
            RaceConfigEntry::SafeAngle => self.safe_angle,
            RaceConfigEntry::BackSpeed => self.back_speed,
            RaceConfigEntry::BackTime => self.back_time,
            RaceConfigEntry::SprintSpeed => self.sprint_speed,
            RaceConfigEntry::SprintTime => self.sprint_time,
            RaceConfigEntry::AlertDistanceCenter => self.alert_distance_center,
            RaceConfigEntry::AlertDistanceSide30 => self.alert_distance_side_30,
            RaceConfigEntry::AlertDistanceSide60 => self.alert_distance_side_60,
            RaceConfigEntry::BackDistanceCenter => self.back_distance_center,
            RaceConfigEntry::BackDistanceSide30 => self.back_distance_side_30,
            RaceConfigEntry::BackDistanceSide60 => self.back_distance_side_60,
            RaceConfigEntry::SlopeDistanceDelta => self.slope_distance_delta,
            RaceConfigEntry::ClimbingSpeed => self.climbing_speed,
            RaceConfigEntry::ClimbingAngle => self.climbing_angle,
            RaceConfigEntry::ClimbingIgnore => self.climbing_ignore,
            RaceConfigEntry::StillnessDelta => self.stillness_delta,
            RaceConfigEntry::StillnessTime => self.stillness_time,
            RaceConfigEntry::UseStillness => self.use_stillness,
            RaceConfigEntry::InversionTime => self.inversion_time,
            RaceConfigEntry::ClimbDirection => self.climb_direction,
            RaceConfigEntry::UseClimbDirection => self.use_climb_direction,
            RaceConfigEntry::UseColorInversion => self.use_color_inversion,
            RaceConfigEntry::End => 0,
        }
    }

    pub fn set(&mut self, entry: RaceConfigEntry, value: i16) {
        match entry {
            RaceConfigEntry::MaxSpeed => self.max_speed = value,
            RaceConfigEntry::MinSpeed => self.min_speed = value,
            RaceConfigEntry::SteerBias => self.steer_bias = value,
            RaceConfigEntry::SafeAngle => self.safe_angle = value,
            RaceConfigEntry::BackSpeed => self.back_speed = value,
            RaceConfigEntry::BackTime => self.back_time = value,
            RaceConfigEntry::SprintSpeed => self.sprint_speed = value,
            RaceConfigEntry::SprintTime => self.sprint_time = value,
            RaceConfigEntry::AlertDistanceCenter => self.alert_distance_center = value,
            RaceConfigEntry::AlertDistanceSide30 => self.alert_distance_side_30 = value,
            RaceConfigEntry::AlertDistanceSide60 => self.alert_distance_side_60 = value,
            RaceConfigEntry::BackDistanceCenter => self.back_distance_center = value,
            RaceConfigEntry::BackDistanceSide30 => self.back_distance_side_30 = value,
            RaceConfigEntry::BackDistanceSide60 => self.back_distance_side_60 = value,
            RaceConfigEntry::SlopeDistanceDelta => self.slope_distance_delta = value,
            RaceConfigEntry::ClimbingSpeed => self.climbing_speed = value,
            RaceConfigEntry::ClimbingAngle => self.climbing_angle = value,
            RaceConfigEntry::ClimbingIgnore => self.climbing_ignore = value,
            RaceConfigEntry::StillnessDelta => self.stillness_delta = value,
            RaceConfigEntry::StillnessTime => self.stillness_time = value,
            RaceConfigEntry::UseStillness => self.use_stillness = value,
            RaceConfigEntry::InversionTime => self.inversion_time = value,
            RaceConfigEntry::ClimbDirection => self.climb_direction = value,
            RaceConfigEntry::UseClimbDirection => self.use_climb_direction = value,
            RaceConfigEntry::UseColorInversion => self.use_color_inversion = value,
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
