use crate::vision::LaserSidePosition;

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
