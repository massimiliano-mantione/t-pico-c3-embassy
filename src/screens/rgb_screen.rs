use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Instant};

use crate::{
    cmd::{Cmd, CMD},
    lcd::{VisualState, VISUAL_STATE},
    motors::motors_stop,
    rgb::RGB,
};

use super::Screen;

pub async fn run() -> Screen {
    let mut ui = VisualState::init();

    let mut r_min = i16::MAX;
    let mut g_min = i16::MAX;
    let mut b_min = i16::MAX;
    let mut r_max = 0i16;
    let mut g_max = 0i16;
    let mut b_max = 0i16;

    let mut now = Instant::now();
    let mut last_inversion = now;
    let mut last_good_cross = now;

    ui.values_h[0].text("RGB");
    ui.values_h[1].value2_red(r_max, r_max);
    ui.values_h[2].value2_red(r_max, r_max);
    ui.values_h[3].value2_red(r_max, r_max);
    ui.values_h[4].text("");
    ui.values_v[0].black();
    ui.values_v[1].black();
    ui.values_v[2].black();
    ui.values_v[3].black();
    ui.values_v[4].black();

    loop {
        match select(RGB.wait(), CMD.wait()).await {
            Either::First(data) => {
                now = Instant::now();

                if data.detect_inversion() {
                    last_inversion = now;
                }
                if data.detect_good_cross() {
                    last_good_cross = now;
                }

                r_min = r_min.min(data.r as i16);
                g_min = g_min.min(data.g as i16);
                b_min = b_min.min(data.b as i16);
                r_max = r_max.max(data.r as i16);
                g_max = g_max.max(data.g as i16);
                b_max = b_max.max(data.b as i16);

                ui.values_h[1].value2_red(r_min, r_max);
                ui.values_h[2].value2_green(g_min, g_max);
                ui.values_h[3].value2_blue(b_min, b_max);
                ui.values_h[4].rgb(data);

                if data.is_red() {
                    ui.values_v[0].red();
                    ui.values_v[1].red();
                } else {
                    ui.values_v[0].black();
                    ui.values_v[1].black();
                }

                if data.is_green() {
                    ui.values_v[3].green();
                    ui.values_v[4].green();
                } else {
                    ui.values_v[3].black();
                    ui.values_v[4].black();
                }

                if now - last_inversion < Duration::from_millis(500) {
                    ui.values_v[2].red();
                } else if now - last_good_cross < Duration::from_millis(500) {
                    ui.values_v[2].green();
                } else {
                    ui.values_v[2].black();
                }
            }
            Either::Second(c) => {
                log::info!("cmd: {}", c.name());
                match c {
                    Cmd::Previous => return Screen::Imu,
                    Cmd::Next => return Screen::Ready,
                    Cmd::Plus | Cmd::Minus => {
                        r_min = i16::MAX;
                        g_min = i16::MAX;
                        b_min = i16::MAX;
                        r_max = 0i16;
                        g_max = 0i16;
                        b_max = 0i16;
                    }
                    _ => {}
                }
            }
        }

        motors_stop();
        VISUAL_STATE.signal(ui);
    }
}
