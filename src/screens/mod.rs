use crate::{
    configuration::RaceConfig,
    imu::IMU_DATA,
    race::{race, Angle},
};

mod config_screen;
mod motors_screen;
mod race_screen;
mod ready_screen;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Ready,
    Race,
    RaceNow,
    Motors,
    Config,
    Simulation,
}

async fn simulation_screen(config: &RaceConfig) -> Screen {
    let imu_data = IMU_DATA.wait().await;
    let start_angle = Angle::from_imu_value(imu_data.yaw);
    race(config, start_angle, true).await
}

pub async fn run() -> ! {
    let mut config = RaceConfig::init();
    let mut screen = Screen::Ready;

    loop {
        screen = match screen {
            Screen::Ready => ready_screen::run(&config).await,
            Screen::Race => {
                race_screen::run(&config, false).await;
                Screen::Config
            }
            Screen::RaceNow => race_screen::run(&config, true).await,
            Screen::Motors => motors_screen::run(&config).await,
            Screen::Config => config_screen::run(&mut config).await,
            Screen::Simulation => simulation_screen(&config).await,
        }
    }
}
