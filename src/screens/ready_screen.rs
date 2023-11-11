use embassy_futures::select::{select4, Either4};

use crate::{
    cmd::{Cmd, CMD},
    configuration::RaceConfig,
    imu::IMU_DATA,
    lasers::RAW_LASER_READINGS,
    lcd::{VisualState, VISUAL_STATE},
    motors::motors_stop,
    rgb::RGB,
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
        match select4(
            RAW_LASER_READINGS.wait(),
            IMU_DATA.wait(),
            CMD.wait(),
            RGB.wait(),
        )
        .await
        {
            Either4::First(data) => {
                v.update(&data, &config);
                ui.update_vision(&v, None);
                log::info!("L dt {}us", data.dt.as_micros());
            }
            Either4::Second(data) => {
                log::info!("IMU dt {}us", data.dt.as_micros());
                ui.values_h[4].imu(data.yaw, data.pitch, data.roll);
            }
            Either4::Third(c) => {
                log::info!("cmd: {}", c.name());
                match c {
                    Cmd::Previous => return Screen::RaceNow,
                    Cmd::Next => return Screen::Race,
                    Cmd::Plus => return Screen::Simulation,
                    Cmd::Minus => return Screen::Config,
                    _ => {}
                }
            }
            Either4::Fourth(data) => {
                log::info!("RGB dt {}us", data.dt.as_micros());
                ui.values_h[3].rgb(data);
            }
        }

        motors_stop();
        VISUAL_STATE.signal(ui);
    }
}
