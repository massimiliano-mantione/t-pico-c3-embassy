use crate::uformat;
use crate::uformat::FormattedText;
use core::convert::Infallible;
use display_interface_spi::SPIInterface;
use embassy_rp::spi::{self, Spi};
use embassy_rp::{
    gpio::{Level, Output},
    peripherals::{PIN_0, PIN_1, PIN_16, PIN_2, PIN_3, PIN_4, PIN_5, SPI0},
};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Delay;
use embedded_graphics::mono_font::ascii::FONT_9X18_BOLD;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::primitives::{
    Circle, Line, Primitive, PrimitiveStyle, PrimitiveStyleBuilder, Triangle,
};
use embedded_graphics::text::Text;
use embedded_graphics_core::pixelcolor::Rgb565;
use embedded_graphics_core::prelude::{DrawTarget, Point, RgbColor};
use embedded_graphics_core::Drawable;
use embedded_hal_0::digital::v2::OutputPin;
use mipidsi::Builder;

#[derive(Clone, Copy)]
pub struct VisualState {
    pub value: u16,
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
    display.clear(Rgb565::WHITE).unwrap();
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

    loop {
        let s = VISUAL_STATE.wait().await;
        log::info!("value: {}", s.value);
        let t = uformat!("V: {}", s.value);
        let style = MonoTextStyle::new(&FONT_9X18_BOLD, Rgb565::WHITE);
        Text::new(
            t.as_str(),
            Point {
                x: 35 / 2,
                y: 240 / 2,
            },
            style,
        )
        .draw(&mut display)
        .ok();
    }
}
