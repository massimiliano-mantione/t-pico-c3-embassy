use embassy_rp::{
    peripherals::{PIN_16, PIN_17, UART0},
    uart::{BufferedUart, Config},
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Instant};
use embedded_io_async::Read;
use static_cell::make_static;

pub static IMU_DATA: Signal<CriticalSectionRawMutex, ImuData> = Signal::new();

pub async fn imu_task(uart0: UART0, pin_16: PIN_16, pin_17: PIN_17) {
    let tx_buf = &mut make_static!([0u8; 16])[..];
    let rx_buf = &mut make_static!([0u8; 16])[..];
    let uart = BufferedUart::new(
        uart0,
        super::Irqs,
        pin_16,
        pin_17,
        tx_buf,
        rx_buf,
        Config::default(),
    );
    let (mut rx, _) = uart.split();

    let mut decoder = Bno080Decoder::init();
    let mut data = ImuData::init();

    loop {
        let mut buf = [0; 1];

        log::info!("reading IMU");

        match rx.read_exact(&mut buf).await {
            Ok(_) => {
                log::info!("IMU byte");

                let received = buf[0];
                if let Some(raw) = decoder.update(received) {
                    data.update(&raw);
                    IMU_DATA.signal(data);
                }
            }
            Err(err) => {
                log::info!("IMU uart read error: {}", err);
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct Bno080RawRvcData {
    yaw: i16,
    pitch: i16,
    roll: i16,
    side: i16,
    forward: i16,
    vertical: i16,
    counter: u8,
}

#[derive(Clone, Copy)]
pub struct ImuData {
    pub yaw: i16,
    pub pitch: i16,
    pub roll: i16,
    pub yaw_speed: i16,
    pub pitch_speed: i16,
    pub roll_speed: i16,
    pub side: i16,
    pub forward: i16,
    pub vertical: i16,
    pub since: Instant,
    pub dt: Duration,
}

const DT_MIN: Duration = Duration::from_micros(100);

fn angle_speed(a1: i16, a2: i16, dt: Duration) -> i16 {
    let a1 = a1 as i32 * 1000000;
    let a2 = a2 as i32 * 1000000;
    let dt = dt.as_micros() as i32;
    let s = (a1 - a2) / dt;
    s as i16
}

impl ImuData {
    pub fn init() -> Self {
        Self {
            yaw: 0,
            pitch: 0,
            roll: 0,
            yaw_speed: 0,
            pitch_speed: 0,
            roll_speed: 0,
            side: 0,
            forward: 0,
            vertical: 0,
            since: Instant::now(),
            dt: DT_MIN,
        }
    }
    pub fn update(&mut self, data: &Bno080RawRvcData) {
        let now = Instant::now();
        let dt = now - self.since;
        let dt = if dt.as_micros() == 0 { DT_MIN } else { dt };

        let yaw_speed = angle_speed(data.yaw, self.yaw, dt);
        let pitch_speed = angle_speed(data.pitch, self.pitch, dt);
        let roll_speed = angle_speed(data.roll, self.roll, dt);

        self.yaw = data.yaw;
        self.pitch = data.pitch;
        self.roll = data.roll;
        self.yaw_speed = yaw_speed;
        self.pitch_speed = pitch_speed;
        self.roll_speed = roll_speed;
        self.side = data.side;
        self.forward = data.forward;
        self.vertical = data.vertical;

        self.since = now;
        self.dt = dt;
    }
}

const HEADER_BYTE: u8 = 0xaa;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum ExpectedByte {
    #[default]
    Header1,
    Header2,
    Counter,
    YavLsb,
    YavMsb,
    PitchLsb,
    PitchMsb,
    RollLsb,
    RollMsb,
    SideLsb,
    SideMsb,
    ForwardLsb,
    ForwardMsb,
    VerticalLsb,
    VerticalMsb,
    Unused1,
    Unused2,
    Unused3,
    Checksum,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct Bno080Decoder {
    expected: ExpectedByte,
    current: Bno080RawRvcData,
    checksum: u8,
    lsb: u8,
}

impl Bno080Decoder {
    pub fn init() -> Self {
        Self::default()
    }

    fn handle_lsb(&mut self, received: u8, expected: ExpectedByte, msg: &'static str) {
        if received == HEADER_BYTE {
            log::info!("{}", msg);
            self.expected = ExpectedByte::Header2;
        } else {
            self.lsb = received;
            self.checksum = self.checksum.wrapping_add(received);
            self.expected = expected;
        }
    }

    fn value(&self, received: u8) -> i16 {
        ((self.lsb as u16) | ((received as u16) << 8)) as i16
    }

    fn process_msb(&mut self, received: u8) -> i16 {
        self.checksum = self.checksum.wrapping_add(received);
        self.value(received)
    }

    fn process_unused(&mut self, received: u8, index: usize) -> bool {
        if received != HEADER_BYTE {
            self.checksum = self.checksum.wrapping_add(received);
            true
        } else {
            log::info!("IMU: no unused {}", index);
            self.expected = ExpectedByte::Header2;
            false
        }
    }

    pub fn update(&mut self, received: u8) -> Option<Bno080RawRvcData> {
        match self.expected {
            ExpectedByte::Header1 => {
                if received == HEADER_BYTE {
                    self.expected = ExpectedByte::Header2;
                }
                None
            }
            ExpectedByte::Header2 => {
                if received == HEADER_BYTE {
                    self.expected = ExpectedByte::Counter;
                } else {
                    self.expected = ExpectedByte::Header1;
                }
                None
            }
            ExpectedByte::Counter => {
                self.checksum = received;
                self.current.counter = received;
                self.expected = ExpectedByte::YavLsb;
                None
            }
            ExpectedByte::YavLsb => {
                self.handle_lsb(received, ExpectedByte::YavMsb, "IMU: no YavLsb");
                None
            }
            ExpectedByte::YavMsb => {
                self.current.yaw = self.process_msb(received);
                self.expected = ExpectedByte::PitchLsb;
                None
            }
            ExpectedByte::PitchLsb => {
                self.handle_lsb(received, ExpectedByte::PitchMsb, "IMU: no PitchLsb");
                None
            }
            ExpectedByte::PitchMsb => {
                self.current.pitch = self.process_msb(received);
                self.expected = ExpectedByte::RollLsb;
                None
            }
            ExpectedByte::RollLsb => {
                self.handle_lsb(received, ExpectedByte::RollMsb, "IMU: no RollLsb");
                None
            }
            ExpectedByte::RollMsb => {
                self.current.roll = self.process_msb(received);
                self.expected = ExpectedByte::SideLsb;
                None
            }
            ExpectedByte::SideLsb => {
                self.handle_lsb(received, ExpectedByte::SideMsb, "IMU: no SideLsb");
                None
            }
            ExpectedByte::SideMsb => {
                self.current.side = self.process_msb(received);
                self.expected = ExpectedByte::ForwardLsb;
                None
            }
            ExpectedByte::ForwardLsb => {
                self.handle_lsb(received, ExpectedByte::ForwardMsb, "IMU: no ForwardLsb");
                None
            }
            ExpectedByte::ForwardMsb => {
                self.current.forward = self.process_msb(received);
                self.expected = ExpectedByte::VerticalLsb;
                None
            }
            ExpectedByte::VerticalLsb => {
                self.handle_lsb(received, ExpectedByte::VerticalMsb, "IMU: no VerticalLsb");
                None
            }
            ExpectedByte::VerticalMsb => {
                self.current.vertical = self.process_msb(received);
                self.expected = ExpectedByte::Unused1;
                None
            }
            ExpectedByte::Unused1 => {
                if self.process_unused(received, 1) {
                    self.expected = ExpectedByte::Unused2;
                }
                None
            }
            ExpectedByte::Unused2 => {
                if self.process_unused(received, 2) {
                    self.expected = ExpectedByte::Unused3;
                }
                None
            }
            ExpectedByte::Unused3 => {
                if self.process_unused(received, 3) {
                    self.expected = ExpectedByte::Checksum;
                }
                None
            }
            ExpectedByte::Checksum => {
                self.expected = ExpectedByte::Header1;
                if self.checksum != received {
                    log::info!(
                        "IMU checksum failed: c {} r {} (counter {})",
                        self.checksum,
                        received,
                        self.current.counter
                    );
                }
                Some(self.current)
            }
        }
    }
}
