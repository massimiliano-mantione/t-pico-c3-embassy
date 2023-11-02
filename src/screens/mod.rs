use crate::configuration::RaceConfig;

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
}

pub async fn run() -> ! {
    let mut config = RaceConfig::init();
    let mut screen = Screen::Ready;

    loop {
        screen = match screen {
            Screen::Ready => ready_screen::run(&config).await,
            Screen::Race => race_screen::run(&config, false).await,
            Screen::RaceNow => race_screen::run(&config, true).await,
            Screen::Motors => motors_screen::run(&config).await,
            Screen::Config => config_screen::run(&mut config).await,
        }
    }
}
