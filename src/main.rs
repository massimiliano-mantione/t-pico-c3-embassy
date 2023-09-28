#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use core::str;
use embassy_executor::{Executor, Spawner};
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::multicore::{spawn_core1, Stack};
use embassy_rp::peripherals::{PIN_25, USB};
use embassy_rp::pwm::{Config as PwmConfig, Pwm};
use embassy_rp::usb::{Driver, InterruptHandler as InterruptHandlerUsb};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Timer};
use fixed::traits::ToFixed;
use rp2040_panic_usb_boot as _;
use static_cell::StaticCell;

//const WIFI_SSID: &'static str = include_str!("WIFI_SSID.txt");
//const WIFI_SECRET: &'static str = include_str!("WIFI_SECRET.txt");

const PWN_DIV_INT: u8 = 5;
const PWM_TOP: u16 = 1000;

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandlerUsb<USB>;
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

#[embassy_executor::task]
async fn core1_task(mut led: Output<'static, PIN_25>) -> ! {
    log::info!("Hello from core 1");
    loop {
        match CHANNEL.recv().await {
            LedState::On => {
                log::info!("core 1 gets ON");
                led.set_high();
            }
            LedState::Off => {
                log::info!("core 1 gets OFF");
                led.set_low();
            }
        }
    }
}

#[cortex_m_rt::entry]
fn main() -> ! {
    let p = embassy_rp::init(Default::default());
    let driver = Driver::new(p.USB, Irqs);
    let led = Output::new(p.PIN_25, Level::Low);

    spawn_core1(p.CORE1, unsafe { &mut CORE1_STACK }, move || {
        let executor1 = EXECUTOR1.init(Executor::new());
        executor1.run(|spawner| {
            spawner.spawn(core1_task(led)).unwrap();
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
