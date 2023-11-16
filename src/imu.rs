use embassy_rp::{
    peripherals::{PIN_16, PIN_17, UART0},
    uart::{BufferedUart, Config},
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Instant};
use embedded_io_async::Read;
use static_cell::make_static;

pub static IMU_DATA: Signal<CriticalSectionRawMutex, ImuData> = Signal::new();

const BUF_SIZE: usize = 64;

pub async fn imu_task(uart0: UART0, pin_16: PIN_16, pin_17: PIN_17) {
    let tx_buf = &mut make_static!([0u8; BUF_SIZE])[..];
    let rx_buf = &mut make_static!([0u8; BUF_SIZE])[..];
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
    let mut stillness_detector = ImuStillnessDetector::new();

    let mut timestamp = Instant::now();
    loop {
        let mut buf = [0; 1];

        match embassy_time::with_timeout(Duration::from_secs(5), rx.read_exact(&mut buf)).await {
            Ok(result) => match result {
                Ok(_) => {
                    let now = Instant::now();
                    let dt = now - timestamp;
                    timestamp = now;

                    let received = buf[0];
                    if let Some(raw) = decoder.update(received) {
                        stillness_detector.process_data(&raw, now, dt);
                        // log::info!(
                        //     "IMU [R {} P {} Y {}] [F {} S {} V {}]",
                        //     raw.roll,
                        //     raw.pitch,
                        //     raw.yaw,
                        //     raw.forward,
                        //     raw.side,
                        //     raw.vertical
                        // );

                        data.update(&raw, now, dt, stillness_detector.last_stillness);
                        IMU_DATA.signal(data);
                    }
                }
                Err(err) => {
                    log::info!("IMU uart read error: {}", err);
                }
            },
            Err(_) => {
                log::error!("timeout reading IMU uart");
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

const STILLNESS_SIDE: i16 = 150;
const STILLNESS_FORWARD: i16 = 150;
const STILL_FOR: Duration = Duration::from_millis(50);
const STILLNESS_TIME_WINDOW: Duration = Duration::from_millis(500);
const STILLNESS_TIME_THRESHOLD: Duration = Duration::from_millis(350);

#[derive(Clone, Copy)]
pub struct ImuData {
    pub yaw: i16,
    pub pitch: i16,
    pub roll: i16,
    pub side: i16,
    pub forward: i16,
    pub vertical: i16,
    pub timestamp: Instant,
    pub dt: Duration,
    pub last_stillness: Option<Instant>,
}

impl ImuData {
    pub fn is_still(&self, now: Instant) -> bool {
        self.last_stillness
            .map(|since| now - since < STILL_FOR)
            .unwrap_or(false)
    }
}

#[derive(Clone, Copy)]
pub struct StillnessMemory {
    pub time_window: Duration,
    pub cumulative_stillness: Duration,
}

impl StillnessMemory {
    pub fn new() -> Self {
        Self {
            time_window: Duration::from_ticks(0),
            cumulative_stillness: Duration::from_ticks(0),
        }
    }

    pub fn ended(&self) -> bool {
        self.time_window >= STILLNESS_TIME_WINDOW
    }

    pub fn update(&mut self, dt: Duration, is_still: bool) {
        self.time_window += dt;
        if is_still {
            self.cumulative_stillness += dt;
        }
    }

    pub fn is_still(&self) -> bool {
        self.cumulative_stillness >= STILLNESS_TIME_THRESHOLD
    }
}

#[derive(Clone, Copy)]
pub struct ImuStillnessDetector {
    pub memory: Option<StillnessMemory>,
    pub last_stillness: Option<Instant>,
}

impl ImuStillnessDetector {
    pub fn new() -> Self {
        Self {
            memory: None,
            last_stillness: None,
        }
    }

    pub fn process_data(&mut self, raw: &Bno080RawRvcData, now: Instant, dt: Duration) {
        let is_still = raw.side.abs() < STILLNESS_SIDE || raw.forward.abs() < STILLNESS_FORWARD;
        match self.memory {
            Some(mut memory) => {
                memory.update(dt, is_still);
                self.memory = if memory.ended() {
                    if memory.is_still() {
                        self.last_stillness = Some(now);
                    }
                    None
                } else {
                    Some(memory)
                };
            }
            None => {
                if is_still {
                    self.memory = Some(StillnessMemory::new());
                }
            }
        }
    }
}

const DT_MIN: Duration = Duration::from_micros(100);

impl ImuData {
    pub fn init() -> Self {
        Self {
            yaw: 0,
            pitch: 0,
            roll: 0,
            side: 0,
            forward: 0,
            vertical: 0,
            timestamp: Instant::now(),
            dt: DT_MIN,
            last_stillness: None,
        }
    }

    pub fn update(
        &mut self,
        data: &Bno080RawRvcData,
        now: Instant,
        dt: Duration,
        last_stillness: Option<Instant>,
    ) {
        let dt = if dt.as_micros() == 0 { DT_MIN } else { dt };

        self.yaw = data.yaw;
        self.pitch = data.pitch;
        self.roll = data.roll;
        self.side = data.side;
        self.forward = data.forward;
        self.vertical = data.vertical;

        self.last_stillness = last_stillness;

        self.timestamp = now;
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
