#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use core::str;
use embassy_executor::Executor;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::i2c::{Config as I2cConfig, I2c as RpI2c, InterruptHandler as InterruptHandlerI2c};
use embassy_rp::multicore::{spawn_core1, Stack};
use embassy_rp::peripherals::{
    I2C0, PIN_0, PIN_1, PIN_10, PIN_12, PIN_13, PIN_16, PIN_17, PIN_2, PIN_25, PIN_26, PIN_27,
    PIN_28, PIN_29, PIN_3, PIN_4, PIN_5, PIN_6, PIN_7, PIN_8, PIN_9, PWM_CH5, PWM_CH6, SPI0, UART0,
    UART1, USB,
};
use embassy_rp::uart::BufferedInterruptHandler;
use embassy_rp::usb::{Driver, InterruptHandler as InterruptHandlerUsb};
use rp2040_panic_usb_boot as _;
use static_cell::StaticCell;

pub mod esp32c3;
mod imu;
mod lasers;
mod lcd;
mod motors;
mod uformat;

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

static mut CORE1_STACK: Stack<16384> = Stack::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();

#[embassy_executor::task]
async fn core0_task(mut input: Input<'static, PIN_7>) -> ! {
    log::info!("Hello from core 0");
    loop {
        input.wait_for_any_edge().await;

        if lasers::RAW_LASER_READINGS.signaled() {
            let l = lasers::RAW_LASER_READINGS.wait().await;
            log::info!(
                "L {} {} {} {} {} {} {} {}",
                l[0],
                l[1],
                l[2],
                l[3],
                l[4],
                l[5],
                l[6],
                l[7]
            );
            log::info!("core 0 sends value");
            lcd::VISUAL_STATE.signal(lcd::VisualState { value: l[0] });
        }

        if imu::IMU_DATA.signaled() {
            let data = imu::IMU_DATA.wait().await;
            log::info!(
                "IMU R {} P {} Y {}   F {} S {} V {}",
                data.roll,
                data.pitch,
                data.yaw,
                data.forward,
                data.side,
                data.vertical
            );
        }

        match input.get_level() {
            Level::Low => {
                log::info!("button 0 ON");
            }
            Level::High => {
                log::info!("button 0 OFF");
            }
        }
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

    let driver = Driver::new(p.USB, Irqs);
    let led = Output::new(p.PIN_25, Level::Low);

    // Init button pins
    let left_pin = Input::new(p.PIN_6, Pull::Up);
    let right_pin = Input::new(p.PIN_7, Pull::Up);

    // Eventually reboot to bootsel
    if left_pin.is_low() || right_pin.is_low() {
        rp2040_hal::rom_data::reset_to_usb_boot(0, 0);
    }

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
    let i2c: lasers::I2cBus =
        RpI2c::new_async(p.I2C0, p.PIN_13, p.PIN_12, Irqs, I2cConfig::default());

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
        spawner.spawn(core0_task(right_pin)).unwrap();
    });

    // let mut pwm_1 = Pwm::new_output_ab(p.PWM_CH1, p.PIN_2, p.PIN_3, pwm_config(0, 0));
    // let mut pwm_2 = Pwm::new_output_ab(p.PWM_CH3, p.PIN_6, p.PIN_7, pwm_config(0, 0));

    //     pwm_1.set_config(&c1);
    //     pwm_2.set_config(&c2);
}
