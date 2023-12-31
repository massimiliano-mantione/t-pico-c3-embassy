use embassy_futures::select::{select3, Either3};

use crate::{
    cmd::{Cmd, CMD},
    configuration::{RaceConfig, RaceConfigEntry},
    imu::IMU_DATA,
    lasers::RAW_LASER_READINGS,
    lcd::{VisualState, VISUAL_STATE},
    motors::motors_stop,
};

use super::Screen;

pub async fn run(config: &mut RaceConfig) -> Screen {
    let mut ui = VisualState::init();

    ui.values_h[0].empty();
    ui.values_h[1].text_red("COUNTRYMAN");
    ui.values_h[2].text_green("CONFIG");
    ui.values_h[3].empty();
    ui.values_h[4].empty();
    ui.values_v[0].empty();
    ui.values_v[1].empty();
    ui.values_v[2].empty();
    ui.values_v[3].empty();
    ui.values_v[4].empty();

    let mut entry = RaceConfigEntry::start();
    let mut editing = false;

    loop {
        match select3(RAW_LASER_READINGS.wait(), IMU_DATA.wait(), CMD.wait()).await {
            Either3::First(_data) => {}
            Either3::Second(_data) => {}
            Either3::Third(c) => {
                log::info!("cmd: {}", c.name());
                match c {
                    Cmd::Previous => {
                        if editing {
                            config.dec(entry);
                        } else {
                            entry = entry.prev();
                        }
                    }
                    Cmd::Next => {
                        if editing {
                            config.inc(entry);
                        } else {
                            entry = entry.next();
                        }
                    }
                    Cmd::Plus => {
                        if editing {
                            for _ in 0..5 {
                                config.inc(entry);
                            }
                        } else {
                            editing = true;
                        }
                    }
                    Cmd::Minus => {
                        if editing {
                            for _ in 0..5 {
                                config.dec(entry);
                            }
                        } else {
                            editing = true;
                        }
                    }
                    Cmd::Exit | Cmd::Ok => {
                        if editing {
                            editing = false;
                        } else {
                            return Screen::Ready;
                        }
                    }
                }
            }
        }

        if editing {
            ui.values_h[3].text_green(entry.name());
        } else {
            ui.values_h[3].text(entry.name());
        }

        let value = config.get(entry);
        match entry.value_name(value) {
            Some(name) => ui.values_h[4].text(name),
            None => ui.values_h[4].value(value),
        }

        motors_stop();
        VISUAL_STATE.signal(ui);
    }
}
