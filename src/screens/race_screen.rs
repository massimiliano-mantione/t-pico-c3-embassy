use embassy_futures::select::{select3, Either3};

use crate::{
    cmd::{Cmd, CMD},
    configuration::RaceConfig,
    imu::IMU_DATA,
    lasers::RAW_LASER_READINGS,
    lcd::{VisualState, VISUAL_STATE},
    motors::{MotorsData, MOTORS_DATA},
    vision::Vision,
};

use super::Screen;

pub async fn run(config: &RaceConfig, now: bool) -> Screen {
    return Screen::Ready;
}
