use embassy_rp::{
    peripherals::{PIN_27, PIN_28, PIN_29, PWM_CH5, PWM_CH6},
    pwm::{Config, Pwm},
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Timer};
use fixed::traits::ToFixed;

const MOTOR_DIV_INT: u8 = 250;
const MOTOR_TOP: u16 = 10000;

const SERVO_DIV_INT: u8 = 250;
const SERVO_TOP: u16 = 10000;

const SERVO_CENTER_DUTY: u16 = 700;
const SERVO_MAX_DELTA_DUTY: u16 = 350;
const SERVO_MAX_DUTY: u16 = SERVO_CENTER_DUTY + SERVO_MAX_DELTA_DUTY;
const SERVO_MIN_DUTY: u16 = SERVO_CENTER_DUTY - SERVO_MAX_DELTA_DUTY;

fn pwm_config_motor(power: i16) -> Config {
    let (duty_a, duty_b) = if power > 0 {
        (power as u16, 0)
    } else if power < 0 {
        (0, (-power) as u16)
    } else {
        (0, 0)
    };

    let mut c = Config::default();
    c.invert_a = false;
    c.invert_b = false;
    c.phase_correct = false;
    c.enable = true;
    c.divider = MOTOR_DIV_INT.to_fixed();
    c.compare_a = duty_a;
    c.compare_b = duty_b;
    c.top = MOTOR_TOP;
    c
}

fn pwm_config_servo(steer: i16) -> Config {
    let duty_b = (((-steer * 10) + (SERVO_CENTER_DUTY as i16)) as u16)
        .min(SERVO_MAX_DUTY)
        .max(SERVO_MIN_DUTY);

    let mut c = Config::default();
    c.invert_a = false;
    c.invert_b = false;
    c.phase_correct = false;
    c.enable = true;
    c.divider = SERVO_DIV_INT.to_fixed();
    c.compare_a = 0;
    c.compare_b = duty_b;
    c.top = SERVO_TOP;
    c
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct MotorsData {
    pub power: i16,
    pub steer: i16,
}

static MOTORS_DATA: Signal<CriticalSectionRawMutex, MotorsData> = Signal::new();

pub fn motors_go(power: i16, steer: i16) {
    MOTORS_DATA.signal(MotorsData { power, steer })
}

pub fn motors_stop() {
    MOTORS_DATA.signal(MotorsData { power: 0, steer: 0 })
}

pub async fn motors_task(
    pwm_ch6: PWM_CH6,
    pwm_ch5: PWM_CH5,
    pin27: PIN_27,
    pin28: PIN_28,
    pin29: PIN_29,
) {
    let mut pwm_motor = Pwm::new_output_ab(pwm_ch6, pin28, pin29, pwm_config_motor(0));
    let mut pwm_servo = Pwm::new_output_b(pwm_ch5, pin27, pwm_config_servo(0));

    for s in 0..7i16 {
        let steer_sign = if s % 2 == 0 { -1 } else { 1 };
        let steer_angle = ((s + 1) / 2) * 10;
        let servo_config = pwm_config_servo(steer_angle * steer_sign);
        pwm_servo.set_config(&servo_config);
        Timer::after(Duration::from_millis(200)).await;
    }
    let servo_config = pwm_config_servo(0);
    pwm_servo.set_config(&servo_config);

    for i in 0..8 {
        let power_sign = if i % 2 == 0 { -1 } else { 1 };
        let power_config = pwm_config_motor(power_sign * 3000);
        pwm_motor.set_config(&power_config);
        Timer::after(Duration::from_millis(100)).await;
    }
    let power_config = pwm_config_motor(0);
    pwm_motor.set_config(&power_config);

    loop {
        let data = MOTORS_DATA.wait().await;
        let motor_config = pwm_config_motor(data.power);
        let servo_config = pwm_config_servo(data.steer);
        pwm_motor.set_config(&motor_config);
        pwm_servo.set_config(&servo_config);
    }
}
