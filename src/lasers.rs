use embassy_rp::i2c::{AbortReason as I2cAbortReason, Async, Error as I2cError, I2c as RpI2c};
use embassy_rp::peripherals::I2C0;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant};
use embedded_hal_async::i2c::I2c;

pub type I2cBus = RpI2c<'static, I2C0, Async>;

pub const RAW_LASERS_COUNT: usize = 8;

#[derive(Clone, Copy)]
pub struct RawLaserReadings {
    pub values: [u16; RAW_LASERS_COUNT],
    pub timestamp: Instant,
    pub dt: Duration,
}

pub static RAW_LASER_READINGS: Signal<CriticalSectionRawMutex, RawLaserReadings> = Signal::new();

const TCA9548A_ADDR: u16 = 0x70;
const GP2Y0E02B_ADDR: u16 = 0x40;
const GP2Y0E02B_REG: u8 = 0x5E;

fn i2c_error_message(err: &I2cError) -> &'static str {
    match err {
        I2cError::Abort(reason) => match reason {
            I2cAbortReason::NoAcknowledge => "abort: no acknowledge",
            I2cAbortReason::ArbitrationLoss => "abort: arbitration loss",
            I2cAbortReason::Other(_) => "abort: other",
        },
        I2cError::InvalidReadBufferLength => "invalid read buffer length",
        I2cError::InvalidWriteBufferLength => "invalid write buffer length",
        I2cError::AddressOutOfRange(_) => "address out of range",
        I2cError::AddressReserved(_) => "address reserved",
    }
}

const I2C_TIMEOUT: Duration = Duration::from_secs(1);

async fn select_i2c_channel(i2c: &mut I2cBus, chan: usize) {
    match embassy_time::with_timeout(I2C_TIMEOUT, i2c.write_async(TCA9548A_ADDR, [1 << chan])).await
    {
        Ok(result) => match result {
            Ok(_) => {}
            Err(err) => {
                log::error!("I2C select write error: {}", i2c_error_message(&err));
            }
        },
        Err(_) => {
            log::error!("I2C select timeout");
        }
    }
}

async fn read_distance(i2c: &mut I2cBus) -> u16 {
    let mut result_buf = [0u8; 2];
    match embassy_time::with_timeout(
        I2C_TIMEOUT,
        i2c.write_read(GP2Y0E02B_ADDR, &[GP2Y0E02B_REG], &mut result_buf),
    )
    .await
    {
        Ok(result) => match result {
            Ok(_) => {
                let raw_distance = ((result_buf[0] as u16) << 4) | ((result_buf[1] & 0xf) as u16);
                let distance = (raw_distance * 10) / (1 << 6);
                distance
            }
            Err(err) => {
                log::error!("I2C read distance error: {}", i2c_error_message(&err));
                0
            }
        },
        Err(_) => {
            log::error!("I2C sensor read timeout");
            0
        }
    }
}

pub async fn lasers_task(mut i2c: I2cBus) {
    let mut raw_readings = RawLaserReadings {
        values: [0u16; RAW_LASERS_COUNT],
        timestamp: Instant::now(),
        dt: Duration::from_micros(100),
    };
    loop {
        for (chan, d) in raw_readings.values.iter_mut().enumerate() {
            select_i2c_channel(&mut i2c, chan).await;
            *d = read_distance(&mut i2c).await;
        }
        let now = Instant::now();
        raw_readings.dt = now - raw_readings.timestamp;
        raw_readings.timestamp = now;
        RAW_LASER_READINGS.signal(raw_readings);
    }
}
