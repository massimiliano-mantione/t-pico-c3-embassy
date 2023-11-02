use embassy_futures::select::{select, Either};
use embassy_time::Duration;

use crate::{
    cmd::{Cmd, CMD},
    configuration::RaceConfig,
    imu::IMU_DATA,
    lasers::RAW_LASER_READINGS,
    lcd::{VisualState, VISUAL_STATE},
    motors::{motors_go, motors_stop},
    vision::Vision,
};

use super::Screen;

pub async fn run(config: &RaceConfig, now: bool) -> Screen {
    let _yaw = match if now { wait_1().await } else { wait_5().await } {
        Some(yaw) => yaw,
        None => return Screen::Ready,
    };
    let go = sprint(config).await;

    if !go {
        return Screen::Ready;
    }

    return Screen::Ready;
}

async fn wait_5() -> Option<i16> {
    let mut ui = VisualState::init();

    motors_stop();

    ui.values_h[0].empty();
    ui.values_h[1].empty();
    ui.values_h[2].text_red("WAIT");
    ui.values_h[3].empty();
    ui.values_h[4].empty();

    ui.values_v[0].yellow();
    ui.values_v[1].red();
    ui.values_v[2].red();
    ui.values_v[3].red();
    ui.values_v[4].red();
    VISUAL_STATE.signal(ui);

    for c in 0usize..4 {
        match select(
            embassy_time::Timer::after(Duration::from_secs(1)),
            CMD.wait(),
        )
        .await
        {
            Either::First(_) => {
                ui.values_v[c + 1].yellow();
                VISUAL_STATE.signal(ui);
            }
            Either::Second(_) => return None,
        }
    }
    match select(
        embassy_time::Timer::after(Duration::from_secs(1)),
        CMD.wait(),
    )
    .await
    {
        Either::First(_) => Some(IMU_DATA.wait().await.yaw),
        Either::Second(_) => None,
    }
}

async fn wait_1() -> Option<i16> {
    let mut ui = VisualState::init();

    motors_stop();

    ui.values_h[0].empty();
    ui.values_h[1].empty();
    ui.values_h[2].text_red("WAIT");
    ui.values_h[3].empty();
    ui.values_h[4].empty();

    ui.values_v[0].yellow();
    ui.values_v[1].red();
    ui.values_v[2].yellow();
    ui.values_v[3].red();
    ui.values_v[4].yellow();
    VISUAL_STATE.signal(ui);

    match select(
        embassy_time::Timer::after(Duration::from_secs(1)),
        CMD.wait(),
    )
    .await
    {
        Either::First(_) => Some(IMU_DATA.wait().await.yaw),
        Either::Second(_) => None,
    }
}

async fn sprint(config: &RaceConfig) -> bool {
    let mut ui = VisualState::init();

    motors_go(config.sprint_speed, 0);

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
    VISUAL_STATE.signal(ui);

    match select(
        embassy_time::Timer::after(Duration::from_millis(config.sprint_time as u64)),
        CMD.wait(),
    )
    .await
    {
        Either::First(_) => true,
        Either::Second(_) => false,
    }
}
