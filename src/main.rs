#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use core::str;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Input, Level, Pull};
use embassy_rp::peripherals::USB;
use embassy_rp::pwm::{Config as PwmConfig, Pwm};
use embassy_rp::usb::{Driver, InterruptHandler as InterruptHandlerUsb};
use embassy_time::{Duration, Timer};
use fixed::traits::ToFixed;
use rp2040_panic_usb_boot as _;

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

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let driver = Driver::new(p.USB, Irqs);
    spawner.spawn(logger_task(driver)).unwrap();

    log::info!("starting");

    let gp0 = Input::new(p.PIN_0, Pull::None);
    let gp1 = Input::new(p.PIN_1, Pull::None);
    let gp26 = Input::new(p.PIN_26, Pull::None);
    let gp27 = Input::new(p.PIN_27, Pull::None);

    let mut pwm_1 = Pwm::new_output_ab(p.PWM_CH1, p.PIN_2, p.PIN_3, pwm_config(0, 0));
    let mut pwm_2 = Pwm::new_output_ab(p.PWM_CH3, p.PIN_6, p.PIN_7, pwm_config(0, 0));

    for counter in 0..10 {
        log::info!("sleeping... {}", counter);
        Timer::after(Duration::from_secs(1)).await;
    }

    loop {
        let l0 = gp0.get_level();
        let l1 = gp1.get_level();
        let l26 = gp26.get_level();
        let l27 = gp27.get_level();
        log::info!(
            "IN: 0:{} 1:{} 26:{} 27:{}",
            level2str(l0),
            level2str(l1),
            level2str(l26),
            level2str(l27)
        );

        let c1 = if l0 == Level::High {
            if l26 == Level::High {
                pwm_config(0, PWM_TOP)
            } else {
                pwm_config(PWM_TOP, 0)
            }
        } else {
            pwm_config(0, 0)
        };
        let c2 = if l1 == Level::High {
            if l26 == Level::High {
                pwm_config(0, PWM_TOP)
            } else {
                pwm_config(PWM_TOP, 0)
            }
        } else {
            pwm_config(0, 0)
        };

        pwm_1.set_config(&c1);
        pwm_2.set_config(&c2);
        Timer::after(Duration::from_millis(100)).await;
    }
}
