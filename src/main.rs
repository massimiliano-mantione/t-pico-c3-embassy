#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use buttons::{LeftButton, RightButton};
use embassy_executor::Executor;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Input, Pull};
use embassy_rp::i2c::{Config as I2cConfig, I2c as RpI2c, InterruptHandler as InterruptHandlerI2c};
use embassy_rp::multicore::{spawn_core1, Stack};
use embassy_rp::peripherals::{
    I2C0, I2C1, PIN_0, PIN_1, PIN_16, PIN_17, PIN_2, PIN_27, PIN_28, PIN_29, PIN_3, PIN_4, PIN_5,
    PIN_8, PIN_9, PWM_CH5, PWM_CH6, SPI0, UART0, UART1, USB,
};
use embassy_rp::uart::BufferedInterruptHandler;
use embassy_rp::usb::{Driver, InterruptHandler as InterruptHandlerUsb};
use rp2040_panic_usb_boot as _;
use static_cell::StaticCell;

pub mod buttons;
pub mod cmd;
pub mod configuration;
pub mod esp32c3;
pub mod imu;
pub mod lasers;
pub mod lcd;
pub mod motors;
pub mod race;
pub mod rgb;
pub mod screens;
pub mod tcs3472;
pub mod trace;
pub mod uformat;
pub mod vision;

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandlerUsb<USB>;
    I2C0_IRQ => InterruptHandlerI2c<I2C0>;
    I2C1_IRQ => InterruptHandlerI2c<I2C1>;
    UART0_IRQ => BufferedInterruptHandler<UART0>;
    UART1_IRQ => BufferedInterruptHandler<UART1>;
});

#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

#[embassy_executor::task]
async fn lasers_task(i2c: lasers::I2cBus0) {
    lasers::lasers_task(i2c).await
}

#[embassy_executor::task]
async fn rgb_task(i2c: rgb::I2cBus1) {
    rgb::rgb_task(i2c).await
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
async fn trace_task() {
    trace::trace_task().await
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

#[embassy_executor::task]
async fn main_task() -> ! {
    log::info!("Hello from main task (core 0)");
    loop {
        screens::run().await;
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
async fn lcd_task(
    spi: SPI0,
    bl: PIN_4,
    tft_miso: PIN_0,
    tft_mosi: PIN_3,
    tft_clk: PIN_2,
    tft_cs: PIN_5,
    tft_dc: PIN_1,
) -> ! {
    log::info!("Hello from lcd task (core 1)");
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
                .spawn(lcd_task(
                    spi0, bl, tft_miso, tft_mosi, tft_clk, tft_cs, tft_dc,
                ))
                .unwrap();
        });
    });

    // Block unused I2C0 pins
    let _ = Input::new(p.PIN_21, Pull::None);
    let _ = Input::new(p.PIN_24, Pull::None);

    log::info!("set up i2c0 ");
    let mut config = I2cConfig::default();
    config.frequency = 400_000;
    let i2c0: lasers::I2cBus0 = RpI2c::new_async(p.I2C0, p.PIN_13, p.PIN_12, Irqs, config);

    log::info!("set up i2c1 ");
    let mut config = I2cConfig::default();
    config.frequency = 400_000;
    let i2c1: rgb::I2cBus1 = RpI2c::new_async(p.I2C1, p.PIN_19, p.PIN_18, Irqs, config);

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| {
        spawner.spawn(logger_task(driver)).unwrap();
        spawner.spawn(lasers_task(i2c0)).unwrap();
        spawner.spawn(rgb_task(i2c1)).unwrap();
        spawner
            .spawn(imu_task(p.UART0, p.PIN_16, p.PIN_17))
            .unwrap();
        // spawner
        //     .spawn(esp32c3_task(p.UART1, p.PIN_8, p.PIN_9))
        //     .unwrap();
        spawner.spawn(trace_task()).unwrap();
        spawner
            .spawn(motors_task(
                p.PWM_CH6, p.PWM_CH5, p.PIN_27, p.PIN_28, p.PIN_29,
            ))
            .unwrap();
        spawner
            .spawn(buttons_task(left_button, right_button))
            .unwrap();
        spawner.spawn(main_task()).unwrap();
    });
}
