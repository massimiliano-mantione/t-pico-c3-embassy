use embassy_rp::i2c::{Async, I2c as RpI2c};
use embassy_rp::peripherals::I2C1;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{with_timeout, Duration, Instant};

use crate::tcs3472::{RgbCGain, Tcs3472};

pub type I2cBus1 = RpI2c<'static, I2C1, Async>;

const RETRY_SECS: u64 = 1;

const DETECT_COLOR_AT_LEAST_FOR: Duration = Duration::from_millis(10);
const DETECT_CROSS_AT_MOST_SINCE: Duration = Duration::from_millis(50);

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RgbEvent {
    pub dt: Duration,
    pub r: u16,
    pub g: u16,
    pub b: u16,
    pub l: u16,
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
        self.is_green()
            && self.last_green_for >= DETECT_COLOR_AT_LEAST_FOR
            && self.not_red_for <= DETECT_CROSS_AT_MOST_SINCE
            && self.last_red_for >= DETECT_COLOR_AT_LEAST_FOR
    }

    pub fn detect_good_cross(&self) -> bool {
        self.is_red()
            && self.last_red_for >= DETECT_COLOR_AT_LEAST_FOR
            && self.not_green_for <= DETECT_CROSS_AT_MOST_SINCE
            && self.last_green_for >= DETECT_COLOR_AT_LEAST_FOR
    }
}

struct RgbRanges {
    pub r_min: u16,
    pub r_max: u16,
    pub g_min: u16,
    pub g_max: u16,
    pub b_min: u16,
    pub b_max: u16,
    pub r_ratio: u16,
    pub g_ratio: u16,
    pub b_ratio: u16,
}

// fn matches_ratios(lead: u32, v1: u32, r1: u32, v2: u32, r2: u32) -> bool {
//     if lead > 0 {
//         let n1 = v1 * 100 / lead;
//         let n2 = v2 * 100 / lead;
//         n1 <= r1 * 100 && n2 <= r2 * 100
//     } else {
//         false
//     }
// }

impl RgbRanges {
    pub fn matches_levels(&self, r: u16, g: u16, b: u16) -> bool {
        r >= self.r_min
            && r <= self.r_max
            && g >= self.g_min
            && g <= self.g_max
            && b >= self.b_min
            && b <= self.b_max
    }

    pub fn matches(&self, r: u16, g: u16, b: u16) -> bool {
        self.matches_levels(r, g, b)
        // && if self.r_ratio == 100 {
        //     matches_ratios(
        //         r as u32,
        //         g as u32,
        //         self.g_ratio as u32,
        //         b as u32,
        //         self.g_ratio as u32,
        //     )
        // } else if self.g_ratio == 100 {
        //     matches_ratios(
        //         g as u32,
        //         r as u32,
        //         self.r_ratio as u32,
        //         b as u32,
        //         self.g_ratio as u32,
        //     )
        // } else if self.b_ratio == 100 {
        //     matches_ratios(
        //         r as u32,
        //         r as u32,
        //         self.g_ratio as u32,
        //         g as u32,
        //         self.g_ratio as u32,
        //     )
        // } else {
        //     false
        // }
    }
}

const RED: RgbRanges = RgbRanges {
    r_min: 75,
    r_max: u16::MAX,
    g_min: 0,
    g_max: 60,
    b_min: 0,
    b_max: 70,
    r_ratio: 100,
    g_ratio: 60,
    b_ratio: 60,
};

const GREEN: RgbRanges = RgbRanges {
    r_min: 0,
    r_max: 70,
    g_min: 75,
    g_max: u16::MAX,
    b_min: 0,
    b_max: 110,
    r_ratio: 60,
    g_ratio: 100,
    b_ratio: 60,
};

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

                if RED.matches(r, g, b) {
                    if not_red_for > DURATION_ZERO {
                        last_red_for = DURATION_ZERO;
                    }
                    last_red_for = (last_red_for + dt).min(DURATION_MAX);
                    not_red_for = DURATION_ZERO;
                } else {
                    not_red_for = (not_red_for + dt).min(DURATION_MAX);
                }

                if GREEN.matches(r, g, b) {
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
