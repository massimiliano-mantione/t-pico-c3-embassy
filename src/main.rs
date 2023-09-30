#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use core::convert::Infallible;
use core::str;
use display_interface_spi::SPIInterface;
use embassy_executor::{Executor, Spawner};
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::i2c::{
    AbortReason as I2cAbortReason, Async, Config as I2cConfig, Error as I2cError, I2c as RpI2c,
    InterruptHandler as InterruptHandlerI2c,
};
use embassy_rp::multicore::{spawn_core1, Stack};
use embassy_rp::peripherals::{I2C0, PIN_0, PIN_1, PIN_12, PIN_13, PIN_25, PIN_5, USB};
use embassy_rp::pwm::{Config as PwmConfig, Pwm};
use embassy_rp::spi::{self, Spi};
use embassy_rp::usb::{Driver, InterruptHandler as InterruptHandlerUsb};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Delay, Duration, Timer};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics_core::pixelcolor::Rgb565;
use embedded_graphics_core::pixelcolor::RgbColor;
use embedded_graphics_core::prelude::DrawTarget;
use embedded_hal_0::digital::v2::OutputPin;
use embedded_hal_1::delay::DelayUs;
use embedded_hal_async::i2c::I2c;
use fixed::traits::ToFixed;
use mipidsi::Builder;
use rp2040_panic_usb_boot as _;
use static_cell::StaticCell;

//const WIFI_SSID: &'static str = include_str!("WIFI_SSID.txt");
//const WIFI_SECRET: &'static str = include_str!("WIFI_SECRET.txt");

const PWN_DIV_INT: u8 = 5;
const PWM_TOP: u16 = 1000;

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandlerUsb<USB>;
    I2C0_IRQ => InterruptHandlerI2c<I2C0>;
});

#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

fn level2str(l: Level) -> &'static str {
    match l {
        Level::Low => "LO",
        Level::High => "HI",
    }
}

fn pwm_config(duty_a: u16, duty_b: u16) -> PwmConfig {
    let mut c = PwmConfig::default();
    c.invert_a = false;
    c.invert_b = false;
    c.phase_correct = false;
    c.enable = true;
    c.divider = PWN_DIV_INT.to_fixed();
    c.compare_a = duty_a;
    c.compare_b = duty_b;
    c.top = PWM_TOP;
    c
}

static mut CORE1_STACK: Stack<4096> = Stack::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();
static CHANNEL: Channel<CriticalSectionRawMutex, LedState, 1> = Channel::new();

enum LedState {
    On,
    Off,
}

#[embassy_executor::task]
async fn core0_task() -> ! {
    log::info!("Hello from core 0");
    loop {
        log::info!("core 0 sends ON");
        CHANNEL.send(LedState::On).await;
        Timer::after(Duration::from_millis(100)).await;
        log::info!("core 0 sends OFF");
        CHANNEL.send(LedState::Off).await;
        Timer::after(Duration::from_millis(400)).await;
    }
}

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

type I2cBus = RpI2c<'static, I2C0, Async>;
const TCA9548A_ADDR: u16 = 0x70;
const DISTANCE_SENSOR_ADDR: u16 = 0x80 >> 1;
const DISTANCE_SENSOR_REG: u8 = 0x5E;

async fn select_i2c_channel(i2c: &mut I2cBus, chan: usize) {
    if let Err(err) = i2c.write_async(TCA9548A_ADDR, [1 << chan]).await {
        log::error!("I2C select write error: {}", i2c_error_message(&err));
    } else {
        log::info!("I2C select OK");
    }
}

async fn read_distance(i2c: &mut I2cBus) -> u16 {
    let mut result_buf = [0u8; 2];
    match i2c
        .write_read(
            DISTANCE_SENSOR_ADDR,
            &[DISTANCE_SENSOR_REG],
            &mut result_buf,
        )
        .await
    {
        Ok(_) => {
            let distance = ((result_buf[0] as u16) << 8) | (result_buf[1] as u16);
            distance
        }
        Err(err) => {
            log::error!("I2C read distance error: {}", i2c_error_message(&err));
            0
        }
    }
}

#[embassy_executor::task]
async fn core1_task(mut led: Output<'static, PIN_25>, sda: PIN_12, scl: PIN_13, i2c_p: I2C0) -> ! {
    log::info!("set up i2c ");
    let mut i2c: I2cBus = RpI2c::new_async(i2c_p, scl, sda, Irqs, I2cConfig::default());

    log::info!("Hello from core 1");
    loop {
        let chan = match CHANNEL.recv().await {
            LedState::On => {
                log::info!("core 1 gets ON");
                led.set_high();
                1
            }
            LedState::Off => {
                log::info!("core 1 gets OFF");
                led.set_low();
                0
            }
        };
        // select_i2c_channel(&mut i2c, chan).await;
        log::info!(
            "I2C read distance {}: {}",
            chan,
            read_distance(&mut i2c).await
        );
    }
}

struct TftPin<'a, PIN: embassy_rp::gpio::Pin> {
    pin: Output<'a, PIN>,
}

impl<'a, PIN> TftPin<'a, PIN>
where
    PIN: embassy_rp::gpio::Pin,
{
    pub fn new(pin: PIN, initial_output: Level) -> Self {
        Self {
            pin: Output::new(pin, initial_output),
        }
    }
}

impl<'a, PIN> OutputPin for TftPin<'a, PIN>
where
    PIN: embassy_rp::gpio::Pin,
{
    type Error = Infallible;

    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.pin.set_low();
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.pin.set_high();
        Ok(())
    }
}

type TftDc<'a> = TftPin<'a, PIN_1>;
type TftCs<'a> = TftPin<'a, PIN_5>;
type TftRst<'a> = TftPin<'a, PIN_0>;

#[cortex_m_rt::entry]
fn main() -> ! {
    let p = embassy_rp::init(Default::default());
    let driver = Driver::new(p.USB, Irqs);
    let led = Output::new(p.PIN_25, Level::Low);

    let tft_miso = p.PIN_16;
    let tft_mosi = p.PIN_3;
    let tft_clk = p.PIN_2;
    let tft_cs = p.PIN_5;
    let tft_dc = p.PIN_1;
    let mut tft_bl = Output::new(p.PIN_4, Level::High);
    tft_bl.set_high();

    let mut tft_delay = Delay;

    let mut config = spi::Config::default();
    config.frequency = 27_000_000;
    let spi = Spi::new_blocking(p.SPI0, tft_clk, tft_mosi, tft_miso, config);

    let di = SPIInterface::new(
        spi,
        TftDc::new(tft_dc, Level::Low),
        TftCs::new(tft_cs, Level::Low),
    );

    let mut display = Builder::st7789_pico1(di)
        .init::<TftRst>(&mut tft_delay, None)
        .unwrap();
    display.clear(Rgb565::WHITE).unwrap();
    tft_delay.delay_ms(500);
    display.clear(Rgb565::BLACK).unwrap();

    let circle1 =
        Circle::new(Point::new(128, 64), 64).into_styled(PrimitiveStyle::with_fill(Rgb565::RED));
    let circle2 = Circle::new(Point::new(64, 64), 64)
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::GREEN, 1));

    let blue_with_red_outline = PrimitiveStyleBuilder::new()
        .fill_color(Rgb565::BLUE)
        .stroke_color(Rgb565::RED)
        .stroke_width(1) // > 1 is not currently supported in embedded-graphics on triangles
        .build();
    let triangle = Triangle::new(
        Point::new(40, 120),
        Point::new(40, 220),
        Point::new(140, 120),
    )
    .into_styled(blue_with_red_outline);
    let line =
        Line::new(Point::new(180, 160), Point::new(239, 239))
            .into_styled(PrimitiveStyle::<Rgb565>::with_stroke(Rgb565::WHITE, 10));
    circle1.draw(&mut display).ok();
    circle2.draw(&mut display).ok();
    triangle.draw(&mut display).ok();
    line.draw(&mut display).ok();

    spawn_core1(p.CORE1, unsafe { &mut CORE1_STACK }, move || {
        let executor1 = EXECUTOR1.init(Executor::new());
        executor1.run(|spawner| {
            spawner
                .spawn(core1_task(led, p.PIN_12, p.PIN_13, p.I2C0))
                .unwrap();
        });
    });

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| {
        spawner.spawn(logger_task(driver)).unwrap();
        spawner.spawn(core0_task()).unwrap();
    });

    // log::info!("starting");

    // let gp0 = Input::new(p.PIN_0, Pull::None);
    // let gp1 = Input::new(p.PIN_1, Pull::None);
    // let gp26 = Input::new(p.PIN_26, Pull::None);
    // let gp27 = Input::new(p.PIN_27, Pull::None);

    // let mut pwm_1 = Pwm::new_output_ab(p.PWM_CH1, p.PIN_2, p.PIN_3, pwm_config(0, 0));
    // let mut pwm_2 = Pwm::new_output_ab(p.PWM_CH3, p.PIN_6, p.PIN_7, pwm_config(0, 0));

    // for counter in 0..10 {
    //     log::info!("sleeping... {}", counter);
    //     Timer::after(Duration::from_secs(1)).await;
    // }

    // loop {
    //     let l0 = gp0.get_level();
    //     let l1 = gp1.get_level();
    //     let l26 = gp26.get_level();
    //     let l27 = gp27.get_level();
    //     log::info!(
    //         "IN: 0:{} 1:{} 26:{} 27:{}",
    //         level2str(l0),
    //         level2str(l1),
    //         level2str(l26),
    //         level2str(l27)
    //     );

    //     let c1 = if l0 == Level::High {
    //         if l26 == Level::High {
    //             pwm_config(0, PWM_TOP)
    //         } else {
    //             pwm_config(PWM_TOP, 0)
    //         }
    //     } else {
    //         pwm_config(0, 0)
    //     };
    //     let c2 = if l1 == Level::High {
    //         if l26 == Level::High {
    //             pwm_config(0, PWM_TOP)
    //         } else {
    //             pwm_config(PWM_TOP, 0)
    //         }
    //     } else {
    //         pwm_config(0, 0)
    //     };

    //     pwm_1.set_config(&c1);
    //     pwm_2.set_config(&c2);
    //     Timer::after(Duration::from_millis(100)).await;
    // }
}
