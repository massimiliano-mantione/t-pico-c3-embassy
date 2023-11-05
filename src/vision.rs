use crate::{
    configuration::RaceConfig,
    lasers::RawLaserReadings,
    race::{Angle, TrackSide},
};

pub const NUM_LASER_POSITIONS: usize = 5;
pub const CENTER_LASER: usize = 2;
pub const LASER_OVERFLOW: u16 = 1200;

pub const LILL: usize = 0;
pub const LIL: usize = 1;
pub const LIC: usize = 2;
pub const LIR: usize = 3;
pub const LIRR: usize = 4;

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

    pub fn is_ok(self) -> bool {
        match self {
            LaserStatus::Back | LaserStatus::Alert => false,
            LaserStatus::Regular | LaserStatus::Overflow => true,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            LaserStatus::Back => "BACK",
            LaserStatus::Alert => "ALRT",
            LaserStatus::Regular => "REGL",
            LaserStatus::Overflow => "OVER",
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
        let slope_delta = config.slope_distance_delta as u16;
        let slope = upper <= (lower + slope_delta) && upper >= (lower + slope_delta / 2);
        self.lower = lower;
        self.upper = upper;
        self.slope = slope;
        self.status = if slope {
            LaserStatus::Overflow
        } else {
            LaserStatus::from_value(self.value(), self.position, config)
        };
    }

    pub fn copy_status(&mut self, other: &Self) {
        self.upper = other.upper;
        self.lower = other.lower;
        self.slope = other.slope;
        self.status = other.status;
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

    pub fn compute_target(&self) -> (Angle, usize, LaserStatus, Option<(usize, usize)>) {
        self.compute_target_with_windows()
    }

    fn find_best_index(&self) -> usize {
        self.lasers
            .iter()
            .map(|l| l.value())
            .enumerate()
            .fold((0usize, 0u16), |(best_i, best_v), (ci, cv)| {
                if cv > best_v {
                    (ci, cv)
                } else {
                    (best_i, best_v)
                }
            })
            .0
    }

    fn find_best_extreme(&self, best_index: usize) -> Angle {
        if best_index < LIC {
            Angle::SLL
        } else if best_index > LIC {
            Angle::SRR
        } else {
            let (target, _, _, _) = self.compute_target_simple();
            if target < Angle::ZERO {
                Angle::SLL
            } else {
                Angle::SRR
            }
        }
    }

    fn find_open_window(&self, best_index: usize) -> (usize, usize) {
        let mut left_index = best_index;
        let mut right_index = best_index;
        while left_index > 0 {
            let next = left_index - 1;
            if self.lasers[next].status.is_ok() {
                left_index = next;
            } else {
                break;
            }
        }
        while right_index < NUM_LASER_POSITIONS - 1 {
            let next = right_index + 1;
            if self.lasers[next].status.is_ok() {
                right_index = next;
            } else {
                break;
            }
        }
        (left_index, right_index)
    }

    pub fn sensor_angle(&self, index: usize) -> Angle {
        match index {
            LILL => Angle::SLL,
            LIL => Angle::SL,
            LIC => Angle::SC,
            LIR => Angle::SR,
            LIRR => Angle::SRR,
            _ => panic!("sensor index out of bounds"),
        }
    }

    pub fn compute_target_with_windows(
        &self,
    ) -> (Angle, usize, LaserStatus, Option<(usize, usize)>) {
        let best_index = self.find_best_index();
        let best_status = self.lasers[best_index].status;

        if best_status.is_ok() {
            let (window_left, window_right) = self.find_open_window(best_index);
            let window = &self.lasers[window_left..=window_right];
            let target = match window.len() {
                1 => self.sensor_angle(best_index),
                2 => {
                    let weights = [window[0].value() as i32, window[1].value() as i32];
                    interpolate(
                        &weights,
                        self.sensor_angle(window_left),
                        self.sensor_angle(window_right),
                    )
                }
                3 => {
                    let weights = [
                        window[0].value() as i32,
                        window[1].value() as i32,
                        window[2].value() as i32,
                    ];
                    interpolate(
                        &weights,
                        self.sensor_angle(window_left),
                        self.sensor_angle(window_right),
                    )
                }
                4 => {
                    let weights = [
                        window[0].value() as i32,
                        window[1].value() as i32,
                        window[2].value() as i32,
                        window[3].value() as i32,
                    ];
                    interpolate(
                        &weights,
                        self.sensor_angle(window_left),
                        self.sensor_angle(window_right),
                    )
                }
                5 => {
                    let weights = [
                        window[0].value() as i32,
                        window[1].value() as i32,
                        window[2].value() as i32,
                        window[3].value() as i32,
                        window[4].value() as i32,
                    ];
                    interpolate(
                        &weights,
                        self.sensor_angle(window_left),
                        self.sensor_angle(window_right),
                    )
                }
                _ => panic!("window length out of bounds"),
            };
            (
                target,
                best_index,
                best_status,
                Some((window_left, window_right)),
            )
        } else {
            (
                self.find_best_extreme(best_index),
                best_index,
                best_status,
                None,
            )
        }
    }

    pub fn compute_target_simple(&self) -> (Angle, usize, LaserStatus, Option<(usize, usize)>) {
        let target = interpolate(
            &[
                self.lasers[0].value() as i32,
                self.lasers[1].value() as i32,
                self.lasers[2].value() as i32,
                self.lasers[3].value() as i32,
                self.lasers[4].value() as i32,
            ],
            Angle::SLL,
            Angle::SRR,
        );
        let index = if target < Angle::SL - Angle::SHALF {
            LILL
        } else if target < Angle::SHALF {
            LIL
        } else if target > Angle::SR + Angle::SHALF {
            LIRR
        } else if target > Angle::SHALF {
            LIL
        } else {
            LIC
        };
        (target, index, self.lasers[index].status, None)
    }

    pub fn copy_status(&mut self, other: &Self) {
        self.lasers
            .iter_mut()
            .zip(other.lasers.iter())
            .for_each(|(laser, other_laser)| laser.copy_status(other_laser));
    }

    pub fn detect_back_panic(&self, config: &RaceConfig) -> bool {
        self.lasers[LILL].value() < config.back_distance_side_60 as u16
            || self.lasers[LIRR].value() < config.back_distance_side_60 as u16
            || self.lasers[LIL].value() < config.back_distance_side_30 as u16
            || self.lasers[LIR].value() < config.back_distance_side_30 as u16
            || self.lasers[LIC].value() < config.back_distance_center as u16
    }

    pub fn compute_alert_power(&self, config: &RaceConfig, target_index: Option<usize>) -> i16 {
        if let Some(target_index) = target_index {
            let distance = self.lasers[target_index].value() as i32;
            let alert = match target_index {
                LILL | LIRR => config.alert_distance_side_60 as i32,
                LIL | LIR => config.alert_distance_side_30 as i32,
                LIC => config.alert_distance_center as i32,
                _ => DMAX,
            };

            if distance >= alert {
                config.max_speed
            } else {
                let max_speed = config.max_speed as i32;
                let min_speed = config.min_speed as i32;
                let speed_span = max_speed - min_speed;

                let alert_span = alert / 2;
                let alert_delta = (distance - (alert / 2)).max(0);

                let speed_delta = speed_span * alert_delta / alert_span;
                (min_speed + speed_delta) as i16
            }
        } else {
            config.max_speed
        }
    }
}

pub struct VisionStatus {
    pub lasers: [LaserStatus; NUM_LASER_POSITIONS],
    pub tracking_side: Option<TrackSide>,
}

const IGNORE_LONE_READING_MM: u16 = 30;

impl VisionStatus {
    pub fn new() -> Self {
        Self {
            lasers: [LaserStatus::Overflow; NUM_LASER_POSITIONS],
            tracking_side: None,
        }
    }

    pub fn update(&mut self, current: &Vision, previous: &Vision) {
        self.lasers
            .iter_mut()
            .zip(current.lasers.iter())
            .for_each(|(laser, current_laser)| {
                *laser = current_laser.status;
            });
        for i in [CENTER_LASER - 1, CENTER_LASER, CENTER_LASER + 1] {
            if self.lasers[i] == LaserStatus::Regular
                && current.lasers[i].value() >= LASER_OVERFLOW - IGNORE_LONE_READING_MM
            {
                if self.lasers[i - 1] == LaserStatus::Overflow
                    && self.lasers[i + 1] == LaserStatus::Overflow
                {
                    self.lasers[i] = LaserStatus::Overflow;
                } else if previous.lasers[i].status == LaserStatus::Overflow {
                    self.lasers[i] = LaserStatus::Overflow;
                }
            }
        }
    }

    pub fn compute_kind(
        &mut self,
        vision: &Vision,
        tracking_side: Option<TrackSide>,
    ) -> VisionKind {
        const CLR: bool = true;
        const BLK: bool = false;

        let (sll, sl, sc, sr, srr) = (
            self.lasers[0] == LaserStatus::Overflow,
            self.lasers[1] == LaserStatus::Overflow,
            self.lasers[2] == LaserStatus::Overflow,
            self.lasers[3] == LaserStatus::Overflow,
            self.lasers[4] == LaserStatus::Overflow,
        );
        let (dll, dl, dc, dr, drr) = (
            vision.lasers[0].value() as i32,
            vision.lasers[1].value() as i32,
            vision.lasers[2].value() as i32,
            vision.lasers[3].value() as i32,
            vision.lasers[4].value() as i32,
        );

        let kind = match (sll, sl, sc, sr, srr) {
            (CLR, CLR, CLR, CLR, CLR) => VisionKind::Clear,
            (BLK, CLR, CLR, CLR, CLR) => VisionKind::LeftTrack1 { dll },
            (BLK, BLK, CLR, CLR, CLR) => VisionKind::LeftTrack2 { dll, dl },
            (BLK, BLK, BLK, CLR, CLR) => VisionKind::LeftTrack3 { dll, dl, dc },
            (CLR, CLR, CLR, CLR, BLK) => VisionKind::RightTrack1 { drr },
            (CLR, CLR, CLR, BLK, BLK) => VisionKind::RightTrack2 { drr, dr },
            (CLR, CLR, BLK, BLK, BLK) => VisionKind::RightTrack3 { drr, dr, dc },
            (BLK, BLK, BLK, BLK, CLR) => VisionKind::RightEscape,
            (CLR, BLK, BLK, BLK, BLK) => VisionKind::LeftEscape,
            (CLR, BLK, BLK, BLK, CLR) => VisionKind::CenterBlocked { dl, dc, dr },
            _ => VisionKind::Blocked {
                dll,
                dl,
                dc,
                dr,
                drr,
            },
        };

        let detected_side = kind.detect_tracking_side();
        if let Some(detected_side) = detected_side {
            match tracking_side {
                Some(side) => {
                    if detected_side == side {
                        self.tracking_side = Some(detected_side)
                    }
                }
                None => self.tracking_side = Some(detected_side),
            }
        }

        kind
    }

    pub fn clear_tracking(&mut self) {
        self.tracking_side = None;
    }
}

pub enum VisionKind {
    Clear,
    LeftTrack1 {
        dll: i32,
    },
    LeftTrack2 {
        dll: i32,
        dl: i32,
    },
    LeftTrack3 {
        dll: i32,
        dl: i32,
        dc: i32,
    },
    LeftEscape,
    RightEscape,
    RightTrack3 {
        drr: i32,
        dr: i32,
        dc: i32,
    },
    RightTrack2 {
        drr: i32,
        dr: i32,
    },
    RightTrack1 {
        drr: i32,
    },
    CenterBlocked {
        dl: i32,
        dc: i32,
        dr: i32,
    },
    Blocked {
        dll: i32,
        dl: i32,
        dc: i32,
        dr: i32,
        drr: i32,
    },
}

impl VisionKind {
    pub fn detect_tracking_side(&self) -> Option<TrackSide> {
        match self {
            VisionKind::Clear => None,
            VisionKind::LeftTrack1 { .. } => Some(TrackSide::Left),
            VisionKind::LeftTrack2 { .. } => Some(TrackSide::Left),
            VisionKind::LeftTrack3 { .. } => Some(TrackSide::Left),
            VisionKind::LeftEscape => Some(TrackSide::Right),
            VisionKind::RightEscape => Some(TrackSide::Left),
            VisionKind::RightTrack3 { .. } => Some(TrackSide::Right),
            VisionKind::RightTrack2 { .. } => Some(TrackSide::Right),
            VisionKind::RightTrack1 { .. } => Some(TrackSide::Right),
            VisionKind::CenterBlocked { .. } => None,
            VisionKind::Blocked {
                dll, dl, dr, drr, ..
            } => {
                if *dll < DMAX || *dl < DMAX {
                    Some(TrackSide::Left)
                } else if *drr < DMAX || *dr < DMAX {
                    Some(TrackSide::Right)
                } else {
                    None
                }
            }
        }
    }
}

fn left_side_pid(dll: &i32, config: &RaceConfig) -> Angle {
    let distance = *dll;
    let desired = config.track_side_distance as i32;
    let delta = distance - desired;
    let kpn = config.steer_kp_n as i32;
    let kpd = config.steer_kp_n as i32;
    let pid = delta * kpn / kpd;
    pid.into()
}

fn biased_left_side_pid(dll: &i32, bias: Option<TrackSide>, config: &RaceConfig) -> Angle {
    match bias {
        Some(TrackSide::Right) => Angle::SMALL,
        Some(TrackSide::Left) | None => left_side_pid(dll, config),
    }
}

fn right_side_pid(drr: &i32, config: &RaceConfig) -> Angle {
    let distance = *drr;
    let desired = config.track_side_distance as i32;
    let delta = desired - distance;
    let kpn = config.steer_kp_n as i32;
    let kpd = config.steer_kp_n as i32;
    let pid = delta * kpn / kpd;
    pid.into()
}

fn biased_right_side_pid(drr: &i32, bias: Option<TrackSide>, config: &RaceConfig) -> Angle {
    match bias {
        Some(TrackSide::Left) => -Angle::SMALL,
        Some(TrackSide::Right) | None => right_side_pid(drr, config),
    }
}

const DMAX: i32 = LASER_OVERFLOW as i32;

impl VisionKind {
    pub fn compute_relative_target(
        &self,
        main_direction: Angle,
        heading: Angle,
        bias: Option<TrackSide>,
        tracking: Option<TrackSide>,
        config: &RaceConfig,
    ) -> (Angle, Option<usize>) {
        match self {
            VisionKind::Clear => match tracking {
                Some(TrackSide::Left) => {
                    let target_limit = main_direction + Angle::L90;
                    let side_result = heading + Angle::SLL;
                    let absolute_target = target_limit.max(side_result);
                    (absolute_target - heading, None)
                }
                Some(TrackSide::Right) => {
                    let target_limit = main_direction + Angle::R90;
                    let side_result = heading + Angle::SRR;
                    let absolute_target = target_limit.min(side_result);
                    (absolute_target - heading, None)
                }
                None => (main_direction - heading, None),
            },
            VisionKind::LeftTrack1 { dll } => (biased_left_side_pid(dll, bias, config), None),
            VisionKind::LeftTrack2 { dll, dl, .. } => (
                biased_left_side_pid(dll, bias, config).max(interpolate(
                    &[*dl, DMAX],
                    Angle::SL,
                    Angle::SC,
                )),
                Some(LIL),
            ),
            VisionKind::LeftTrack3 { dc, .. } => {
                (interpolate(&[*dc, DMAX], Angle::SC, Angle::SRR), Some(LIC))
            }
            VisionKind::LeftEscape => (Angle::SRR, Some(LIR)),
            VisionKind::RightEscape => (Angle::SRR, Some(LIL)),
            VisionKind::RightTrack3 { dc, .. } => {
                (interpolate(&[DMAX, *dc], Angle::SLL, Angle::SC), Some(LIC))
            }
            VisionKind::RightTrack2 { drr, dr } => (
                biased_right_side_pid(drr, bias, config).min(interpolate(
                    &[DMAX, *dr],
                    Angle::SC,
                    Angle::SL,
                )),
                Some(LIL),
            ),
            VisionKind::RightTrack1 { drr } => (biased_right_side_pid(drr, bias, config), None),
            VisionKind::CenterBlocked { dl, dc, dr } => {
                let target = interpolate(&[*dl, *dc, *dr], Angle::SL, Angle::SR);
                if target.value() < 0 {
                    (Angle::SLL, Some(LIL))
                } else {
                    (Angle::SRR, Some(LIR))
                }
            }
            VisionKind::Blocked {
                dll,
                dl,
                dc,
                dr,
                drr,
            } => {
                let target = interpolate(&[*dll, *dl, *dc, *dr, *drr], Angle::SLL, Angle::SRR);
                let index = if target < Angle::SL - Angle::SHALF {
                    LILL
                } else if target < Angle::SHALF {
                    LIL
                } else if target > Angle::SR + Angle::SHALF {
                    LIRR
                } else if target > Angle::SHALF {
                    LIL
                } else {
                    LIC
                };
                (target, Some(index))
            }
        }
    }
}

// Returns the non-normalized weighted average (weighted average * sum), and the sum of the values
pub fn weighted_average<const N: usize>(values: &[i32; N]) -> (i32, i32) {
    if N < 1 {
        return (0, 1);
    }

    if N < 2 {
        return (0, values[0]);
    }

    let sum: i32 = values.iter().copied().sum();
    if sum == 0 {
        return (0, 1);
    };

    let side_double = N as i32;
    let center_double = side_double - 1;

    let average = values
        .iter()
        .copied()
        .enumerate()
        .map(|(index, value)| (index as i32 * 2, value))
        .fold(0, |result, (index_double, value)| {
            let weight = index_double - center_double;
            result + (value * weight)
        });
    (average / center_double, sum)
}

pub fn interpolate<const N: usize>(weights: &[i32; N], from: Angle, to: Angle) -> Angle {
    let (from, to): (i32, i32) = (from.into(), to.into());
    let (weight, sum) = weighted_average(weights);
    let middle2 = from + to;
    let side2 = to - from;

    let normalized_side2 = (weight * side2) / sum;
    let normalized2 = middle2 + normalized_side2;
    let normalized = normalized2 / 2;

    normalized.into()
}

pub fn is_in_window(index: usize, borders: Option<(usize, usize)>) -> bool {
    match borders {
        Some((left, right)) => index >= left && index <= right,
        None => false,
    }
}
