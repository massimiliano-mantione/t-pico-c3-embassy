use embassy_rp::i2c::{AbortReason as I2cAbortReason, Async, Error as I2cError, I2c as RpI2c};
use embassy_rp::peripherals::I2C1;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{with_timeout, Duration, Instant};

use crate::tcs3472::{RgbCGain, Tcs3472};

pub type I2cBus1 = RpI2c<'static, I2C1, Async>;

const RETRY_SECS: u64 = 1;

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

    loop {
        match with_timeout(Duration::from_secs(5), tcs3472.read_all_channels_async()).await {
            Ok(Ok(rgbc)) => {
                log::info!(
                    "RGBC: r {} g {} b {} c {}",
                    rgbc.red,
                    rgbc.green,
                    rgbc.blue,
                    rgbc.clear
                );
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
