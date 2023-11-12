use embassy_futures::select::{select3, Either3};

use crate::{
    cmd::{Cmd, CMD},
    configuration::RaceConfig,
    imu::IMU_DATA,
    lasers::RAW_LASER_READINGS,
    lcd::{VisualState, VISUAL_STATE},
    motors::motors_go,
    vision::Vision,
};

use super::Screen;

pub async fn run(config: &RaceConfig) -> Screen {
    let mut ui = VisualState::init();
    let mut v = Vision::new();

    ui.values_h[0].empty();
    ui.values_h[1].text_red("COUNTRYMAN");
    ui.values_h[2].text_green("MOTORS");
    ui.values_h[3].empty();
    ui.values_h[4].empty();

    let mut steer = 0;
    let mut power = 0;

    loop {
        match select3(RAW_LASER_READINGS.wait(), IMU_DATA.wait(), CMD.wait()).await {
            Either3::First(data) => {
                v.update(&data, &config);
                ui.update_vision(&v, None);
            }
            Either3::Second(data) => {
                steer = -(data.yaw / 100).min(35).max(-35);
                let pitch = (data.pitch as i32 / 100).min(90).max(-90);
                power = if pitch > 10 {
                    ((pitch - 10) * 10000 / 80).min(10000)
                } else if data.pitch < -10 {
                    ((pitch + 10) * 10000 / 80).max(-10000)
                } else {
                    0
                } as i16;
                ui.values_h[3].value(data.yaw / 100);
                ui.values_h[4].value(power);
            }
            Either3::Third(c) => {
                log::info!("cmd: {}", c.name());
                match c {
                    Cmd::Previous => return Screen::Simulation,
                    Cmd::Next => return Screen::Imu,
                    _ => {}
                }
            }
        }

        motors_go(power, steer);
        VISUAL_STATE.signal(ui);
    }
}
