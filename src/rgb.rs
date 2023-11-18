use embassy_rp::i2c::{Async, I2c as RpI2c};
use embassy_rp::peripherals::I2C1;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{with_timeout, Duration, Instant};

use crate::tcs3472::{RgbCGain, Tcs3472};

pub type I2cBus1 = RpI2c<'static, I2C1, Async>;

const RETRY_SECS: u64 = 1;

const DETECT_COLOR_AT_LEAST_FOR: Duration = Duration::from_millis(10);
const DETECT_CROSS_AT_MOST_SINCE: Duration = Duration::from_millis(1500);

fn rgb2hsv(r: i32, g: i32, b: i32) -> (i32, i32, i32) {
    const HUE_DEGREE: i32 = 512;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let (h, s) = if delta == 0 {
        (-1, 0)
    } else {
        let h = if r == max {
            ((g - b) * 60 * HUE_DEGREE) / delta
        } else if g == max {
            ((b - r) * 60 * HUE_DEGREE) / delta + 120 * HUE_DEGREE
        } else if b == max {
            ((r - g) * 60 * HUE_DEGREE) / delta + 240 * HUE_DEGREE
        } else {
            0
        };
        let h = if h < 0 { h + 360 * HUE_DEGREE } else { h };
        let s = (256 * delta - 8) / max;

        (h / HUE_DEGREE, s)
    };

    (h, s, max)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RgbEvent {
    pub dt: Duration,
    pub r: u16,
    pub g: u16,
    pub b: u16,
    pub l: u16,
    pub h: u16,
    pub s: u16,
    pub v: u16,
    pub not_red_for: Duration,
    pub not_green_for: Duration,
    pub last_red_for: Duration,
    pub last_green_for: Duration,
}

impl RgbEvent {
    pub fn is_red(&self) -> bool {
        self.not_red_for == Duration::from_micros(0)
    }

    pub fn is_green(&self) -> bool {
        self.not_green_for == Duration::from_micros(0)
    }

    pub fn detect_inversion(&self) -> bool {
        self.is_red()
            && self.last_red_for >= DETECT_COLOR_AT_LEAST_FOR
            && self.not_green_for <= DETECT_CROSS_AT_MOST_SINCE
            && self.last_green_for >= DETECT_COLOR_AT_LEAST_FOR
    }

    pub fn detect_good_cross(&self) -> bool {
        self.is_green()
            && self.last_green_for >= DETECT_COLOR_AT_LEAST_FOR
            && self.not_red_for <= DETECT_CROSS_AT_MOST_SINCE
            && self.last_red_for >= DETECT_COLOR_AT_LEAST_FOR
    }
}

const DURATION_ZERO: Duration = Duration::from_secs(0);
const DURATION_MAX: Duration = Duration::from_secs(60);
const MIN_DT: Duration = Duration::from_micros(2000);

pub static RGB: Signal<CriticalSectionRawMutex, RgbEvent> = Signal::new();

pub async fn rgb_task(i2c: I2cBus1) {
    let mut tcs3472 = Tcs3472::new(i2c);
    let mut init_error = false;

    loop {
        if let Err(_) = tcs3472.enable() {
            init_error = true;
            log::error!("tcs3472 enable error");
        }
        if let Err(_) = tcs3472.set_rgbc_gain(RgbCGain::_16x) {
            init_error = true;
            log::error!("tcs3472 set_rgbc_gain error");
        }
        if let Err(_) = tcs3472.set_integration_cycles(1) {
            init_error = true;
            log::error!("tcs3472 set_integration_cycles error");
        }
        if let Err(_) = tcs3472.set_wait_cycles(1) {
            init_error = true;
            log::error!("tcs3472 set_wait_cycles error");
        }
        if let Err(_) = tcs3472.enable_rgbc() {
            init_error = true;
            log::error!("tcs3472 enable_rgbc error");
        }

        if init_error {
            embassy_time::Timer::after(Duration::from_secs(RETRY_SECS)).await;
            log::info!("RGB init error: retrying");
            continue;
        } else {
            break;
        }
    }

    let mut last_timestamp = Instant::now();
    let mut not_red_for = DURATION_MAX;
    let mut not_green_for = DURATION_MAX;
    let mut last_red_for = DURATION_ZERO;
    let mut last_green_for = DURATION_ZERO;

    loop {
        match with_timeout(Duration::from_secs(5), tcs3472.read_all_channels_async()).await {
            Ok(Ok(rgbc)) => {
                let now = Instant::now();
                let dt = now - last_timestamp;
                let (r, g, b, l) = (rgbc.red, rgbc.green, rgbc.blue, rgbc.clear);
                let (h, s, v) = rgb2hsv(r as i32, g as i32, b as i32);

                // 100 40 40
                // 0 90 40
                const RED_HUE: i32 = 1;
                const RED_SAT: i32 = 90;
                const GREEN_HUE: i32 = 110;
                const GREEN_SAT: i32 = 40;
                const MIN_VAL: i32 = 40;
                const HUE_DELTA: i32 = 15;
                let red_matches = (h >= 0 && h <= RED_HUE + HUE_DELTA
                    || h >= RED_HUE + 360 - HUE_DELTA)
                    && s >= RED_SAT
                    && v >= MIN_VAL;
                let green_matches = h >= GREEN_HUE - HUE_DELTA
                    && h <= GREEN_HUE + HUE_DELTA
                    && s >= GREEN_SAT
                    && v >= MIN_VAL;
                let (h, s, v) = (h as u16, s as u16, v as u16);

                if red_matches {
                    if not_red_for > DURATION_ZERO {
                        last_red_for = DURATION_ZERO;
                    }
                    last_red_for = (last_red_for + dt).min(DURATION_MAX);
                    not_red_for = DURATION_ZERO;
                } else {
                    not_red_for = (not_red_for + dt).min(DURATION_MAX);
                }

                if green_matches {
                    if not_green_for > DURATION_ZERO {
                        last_green_for = DURATION_ZERO;
                    }
                    last_green_for = (last_green_for + dt).min(DURATION_MAX);
                    not_green_for = DURATION_ZERO;
                } else {
                    not_green_for = (not_green_for + dt).min(DURATION_MAX);
                }

                // log::info!(
                //     "RGBC: r {} g {} b {} c {}, NR {}, NG {}",
                //     rgbc.red,
                //     rgbc.green,
                //     rgbc.blue,
                //     rgbc.clear,
                //     not_red_for.as_micros(),
                //     not_green_for.as_micros(),
                // );

                RGB.signal(RgbEvent {
                    dt,
                    r,
                    g,
                    b,
                    l,
                    h,
                    s,
                    v,
                    not_red_for,
                    not_green_for,
                    last_red_for,
                    last_green_for,
                });
                last_timestamp = now;
                if dt < MIN_DT {
                    embassy_time::Timer::after(MIN_DT - dt).await;
                }
            }
            Ok(Err(_)) => {
                log::info!("RGB read error");
                embassy_time::Timer::after(Duration::from_secs(RETRY_SECS)).await;
            }
            Err(_) => {
                log::info!("RGB read timeout");
            }
        }
    }
}
