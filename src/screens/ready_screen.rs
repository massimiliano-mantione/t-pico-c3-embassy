use embassy_futures::select::{select3, Either3};

use crate::{
    cmd::{Cmd, CMD},
    configuration::RaceConfig,
    imu::IMU_DATA,
    lasers::RAW_LASER_READINGS,
    lcd::{VisualState, VISUAL_STATE},
    motors::motors_stop,
    vision::Vision,
};

use super::Screen;

pub async fn run(config: &RaceConfig) -> Screen {
    let mut ui = VisualState::init();
    let mut v = Vision::new();

    motors_stop();

    ui.values_h[0].empty();
    ui.values_h[1].text_red("COUNTRYMAN");
    ui.values_h[2].text_green("READY");
    ui.values_h[3].empty();

    loop {
        match select3(RAW_LASER_READINGS.wait(), IMU_DATA.wait(), CMD.wait()).await {
            Either3::First(data) => {
                v.update(&data, &config);
                ui.update_vision(&v);

                log::info!("L dt {}us", data.dt.as_micros());
            }
            Either3::Second(data) => {
                log::info!("IMU dt {}us", data.dt.as_micros());
                ui.values_h[4].imu(data.yaw, data.pitch, data.roll);
            }
            Either3::Third(c) => {
                log::info!("cmd: {}", c.name());
                match c {
                    Cmd::Previous => return Screen::RaceNow,
                    Cmd::Next => return Screen::Race,
                    Cmd::Plus => return Screen::Motors,
                    Cmd::Minus => return Screen::Config,
                    _ => {}
                }
            }
        }

        motors_stop();
        VISUAL_STATE.signal(ui);
    }
}
