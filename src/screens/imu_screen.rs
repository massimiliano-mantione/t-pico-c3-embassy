use embassy_futures::select::{select3, Either3};

use crate::{
    cmd::{Cmd, CMD},
    configuration::RaceConfig,
    imu::IMU_DATA,
    lasers::RAW_LASER_READINGS,
    lcd::{VisualState, VISUAL_STATE},
    motors::motors_stop,
    race::Angle,
    vision::Vision,
};

use super::Screen;

pub async fn run(config: &RaceConfig) -> Screen {
    let mut ui = VisualState::init();
    let mut v = Vision::new();
    let mut current_pitch = Angle::ZERO;

    ui.values_h[0].text_green("IMU");
    ui.values_h[1].text("");
    ui.values_h[2].text("");
    ui.values_h[3].text("");
    ui.values_h[4].text("");

    loop {
        match select3(RAW_LASER_READINGS.wait(), IMU_DATA.wait(), CMD.wait()).await {
            Either3::First(data) => {
                v.update(&data, &config, current_pitch);
                ui.update_vision(&v, None);
            }
            Either3::Second(data) => {
                current_pitch = Angle::from_imu_value(data.pitch);
                ui.values_h[1].imu_angles(data.yaw, data.pitch, data.roll);
                ui.values_h[2].value(data.forward);
                ui.values_h[3].value(data.side);
                ui.values_h[4].value(data.vertical);
            }
            Either3::Third(c) => {
                log::info!("cmd: {}", c.name());
                match c {
                    Cmd::Previous => return Screen::Motors,
                    Cmd::Next => return Screen::Ready,
                    _ => {}
                }
            }
        }

        motors_stop();
        VISUAL_STATE.signal(ui);
    }
}
