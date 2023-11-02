#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use buttons::{LeftButton, RightButton};
use embassy_executor::Executor;
use embassy_futures::select::{select3, Either3};
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Input, Pull};
use embassy_rp::i2c::{Config as I2cConfig, I2c as RpI2c, InterruptHandler as InterruptHandlerI2c};
use embassy_rp::multicore::{spawn_core1, Stack};
use embassy_rp::peripherals::{
    I2C0, PIN_0, PIN_1, PIN_16, PIN_17, PIN_2, PIN_27, PIN_28, PIN_29, PIN_3, PIN_4, PIN_5, PIN_8,
    PIN_9, PWM_CH5, PWM_CH6, SPI0, UART0, UART1, USB,
};
use embassy_rp::uart::BufferedInterruptHandler;
use embassy_rp::usb::{Driver, InterruptHandler as InterruptHandlerUsb};
use rp2040_panic_usb_boot as _;
use static_cell::StaticCell;

mod buttons;
mod cmd;
mod esp32c3;
mod imu;
mod lasers;
mod lcd;
mod motors;
mod uformat;
mod vision;

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandlerUsb<USB>;
    I2C0_IRQ => InterruptHandlerI2c<I2C0>;
    UART0_IRQ => BufferedInterruptHandler<UART0>;
    UART1_IRQ => BufferedInterruptHandler<UART1>;
});

#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

#[embassy_executor::task]
async fn lasers_task(i2c: lasers::I2cBus) {
    lasers::lasers_task(i2c).await
}

#[embassy_executor::task]
async fn imu_task(uart0: UART0, pin_16: PIN_16, pin_17: PIN_17) {
    imu::imu_task(uart0, pin_16, pin_17).await
}

#[embassy_executor::task]
async fn esp32c3_task(uart1: UART1, pin_8: PIN_8, pin_9: PIN_9) {
    esp32c3::esp32c3_task(uart1, pin_8, pin_9).await
}

#[embassy_executor::task]
async fn motors_task(
    pwm_ch6: PWM_CH6,
    pwm_ch5: PWM_CH5,
    pin27: PIN_27,
    pin28: PIN_28,
    pin29: PIN_29,
) {
    motors::motors_task(pwm_ch6, pwm_ch5, pin27, pin28, pin29).await
}

#[embassy_executor::task]
async fn buttons_task(left_button: LeftButton, right_button: RightButton) {
    buttons::buttons_task(left_button, right_button).await
}

static mut CORE1_STACK: Stack<16384> = Stack::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();

use vision::Vision;

#[embassy_executor::task]
async fn core0_task() -> ! {
    log::info!("Hello from core 0");

    let mut ui = lcd::VisualState::init();
    let mut v = Vision::new();
    let config = vision::RaceConfig::init();

    let mut steer = 0i16;
    let mut power = 0i16;

    loop {
        match select3(
            lasers::RAW_LASER_READINGS.wait(),
            imu::IMU_DATA.wait(),
            cmd::CMD.wait(),
        )
        .await
        {
            Either3::First(data) => {
                v.update(&data, &config);
                ui.update_vision(&v);

                log::info!("L dt {}us", data.dt.as_micros());
            }
            Either3::Second(data) => {
                log::info!("IMU dt {}us", data.dt.as_micros());
                ui.values_h[0].imu(data.yaw, data.pitch, data.roll);
            }
            Either3::Third(c) => {
                log::info!("cmd: {}", c.name());
                match c {
                    cmd::Cmd::Previous => steer -= 1,
                    cmd::Cmd::Next => steer += 1,
                    cmd::Cmd::Plus => power += 1000,
                    cmd::Cmd::Minus => power -= 1000,
                    _ => {}
                }
            }
        }

        ui.values_h[1].text("STEER");
        ui.values_h[2].steer(steer);
        ui.values_h[3].text("POWER");
        ui.values_h[4].power(power);

        motors::MOTORS_DATA.signal(motors::MotorsData { power, steer });
        lcd::VISUAL_STATE.signal(ui);
    }
}

#[embassy_executor::task]
async fn tft_task(
    spi: SPI0,
    bl: PIN_4,
    tft_miso: PIN_0,
    tft_mosi: PIN_3,
    tft_clk: PIN_2,
    tft_cs: PIN_5,
    tft_dc: PIN_1,
) {
    lcd::tft_task(spi, bl, tft_miso, tft_mosi, tft_clk, tft_cs, tft_dc).await
}

#[embassy_executor::task]
async fn core1_task(
    spi: SPI0,
    bl: PIN_4,
    tft_miso: PIN_0,
    tft_mosi: PIN_3,
    tft_clk: PIN_2,
    tft_cs: PIN_5,
    tft_dc: PIN_1,
) -> ! {
    log::info!("Hello from core 1");
    lcd::tft_task(spi, bl, tft_miso, tft_mosi, tft_clk, tft_cs, tft_dc).await
}

#[cortex_m_rt::entry]
fn main() -> ! {
    let p = embassy_rp::init(Default::default());
    // let led = Output::new(p.PIN_25, Level::Low);

    // Init button pins
    let left_button = Input::new(p.PIN_6, Pull::Up);
    let right_button = Input::new(p.PIN_7, Pull::Up);

    // Eventually reboot to bootsel
    if left_button.is_low() || right_button.is_low() {
        rp2040_hal::rom_data::reset_to_usb_boot(0, 0);
    }

    let driver = Driver::new(p.USB, Irqs);

    // Init TFT display
    let spi0 = p.SPI0;
    let bl = p.PIN_4;
    let tft_miso: PIN_0 = p.PIN_0;
    let tft_mosi: PIN_3 = p.PIN_3;
    let tft_clk: PIN_2 = p.PIN_2;
    let tft_cs: PIN_5 = p.PIN_5;
    let tft_dc: PIN_1 = p.PIN_1;

    spawn_core1(p.CORE1, unsafe { &mut CORE1_STACK }, move || {
        let executor1 = EXECUTOR1.init(Executor::new());
        executor1.run(|spawner| {
            spawner
                .spawn(core1_task(
                    spi0, bl, tft_miso, tft_mosi, tft_clk, tft_cs, tft_dc,
                ))
                .unwrap();
        });
    });

    log::info!("set up i2c ");
    let mut config = I2cConfig::default();
    config.frequency = 400_000;
    let i2c: lasers::I2cBus = RpI2c::new_async(p.I2C0, p.PIN_13, p.PIN_12, Irqs, config);

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| {
        spawner.spawn(logger_task(driver)).unwrap();
        spawner.spawn(lasers_task(i2c)).unwrap();
        spawner
            .spawn(imu_task(p.UART0, p.PIN_16, p.PIN_17))
            .unwrap();
        // spawner
        //     .spawn(esp32c3_task(p.UART1, p.PIN_8, p.PIN_9))
        //     .unwrap();
        spawner
            .spawn(motors_task(
                p.PWM_CH6, p.PWM_CH5, p.PIN_27, p.PIN_28, p.PIN_29,
            ))
            .unwrap();
        spawner
            .spawn(buttons_task(left_button, right_button))
            .unwrap();
        spawner.spawn(core0_task()).unwrap();
    });
}
