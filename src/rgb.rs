use embassy_rp::i2c::{Async, I2c as RpI2c};
use embassy_rp::peripherals::I2C1;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{with_timeout, Duration, Instant};

use crate::tcs3472::{RgbCGain, Tcs3472};

pub type I2cBus1 = RpI2c<'static, I2C1, Async>;

const RETRY_SECS: u64 = 1;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RgbEvent {
    pub dt: Duration,
    pub r: u16,
    pub g: u16,
    pub b: u16,
    pub l: u16,
}

pub static RGB: Signal<CriticalSectionRawMutex, RgbEvent> = Signal::new();

pub async fn rgb_task(i2c: I2cBus1) {
    let mut tcs3472 = Tcs3472::new(i2c);
    let mut init_error = false;

    loop {
        if let Err(_) = tcs3472.enable() {
            init_error = true;
            log::error!("tcs3472 enable error");
        }
        if let Err(_) = tcs3472.set_rgbc_gain(RgbCGain::_60x) {
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
    loop {
        match with_timeout(Duration::from_secs(5), tcs3472.read_all_channels_async()).await {
            Ok(Ok(rgbc)) => {
                // log::info!(
                //     "RGBC: r {} g {} b {} c {}",
                //     rgbc.red,
                //     rgbc.green,
                //     rgbc.blue,
                //     rgbc.clear
                // );
                let now = Instant::now();
                RGB.signal(RgbEvent {
                    dt: now - last_timestamp,
                    r: rgbc.red,
                    g: rgbc.green,
                    b: rgbc.blue,
                    l: rgbc.clear,
                });
                last_timestamp = now;
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
