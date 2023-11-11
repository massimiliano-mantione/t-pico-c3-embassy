use embassy_futures::join::join;
use embassy_futures::select::{select3, Either3};
use embassy_time::{Duration, Instant};

use crate::cmd::{Cmd, CMD};
use crate::imu::IMU_DATA;
use crate::lasers::RAW_LASER_READINGS;
use crate::lcd::VISUAL_STATE;
use crate::motors::motors_go;
use crate::screens::Screen;
use crate::vision::LaserStatus;
use crate::{configuration::RaceConfig, lcd::VisualState, vision::Vision};

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum Direction {
    Start = 0,
    Right = 1,
    Back = 2,
    Left = 3,
}

impl From<i16> for Direction {
    fn from(value: i16) -> Self {
        unsafe { core::mem::transmute((value as i8) % 4) }
    }
}

impl From<i8> for Direction {
    fn from(value: i8) -> Self {
        unsafe { core::mem::transmute(value % 4) }
    }
}

impl Into<i8> for Direction {
    fn into(self) -> i8 {
        unsafe { core::mem::transmute(self) }
    }
}

impl Direction {
    pub fn right(self) -> Self {
        let v: i8 = self.into();
        (v + 1).into()
    }

    pub fn left(self) -> Self {
        let v: i8 = self.into();
        (v + 5).into()
    }

    pub fn go_right(&mut self) {
        *self = self.right()
    }

    pub fn go_left(&mut self) {
        *self = self.left()
    }

    pub fn inversion(self) -> Self {
        let v: i8 = self.into();
        (v + 2).into()
    }

    pub fn invert(&mut self) {
        *self = self.inversion()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Angle {
    value: i32,
}

impl From<i32> for Angle {
    fn from(value: i32) -> Self {
        Self { value }.normalize()
    }
}

impl Into<i32> for Angle {
    fn into(self) -> i32 {
        self.value
    }
}

impl Into<i16> for Angle {
    fn into(self) -> i16 {
        self.value as i16
    }
}

impl Angle {
    fn normalize(self) -> Self {
        let mut result = self;
        while result.value > 180 {
            result.value -= 360;
        }
        while result.value < -180 {
            result.value += 360;
        }
        result
    }

    pub fn from_imu_value(imu_value: i16) -> Self {
        Self {
            value: imu_value as i32 / 100,
        }
    }

    pub fn value(self) -> i32 {
        self.into()
    }

    pub const ZERO: Self = Self { value: 0 };
    pub const R90: Self = Self { value: 90 };
    pub const L90: Self = Self { value: -90 };
    pub const BACK: Self = Self { value: 180 };
    pub const L45: Self = Self { value: -45 };
    pub const R45: Self = Self { value: 45 };

    pub const SMALL: Self = Self { value: 5 };

    pub const SLL: Self = Self { value: -60 };
    pub const SL: Self = Self { value: -30 };
    pub const SC: Self = Self { value: 0 };
    pub const SR: Self = Self { value: 30 };
    pub const SRR: Self = Self { value: 60 };
    pub const SHALF: Self = Self { value: 15 };

    pub const TILT_ALERT: Self = Self { value: 45 };

    pub const MAX_STEER: Self = Self { value: 35 };
    pub const MIN_STEER: Self = Self { value: -35 };
}

impl core::ops::Add for Angle {
    type Output = Angle;

    fn add(self, rhs: Self) -> Self::Output {
        Self::from(self.value + rhs.value)
    }
}

impl core::ops::AddAssign for Angle {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl core::ops::Sub for Angle {
    type Output = Angle;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::from(self.value - rhs.value)
    }
}

impl core::ops::SubAssign for Angle {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl core::ops::Neg for Angle {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self { value: -self.value }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum TrackSide {
    Left = 0,
    Right = 1,
}

impl From<i8> for TrackSide {
    fn from(value: i8) -> Self {
        unsafe { core::mem::transmute(value % 2) }
    }
}

impl Into<i8> for TrackSide {
    fn into(self) -> i8 {
        unsafe { core::mem::transmute(self) }
    }
}

impl From<i16> for TrackSide {
    fn from(value: i16) -> Self {
        (value as i8).into()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RaceAction {
    pub power: i16,
    pub steer: Angle,
}

fn detect_tilt_alert(pitch: Angle, roll: Angle) -> bool {
    pitch < -Angle::TILT_ALERT
        || pitch > Angle::TILT_ALERT
        || roll < -Angle::TILT_ALERT
        || roll > Angle::TILT_ALERT
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct BackSteering {
    pub remaining_time: Duration,
    pub steer: Angle,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct RouteTarget {
    pub remaining_time: Duration,
    pub go_back: bool,
    pub start: Angle,
    pub target: Angle,
}

pub async fn race(config: &RaceConfig, start_angle: Angle, simulate: bool) -> Screen {
    let mut last_timestamp = Instant::now();
    let mut remaining_sprint = Some(Duration::from_millis(config.sprint_time as u64));
    let mut remaining_back_panic = None;
    let mut route_target = None;
    let mut cv = Vision::new();

    let mut ui = VisualState::init();

    if simulate {
        ui.values_h[0].text_green("SYM");
        ui.values_h[1].empty();
        ui.values_h[2].empty();
        ui.values_h[3].empty();
        ui.values_h[4].empty();
        ui.update_vision(&cv, None);
    } else {
        ui.values_h[0].empty();
        ui.values_h[1].empty();
        ui.values_h[2].text_green("RACE");
        ui.values_h[3].empty();
        ui.values_h[4].empty();

        ui.values_v[0].green();
        ui.values_v[1].green();
        ui.values_v[2].green();
        ui.values_v[3].green();
        ui.values_v[4].green();
    }
    VISUAL_STATE.signal(ui);

    let (raw_laser_readings, imu_data) = join(RAW_LASER_READINGS.wait(), IMU_DATA.wait()).await;
    cv.update(&raw_laser_readings, &config);
    let mut current_heading = Angle::from_imu_value(imu_data.yaw);
    let mut current_pitch = Angle::from_imu_value(imu_data.pitch);
    let mut tilt_alert = detect_tilt_alert(current_pitch, Angle::from_imu_value(imu_data.roll));
    loop {
        let now = Instant::now();
        let dt = (now - last_timestamp).max(Duration::from_micros(100));
        last_timestamp = now;

        let (relative_target, _target_index, mut power_state, window_borders) = cv.compute_target();
        let steer = relative_target.min(Angle::MAX_STEER).max(Angle::MIN_STEER);

        let is_in_back_panic = if cv.detect_back_panic(config) {
            remaining_back_panic = Some(BackSteering {
                remaining_time: Duration::from_millis(config.back_time as u64),
                steer,
            });
            true
        } else {
            false
        };

        if route_target.is_none() && remaining_sprint.is_none() && !simulate {
            if let Some(stillness) = imu_data.stillness {
                if stillness.as_millis() as i16 >= config.stillness_time {
                    ui.blue();
                    let target_delta = if steer < Angle::ZERO {
                        Angle::L45
                    } else {
                        Angle::R45
                    };
                    route_target = Some(RouteTarget {
                        remaining_time: Duration::from_millis(config.inversion_time as u64),
                        go_back: true,
                        start: current_heading,
                        target: current_heading + target_delta,
                    });
                }
            }
        }

        if simulate {
            if imu_data.stillness.is_some() {
                ui.values_h[0].text_blue("SYM");
            } else {
                ui.values_h[0].text_green("SYM");
            }
        }

        let (power, steer) = if tilt_alert {
            if !simulate {
                ui.black();
            }
            (0, Angle::ZERO)
        } else if let Some(back_steering) = remaining_back_panic {
            ui.red();
            remaining_back_panic = if back_steering.remaining_time > dt {
                Some(BackSteering {
                    remaining_time: back_steering.remaining_time - dt,
                    ..back_steering
                })
            } else {
                None
            };
            power_state = LaserStatus::Back;
            (
                -config.back_speed,
                if is_in_back_panic {
                    -back_steering.steer
                } else {
                    Angle::ZERO
                },
            )
        } else if let Some(sprint) = remaining_sprint {
            if !simulate {
                ui.white();
            }
            remaining_sprint = if sprint > dt { Some(sprint - dt) } else { None };
            (config.sprint_speed, steer)
        } else if let Some(target) = route_target {
            if !simulate {
                ui.blue();
            }

            let delta = target.target - current_heading;

            if delta.value().abs() < 5 {
                route_target = None;
                (0, Angle::ZERO)
            } else {
                let steer = delta.min(Angle::MAX_STEER).max(Angle::MIN_STEER);

                let new_target = if target.remaining_time > dt {
                    RouteTarget {
                        remaining_time: target.remaining_time - dt,
                        ..target
                    }
                } else {
                    RouteTarget {
                        remaining_time: Duration::from_millis(config.inversion_time as u64),
                        go_back: !target.go_back,
                        ..target
                    }
                };
                route_target = Some(new_target);

                if new_target.go_back {
                    (config.back_speed, -steer)
                } else {
                    (config.max_speed, steer)
                }
            }
        } else {
            if !simulate {
                ui.green();
            }
            (
                config.turn_speed(steer) + config.climb_power_boost(current_pitch),
                steer,
            )
        };

        let action = RaceAction { power, steer };

        // log::info!(
        //     "RACE: power {} steer {}",
        //     action.power,
        //     action.steer.value()
        // );

        if simulate {
            ui.values_h[1].value(current_heading.into());
            ui.values_h[2].value(action.power);
            ui.values_h[3].steer(action.steer.into());
            ui.values_h[4].target(relative_target.into(), power_state);
            ui.update_vision(&cv, window_borders);
        }
        VISUAL_STATE.signal(ui);

        motors_go(if simulate { 0 } else { action.power }, action.steer.into());

        match select3(RAW_LASER_READINGS.wait(), IMU_DATA.wait(), CMD.wait()).await {
            Either3::First(data) => {
                cv.update(&data, &config);
            }
            Either3::Second(imu_data) => {
                current_heading = Angle::from_imu_value(imu_data.yaw);
                current_pitch = Angle::from_imu_value(imu_data.pitch);
                tilt_alert = detect_tilt_alert(current_pitch, Angle::from_imu_value(imu_data.roll));
            }
            Either3::Third(cmd) => {
                let next_screen = match cmd {
                    Cmd::Previous | Cmd::Minus => Screen::Ready,
                    Cmd::Next | Cmd::Plus => Screen::Motors,
                    Cmd::Ok | Cmd::Exit => Screen::Config,
                };
                return next_screen;
            }
        }
    }
}
