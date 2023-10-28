use embassy_rp::{
    gpio::Input,
    peripherals::{PIN_6, PIN_7},
};
use embassy_time::{Duration, Instant};

use crate::cmd::{Cmd, CMD};

pub type LeftButton = Input<'static, PIN_6>;
pub type RightButton = Input<'static, PIN_7>;

const DEBOUNCE: Duration = Duration::from_micros(900);
const HOLD: Duration = Duration::from_millis(370);

pub async fn buttons_task(mut left_button: LeftButton, mut right_button: RightButton) {
    let mut left_pressed = None;
    let mut right_pressed = None;
    loop {
        embassy_futures::select::select(
            left_button.wait_for_any_edge(),
            right_button.wait_for_any_edge(),
        )
        .await;

        let left = left_button.is_low();
        let right = right_button.is_low();
        let now = Instant::now();

        let mut forget_left = false;
        let mut forget_right = false;

        if left {
            left_pressed = Some(now);
        } else {
            if let Some(pressed) = left_pressed.take() {
                let elapsed = now - pressed;
                if right {
                    if elapsed > HOLD {
                        // no cmd: cancel double press
                        forget_right = true;
                    } else if elapsed > DEBOUNCE {
                        CMD.signal(Cmd::Ok);
                        forget_right = true;
                    }
                } else {
                    if elapsed > HOLD {
                        CMD.signal(Cmd::Minus);
                    } else if elapsed > DEBOUNCE {
                        CMD.signal(Cmd::Previous);
                    }
                }
            }
        }

        if right {
            right_pressed = Some(now);
        } else {
            if let Some(pressed) = right_pressed.take() {
                let elapsed = now - pressed;
                if left {
                    if elapsed > HOLD {
                        // no cmd: cancel double press
                        forget_left = true;
                    } else if elapsed > DEBOUNCE {
                        CMD.signal(Cmd::Exit);
                        forget_left = true;
                    }
                } else {
                    if elapsed > HOLD {
                        CMD.signal(Cmd::Plus);
                    } else if elapsed > DEBOUNCE {
                        CMD.signal(Cmd::Next);
                    }
                }
            }
        }

        if forget_left {
            left_pressed = None;
        }
        if forget_right {
            right_pressed = None;
        }
    }
}
