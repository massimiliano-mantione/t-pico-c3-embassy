use embassy_futures::select::{select4, Either4};
use embassy_time::Instant;

use crate::{
    cmd::{Cmd, CMD},
    configuration::RaceConfig,
    imu::IMU_DATA,
    lasers::RAW_LASER_READINGS,
    lcd::{VisualState, VISUAL_STATE},
    motors::motors_stop,
    race::Angle,
    rgb::RGB,
    vision::Vision,
};

use super::Screen;

pub async fn run(config: &RaceConfig) -> Screen {
    let mut ui = VisualState::init();
    let mut v = Vision::new();
    let mut current_pitch = Angle::ZERO;

    motors_stop();

    ui.values_h[0].empty();
    ui.values_h[1].text_red("COUNTRYMAN");
    ui.values_h[2].text_green("READY");
    ui.values_h[3].empty();

    let now = Instant::now();
    let mut last_las = now;
    let mut last_imu = now;
    let mut last_rgb = now;

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
                let now = Instant::now();
                log::info!(
                    "LAS dt {}us p {}us",
                    data.dt.as_micros(),
                    now.duration_since(last_las).as_micros()
                );
                last_las = now;
                v.update(&data, &config, current_pitch);
                ui.update_vision(&v, None);
            }
            Either4::Second(data) => {
                let now = Instant::now();
                log::info!(
                    "IMU dt {}us p {}us",
                    data.dt.as_micros(),
                    now.duration_since(last_imu).as_micros()
                );
                last_imu = now;
                current_pitch = Angle::from_imu_value(data.pitch);
                ui.values_h[4].imu(
                    data.yaw,
                    data.pitch,
                    data.roll,
                    config.detect_climb(current_pitch),
                    config.detect_downhill(current_pitch),
                );
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
                let now = Instant::now();
                log::info!(
                    "RGB dt {}us p {}us",
                    data.dt.as_micros(),
                    now.duration_since(last_rgb).as_micros()
                );
                last_rgb = now;
                ui.values_h[3].rgb(data);
            }
        }

        motors_stop();
        VISUAL_STATE.signal(ui);
    }
}
