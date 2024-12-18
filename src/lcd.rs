use crate::race::Angle;
use crate::rgb::RgbEvent;
use crate::uformat;
use crate::uformat::FormattedText;
use crate::vision::{is_in_window, LaserData, LaserStatus, Vision, LASER_OVERFLOW};
use core::convert::Infallible;
use display_interface_spi::SPIInterface;
use embassy_rp::spi::{self, Spi};
use embassy_rp::{
    gpio::{Level, Output},
    peripherals::{PIN_0, PIN_1, PIN_2, PIN_3, PIN_4, PIN_5, SPI0},
};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Delay, Duration, Instant};
use embedded_graphics::mono_font::iso_8859_9::FONT_10X20;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::primitives::{
    Line, Primitive, PrimitiveStyle, PrimitiveStyleBuilder, StyledDrawable,
};
use embedded_graphics::text::Text;
use embedded_graphics_core::pixelcolor::Rgb565;
use embedded_graphics_core::prelude::{DrawTarget, Point, RgbColor, Size, WebColors};
use embedded_graphics_core::primitives::Rectangle;
use embedded_graphics_core::Drawable;
//use embedded_graphics_framebuf::FrameBuf;
use embedded_hal_0::digital::v2::OutputPin;
use mipidsi::Builder;

pub const STATES_COUNT: usize = 5;

const VISUAL_STATE_H_HEIGHT: i32 = 240 / 10;
const VISUAL_STATE_H_WIDTH: i32 = 135;
// const VISUAL_STATE_H_SIZE: usize =
//     (VISUAL_STATE_H_HEIGHT as usize) * (VISUAL_STATE_H_WIDTH as usize);
// pub type VisualStateHBuf = [Rgb565; VISUAL_STATE_H_SIZE];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VisualStateH {
    Empty,
    Text {
        text: &'static str,
        color: Rgb565,
    },
    Value {
        value: i16,
        color: Rgb565,
    },
    Value2 {
        value1: i16,
        value2: i16,
        color: Rgb565,
    },
    #[allow(unused)]
    Gauge {
        value: i16,
        max: i16,
        color: Rgb565,
    },
    Imu {
        yaw: i16,
        pitch: i16,
        roll: i16,
        climb: bool,
        downhill: bool,
    },
    Rgb {
        r: u16,
        g: u16,
        b: u16,
    },
    Hsv {
        r: u16,
        g: u16,
        b: u16,
        h: u16,
        s: u16,
        v: u16,
    },
    ImuAngles {
        roll: Angle,
        pitch: Angle,
        yaw: Angle,
    },
}

impl VisualStateH {
    pub fn empty(&mut self) {
        *self = Self::Empty
    }

    pub fn text(&mut self, text: &'static str) {
        *self = Self::Text {
            text,
            color: Rgb565::WHITE,
        }
    }

    pub fn text_red(&mut self, text: &'static str) {
        *self = Self::Text {
            text,
            color: Rgb565::RED,
        }
    }

    pub fn text_green(&mut self, text: &'static str) {
        *self = Self::Text {
            text,
            color: Rgb565::GREEN,
        }
    }

    pub fn text_blue(&mut self, text: &'static str) {
        *self = Self::Text {
            text,
            color: Rgb565::BLUE,
        }
    }

    pub fn value(&mut self, value: i16) {
        *self = Self::Value {
            value,
            color: Rgb565::WHITE,
        }
    }

    pub fn value2_red(&mut self, value1: i16, value2: i16) {
        *self = Self::Value2 {
            value1,
            value2,
            color: Rgb565::RED,
        }
    }

    pub fn value2_green(&mut self, value1: i16, value2: i16) {
        *self = Self::Value2 {
            value1,
            value2,
            color: Rgb565::GREEN,
        }
    }

    pub fn value2_blue(&mut self, value1: i16, value2: i16) {
        *self = Self::Value2 {
            value1,
            value2,
            color: Rgb565::BLUE,
        }
    }

    pub fn steer(&mut self, angle: i16) {
        *self = Self::Gauge {
            value: angle,
            max: 35,
            color: Rgb565::WHITE,
        }
    }

    pub fn target(&mut self, angle: i16, status: LaserStatus) {
        *self = Self::Gauge {
            value: angle,
            max: 60,
            color: match status {
                LaserStatus::Back => Rgb565::RED,
                LaserStatus::Alert => Rgb565::YELLOW,
                LaserStatus::Regular => Rgb565::BLUE,
                LaserStatus::Overflow => Rgb565::GREEN,
            },
        }
    }

    pub fn imu(&mut self, yaw: i16, pitch: i16, roll: i16, climb: bool, downhill: bool) {
        *self = Self::Imu {
            yaw,
            pitch,
            roll,
            climb,
            downhill,
        }
    }

    #[allow(unused)]
    pub fn power(&mut self, power: i16) {
        *self = Self::Value {
            value: power,
            color: if power > 0 {
                Rgb565::GREEN
            } else if power < 0 {
                Rgb565::RED
            } else {
                Rgb565::BLUE
            },
        }
    }

    pub fn rgb(&mut self, data: RgbEvent) {
        *self = Self::Rgb {
            r: data.r,
            g: data.g,
            b: data.b,
        }
    }

    pub fn hsv(&mut self, data: RgbEvent) {
        *self = Self::Hsv {
            r: data.r,
            g: data.g,
            b: data.b,
            h: data.h,
            s: data.s,
            v: data.v,
        }
    }

    pub fn imu_angles(&mut self, yaw: i16, pitch: i16, roll: i16) {
        *self = Self::ImuAngles {
            roll: Angle::from_imu_value(roll),
            pitch: Angle::from_imu_value(pitch),
            yaw: Angle::from_imu_value(yaw),
        }
    }

    pub fn position(index: usize) -> Point {
        Point {
            x: 0,
            y: VISUAL_STATE_H_HEIGHT * (index as i32),
        }
    }

    pub const fn size() -> Size {
        Size::new(VISUAL_STATE_H_WIDTH as u32, VISUAL_STATE_H_HEIGHT as u32)
    }

    pub fn needs_border(&self) -> bool {
        match self {
            VisualStateH::Empty { .. } => false,
            VisualStateH::Text { .. } => false,
            VisualStateH::Value { .. } => false,
            VisualStateH::Value2 { .. } => false,
            VisualStateH::Gauge { .. } => false,
            VisualStateH::Imu { .. } => true,
            VisualStateH::Rgb { .. } => false,
            VisualStateH::Hsv { .. } => false,
            VisualStateH::ImuAngles { .. } => false,
        }
    }

    pub fn needs_clearing(&self) -> bool {
        match self {
            VisualStateH::Empty { .. } => true,
            VisualStateH::Text { .. } => true,
            VisualStateH::Value { .. } => true,
            VisualStateH::Value2 { .. } => true,
            VisualStateH::Gauge { .. } => true,
            VisualStateH::Imu { .. } => true,
            VisualStateH::Rgb { .. } => false,
            VisualStateH::Hsv { .. } => false,
            VisualStateH::ImuAngles { .. } => true,
        }
    }

    pub fn draw(&self, index: usize, target: &mut impl DrawTarget<Color = Rgb565>) {
        if self.needs_clearing() {
            if self.needs_border() {
                let rectangle = Rectangle::new(Self::position(index), Self::size());
                let style = PrimitiveStyleBuilder::new()
                    .fill_color(Rgb565::BLACK)
                    .stroke_color(Rgb565::CSS_SLATE_GRAY)
                    .stroke_width(1) // > 1 is not currently supported in embedded-graphics on triangles
                    .build();
                rectangle.draw_styled(&style, target).ok();
            } else {
                target
                    .fill_solid(
                        &Rectangle::new(Self::position(index), Self::size()),
                        Rgb565::BLACK,
                    )
                    .ok();
            }
        }

        match *self {
            VisualStateH::Empty => {}
            VisualStateH::Text { text, color } => {
                let style = MonoTextStyle::new(&FONT_10X20, color);
                Text::new(
                    text,
                    Self::position(index)
                        + Point {
                            x: 2,
                            y: VISUAL_STATE_H_HEIGHT - 5,
                        },
                    style,
                )
                .draw(target)
                .ok();
            }
            VisualStateH::Value { value, color } => {
                let text = uformat!("{}", value);
                let style = MonoTextStyle::new(&FONT_10X20, color);
                Text::new(
                    text.as_str(),
                    Self::position(index)
                        + Point {
                            x: 2,
                            y: VISUAL_STATE_H_HEIGHT - 5,
                        },
                    style,
                )
                .draw(target)
                .ok();
            }
            VisualStateH::Value2 {
                value1,
                value2,
                color,
            } => {
                let text = uformat!("{} {}", value1, value2);
                let style = MonoTextStyle::new(&FONT_10X20, color);
                Text::new(
                    text.as_str(),
                    Self::position(index)
                        + Point {
                            x: 2,
                            y: VISUAL_STATE_H_HEIGHT - 5,
                        },
                    style,
                )
                .draw(target)
                .ok();
            }
            VisualStateH::Gauge { value, max, color } => {
                let center = VISUAL_STATE_H_WIDTH / 2;
                let delta = (value as i32) * (center - 1) / (max as i32);
                Line::new(
                    Self::position(index) + Point::new(center + delta, 1),
                    Self::position(index) + Point::new(center + delta, VISUAL_STATE_H_HEIGHT - 1),
                )
                .into_styled(PrimitiveStyle::<Rgb565>::with_stroke(color, 3))
                .draw(target)
                .ok();
            }
            VisualStateH::Imu {
                yaw,
                pitch,
                roll,
                climb,
                downhill,
            } => {
                let center_x = VISUAL_STATE_H_WIDTH / 2;
                let center_y = VISUAL_STATE_H_HEIGHT / 2;

                let (yaw, pitch, roll) = (yaw / 100, pitch / 100, roll / 100);

                let (yaw_delta, yaw_color) = if yaw < -90 {
                    (yaw + 180, Rgb565::YELLOW)
                } else if yaw < 90 {
                    (yaw, Rgb565::GREEN)
                } else {
                    (180 - yaw, Rgb565::YELLOW)
                };
                let yaw_x = center_x + ((yaw_delta as i32 * (center_x - 1)) / 90);
                let (pitch_delta, pitch_color) = if pitch < -90 {
                    (pitch + 180, Rgb565::RED)
                } else if pitch < 90 {
                    (
                        pitch,
                        if climb {
                            Rgb565::YELLOW
                        } else if downhill {
                            Rgb565::GREEN
                        } else {
                            Rgb565::BLUE
                        },
                    )
                } else {
                    (180 - pitch, Rgb565::RED)
                };
                let pitch_y = center_y - ((pitch_delta as i32 * (center_y - 1)) / 90);
                let (roll_delta, roll_color) = if roll < -90 {
                    (roll + 180, Rgb565::RED)
                } else if yaw < 90 {
                    (roll, Rgb565::BLUE)
                } else {
                    (180 - roll, Rgb565::RED)
                };
                let roll_x = center_x + ((roll_delta as i32 * (center_x - 1)) / 90);

                Line::new(
                    Self::position(index) + Point::new(yaw_x, 1),
                    Self::position(index) + Point::new(yaw_x, VISUAL_STATE_H_HEIGHT - 1),
                )
                .into_styled(PrimitiveStyle::<Rgb565>::with_stroke(yaw_color, 3))
                .draw(target)
                .ok();
                Line::new(
                    Self::position(index) + Point::new(1, pitch_y),
                    Self::position(index) + Point::new(VISUAL_STATE_H_WIDTH - 1, pitch_y),
                )
                .into_styled(PrimitiveStyle::<Rgb565>::with_stroke(pitch_color, 1))
                .draw(target)
                .ok();
                Line::new(
                    Self::position(index) + Point::new(roll_x, 1),
                    Self::position(index) + Point::new(roll_x, VISUAL_STATE_H_HEIGHT - 1),
                )
                .into_styled(PrimitiveStyle::<Rgb565>::with_stroke(roll_color, 1))
                .draw(target)
                .ok();
            }
            VisualStateH::Rgb { r, g, b } => {
                let rb = (r >> 2).min(255) as u8;
                let gb = (g >> 2).min(255) as u8;
                let bb = (b >> 2).min(255) as u8;
                let color = Rgb565::new(rb, gb, bb);

                target
                    .fill_solid(&Rectangle::new(Self::position(index), Self::size()), color)
                    .ok();

                let text = uformat!("{} {} {}", r, g, b);
                let style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
                Text::new(
                    text.as_str(),
                    Self::position(index)
                        + Point {
                            x: 2,
                            y: VISUAL_STATE_H_HEIGHT - 5,
                        },
                    style,
                )
                .draw(target)
                .ok();
            }
            VisualStateH::Hsv { r, g, b, h, s, v } => {
                let rb = (r >> 2).min(255) as u8;
                let gb = (g >> 2).min(255) as u8;
                let bb = (b >> 2).min(255) as u8;
                let color = Rgb565::new(rb, gb, bb);

                target
                    .fill_solid(&Rectangle::new(Self::position(index), Self::size()), color)
                    .ok();

                let text = uformat!("{} {} {}", h, s, v);
                let style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
                Text::new(
                    text.as_str(),
                    Self::position(index)
                        + Point {
                            x: 2,
                            y: VISUAL_STATE_H_HEIGHT - 5,
                        },
                    style,
                )
                .draw(target)
                .ok();
            }
            Self::ImuAngles { roll, pitch, yaw } => {
                let (r, p, y): (i32, i32, i32) = (roll.into(), pitch.into(), yaw.into());
                let rs = if r < 0 { "-" } else { " " };
                let r = r.abs();
                let r3 = (r / 100) % 10;
                let r2 = (r / 10) % 10;
                let r1 = (r / 1) % 10;
                let ps = if p < 0 { "-" } else { " " };
                let p = p.abs();
                let p3 = (p / 100) % 10;
                let p2 = (p / 10) % 10;
                let p1 = (p / 1) % 10;
                let ys = if y < 0 { "-" } else { " " };
                let y = y.abs();
                let y3 = (y / 100) % 10;
                let y2 = (y / 10) % 10;
                let y1 = (y / 1) % 10;
                let text = uformat!(
                    "{}{}{}{} {}{}{}{} {}{}{}{}",
                    rs,
                    r3,
                    r2,
                    r1,
                    ps,
                    p3,
                    p2,
                    p1,
                    ys,
                    y3,
                    y2,
                    y1
                );
                let style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
                Text::new(
                    text.as_str(),
                    Self::position(index)
                        + Point {
                            x: 2,
                            y: VISUAL_STATE_H_HEIGHT - 5,
                        },
                    style,
                )
                .draw(target)
                .ok();
            }
        }
    }
}

const VISUAL_STATE_V_HEIGHT: i32 = 240 / 2;
const VISUAL_STATE_V_WIDTH: i32 = 135 / 5;
// const VISUAL_STATE_V_SIZE: usize =
//     (VISUAL_STATE_V_HEIGHT as usize) * (VISUAL_STATE_V_WIDTH as usize);
// pub type VisualStateVBuf = [Rgb565; VISUAL_STATE_V_SIZE];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VisualStateV {
    Gauge {
        value: i16,
        max: i16,
        color: Rgb565,
    },
    Bar {
        value: u16,
        max: u16,
        mark: u16,
        color: Rgb565,
        mark_color: Rgb565,
        selected: bool,
    },
    Solid {
        color: Rgb565,
    },
}

impl VisualStateV {
    pub fn laser(&mut self, data: &LaserData, selected: bool) {
        let color = if data.slope {
            Rgb565::GREEN
        } else {
            match data.status {
                LaserStatus::Back => Rgb565::RED,
                LaserStatus::Alert => Rgb565::YELLOW,
                LaserStatus::Regular => Rgb565::BLUE,
                LaserStatus::Overflow => Rgb565::GREEN,
            }
        };
        *self = Self::Bar {
            value: data.lower.min(LASER_OVERFLOW),
            max: LASER_OVERFLOW,
            mark: data.upper.min(LASER_OVERFLOW),
            color,
            mark_color: Rgb565::WHITE,
            selected,
        }
    }

    pub fn empty(&mut self) {
        *self = Self::Solid {
            color: Rgb565::BLACK,
        }
    }

    pub fn solid(&mut self, color: Rgb565) {
        *self = Self::Solid { color }
    }

    pub fn green(&mut self) {
        self.solid(Rgb565::GREEN)
    }

    pub fn red(&mut self) {
        self.solid(Rgb565::RED)
    }

    pub fn yellow(&mut self) {
        self.solid(Rgb565::YELLOW)
    }

    pub fn black(&mut self) {
        self.solid(Rgb565::BLACK)
    }

    pub fn position(index: usize) -> Point {
        Point {
            x: VISUAL_STATE_V_WIDTH * (index as i32),
            y: VISUAL_STATE_V_HEIGHT,
        }
    }

    pub fn needs_clearing(&self) -> bool {
        match self {
            VisualStateV::Gauge { .. } => true,
            VisualStateV::Bar { .. } => true,
            VisualStateV::Solid { .. } => false,
        }
    }

    pub fn draw(&self, index: usize, target: &mut impl DrawTarget<Color = Rgb565>) {
        if self.needs_clearing() {
            target
                .fill_solid(
                    &Rectangle::new(
                        Self::position(index),
                        Size::new(VISUAL_STATE_V_WIDTH as u32, VISUAL_STATE_V_HEIGHT as u32),
                    ),
                    Rgb565::BLACK,
                )
                .ok();
        }

        match *self {
            VisualStateV::Gauge { value, max, color } => {
                let width = VISUAL_STATE_V_WIDTH - 2;
                let center = VISUAL_STATE_V_HEIGHT / 2;
                let delta = (value as i32) * (center - 1) / (max as i32);
                Line::new(
                    Self::position(index) + Point::new(1, center - delta),
                    Self::position(index) + Point::new(width + 1, center - delta),
                )
                .into_styled(PrimitiveStyle::<Rgb565>::with_stroke(color, 3))
                .draw(target)
                .ok();
            }
            VisualStateV::Bar {
                value,
                max,
                mark,
                color,
                mark_color,
                selected,
            } => {
                let width = VISUAL_STATE_V_WIDTH as u32 - 2;
                let height = (value as u32) * ((VISUAL_STATE_V_HEIGHT - 2) as u32) / (max as u32);
                let rectangle = Rectangle::new(
                    Self::position(index)
                        + Point::new(1, VISUAL_STATE_V_HEIGHT - (height as i32 + 1)),
                    Size::new(width, height),
                );

                let style = PrimitiveStyleBuilder::new()
                    .fill_color(color)
                    .stroke_color(Rgb565::CSS_SLATE_GRAY)
                    .stroke_width(1) // > 1 is not currently supported in embedded-graphics on triangles
                    .build();
                rectangle.draw_styled(&style, target).ok();
                let mark_value =
                    (mark as i32) * ((VISUAL_STATE_V_HEIGHT - 2) as i32) / (max as i32);
                let mark_height = VISUAL_STATE_V_HEIGHT - (mark_value + 1);
                Line::new(
                    Self::position(index) + Point::new(1, mark_height),
                    Self::position(index) + Point::new(width as i32 + 1, mark_height),
                )
                .into_styled(PrimitiveStyle::<Rgb565>::with_stroke(mark_color, 3))
                .draw(target)
                .ok();
                if selected {
                    Line::new(
                        Self::position(index) + Point::new(1, 0),
                        Self::position(index) + Point::new(width as i32 + 1, 0),
                    )
                    .into_styled(PrimitiveStyle::<Rgb565>::with_stroke(Rgb565::GREEN, 3))
                    .draw(target)
                    .ok();
                }
            }
            VisualStateV::Solid { color } => {
                target
                    .fill_solid(
                        &Rectangle::new(
                            Self::position(index),
                            Size::new(VISUAL_STATE_V_WIDTH as u32, VISUAL_STATE_V_HEIGHT as u32),
                        ),
                        color,
                    )
                    .ok();
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct VisualState {
    pub values_h: [VisualStateH; STATES_COUNT],
    pub values_v: [VisualStateV; STATES_COUNT],
}

impl VisualState {
    pub fn init() -> Self {
        Self {
            values_h: [VisualStateH::Text {
                text: "",
                color: Rgb565::BLACK,
            }; STATES_COUNT],
            values_v: [VisualStateV::Gauge {
                value: 0,
                max: 1,
                color: Rgb565::BLACK,
            }; STATES_COUNT],
        }
    }

    pub fn update_vision(&mut self, v: &Vision, window: Option<(usize, usize)>) {
        for (i, vv) in self.values_v.iter_mut().enumerate() {
            vv.laser(&v.lasers[i], is_in_window(i, window));
        }
    }

    pub fn solid(&mut self, color: Rgb565) {
        self.values_v[0].solid(color);
        self.values_v[1].solid(color);
        self.values_v[2].solid(color);
        self.values_v[3].solid(color);
        self.values_v[4].solid(color);
    }

    pub fn green(&mut self) {
        self.solid(Rgb565::GREEN);
    }
    pub fn red(&mut self) {
        self.solid(Rgb565::RED);
    }
    pub fn yellow(&mut self) {
        self.solid(Rgb565::YELLOW);
    }
    pub fn blue(&mut self) {
        self.solid(Rgb565::BLUE);
    }
    pub fn black(&mut self) {
        self.solid(Rgb565::BLACK);
    }
    pub fn white(&mut self) {
        self.solid(Rgb565::WHITE);
    }
}

pub static VISUAL_STATE: Signal<CriticalSectionRawMutex, VisualState> = Signal::new();

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

pub async fn tft_task(
    spi: SPI0,
    bl: PIN_4,
    tft_miso: PIN_0,
    tft_mosi: PIN_3,
    tft_clk: PIN_2,
    tft_cs: PIN_5,
    tft_dc: PIN_1,
) -> ! {
    let mut tft_bl = Output::new(bl, Level::High);
    tft_bl.set_high();
    let mut tft_delay = Delay;
    let mut config = spi::Config::default();
    config.frequency = 27_000_000;
    let spi = Spi::new_blocking(spi, tft_clk, tft_mosi, tft_miso, config);
    let di = SPIInterface::new(
        spi,
        TftDc::new(tft_dc, Level::Low),
        TftCs::new(tft_cs, Level::Low),
    );
    let mut display = Builder::st7789_pico1(di)
        .init::<TftRst>(&mut tft_delay, None)
        .unwrap();
    display.clear(Rgb565::BLACK).unwrap();

    let mut current_state = VisualState::init();

    loop {
        const MIN_FRAME_DT: Duration = Duration::from_millis(50);
        let new_state = VISUAL_STATE.wait().await;
        let start = Instant::now();

        for (i, s) in new_state.values_h.iter().copied().enumerate() {
            if current_state.values_h[i] != s {
                s.draw(i, &mut display);
                current_state.values_h[i] = s;
            }
        }
        for (i, s) in new_state.values_v.iter().copied().enumerate() {
            if current_state.values_v[i] != s {
                s.draw(i, &mut display);
                current_state.values_v[i] = s;
            }
        }

        let done = Instant::now();
        let elapsed = done - start;
        if elapsed < MIN_FRAME_DT {
            embassy_time::Timer::after(MIN_FRAME_DT - elapsed).await;
        }
    }
}
