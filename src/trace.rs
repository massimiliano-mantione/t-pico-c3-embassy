use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Duration;
use static_cell::make_static;

use crate::{
    imu::ImuData,
    race::{Angle, BackSteering, RouteTarget},
};

pub static TRACE: Signal<CriticalSectionRawMutex, TraceCommand> = Signal::new();

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TraceCommand {
    Clear,
    Print,
    Push(TraceEvent),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TraceEventKind {}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TraceEvent {
    pub absolute_heading: Angle,
    pub track_heading: Angle,
    pub steer: Angle,
    pub speed: i16,
    pub roll: Angle,
    pub pitch: Angle,
    pub yaw: Angle,
    pub forward: i16,
    pub side: i16,
    pub vertical: i16,
    pub has_back_panic: bool,
    pub remaining_back_panic_ms: u32,
    pub has_target: bool,
    pub remaining_target_ms: u32,
    pub target: Angle,
    pub target_back: bool,
    pub stillness_ms: u32,
    pub dt_us: u32,
}

impl TraceEvent {
    pub fn new(
        absolute_heading: Angle,
        track_heading: Angle,
        imu_data: &ImuData,
        has_back_panic: bool,
        remaining_back_panic: &Option<BackSteering>,
        target: &Option<RouteTarget>,
        steer: Angle,
        speed: i16,
        dt: Duration,
    ) -> Self {
        Self {
            absolute_heading,
            track_heading,
            steer,
            speed,
            roll: Angle::from_imu_value(imu_data.roll),
            pitch: Angle::from_imu_value(imu_data.pitch),
            yaw: Angle::from_imu_value(imu_data.yaw),
            forward: imu_data.forward,
            side: imu_data.side,
            vertical: imu_data.vertical,
            has_back_panic,
            remaining_back_panic_ms: remaining_back_panic
                .map(|bs| bs.remaining_time.as_millis() as u32)
                .unwrap_or(0),
            has_target: target.is_some(),
            remaining_target_ms: target
                .map(|t| t.remaining_time.as_millis() as u32)
                .unwrap_or(0),
            target: target.map(|t| t.target).unwrap_or(Angle::ZERO),
            target_back: target.map(|t| t.go_back).unwrap_or(false),
            stillness_ms: imu_data
                .stillness
                .map(|s| s.as_millis() as u32)
                .unwrap_or(0),
            dt_us: dt.as_micros() as u32,
        }
    }

    pub fn print(&self, index: usize, elapsed: Duration) {
        log::info!(
            "{} ({}ms): DT {}us [AB {} TR {}] [ST {} SP {}] [RPY {} {} {}] [FSV {} {} {}] [BP {} {}ms] [TGT {} {} {} {}ms] [STILL {}ms]",
            index,
            elapsed.as_millis(),
            self.dt_us,
            self.absolute_heading.value(),
            self.track_heading.value(),
            self.steer.value(),
            self.speed,
            self.roll.value(),
            self.pitch.value(),
            self.yaw.value(),
            self.forward,
            self.side,
            self.vertical,
            if self.has_back_panic { "T" } else { "F" },
            self.remaining_back_panic_ms,
            if self.has_target { "T" } else { "F" },
            if self.target_back { "BK" } else { "FW" },
            self.target.value(),
            self.remaining_target_ms,
            self.stillness_ms,
        );
    }
}

const EMPTY_EVENT: TraceEvent = TraceEvent {
    absolute_heading: Angle::ZERO,
    track_heading: Angle::ZERO,
    steer: Angle::ZERO,
    speed: 0,
    roll: Angle::ZERO,
    pitch: Angle::ZERO,
    yaw: Angle::ZERO,
    forward: 0,
    side: 0,
    vertical: 0,
    has_back_panic: false,
    remaining_back_panic_ms: 0,
    has_target: false,
    remaining_target_ms: 0,
    target: Angle::ZERO,
    target_back: false,
    stillness_ms: 0,
    dt_us: 0,
};

const TRACE_EVENTS: usize = 3000;

struct TraceData {
    pub start: usize,
    pub end: usize,
    pub events: [TraceEvent; TRACE_EVENTS],
}

impl TraceData {
    pub fn clear(&mut self) {
        self.start = 0;
        self.end = 0;
    }

    pub fn push(&mut self, event: TraceEvent) {
        let new_end = (self.end + 1) % TRACE_EVENTS;
        if self.start == new_end {
            self.start = (self.start + 1) % TRACE_EVENTS;
        }
        self.events[self.end] = event;
        self.end = new_end;
    }

    pub async fn print(&self) {
        log::info!("printing trace: [{} {}]", self.start, self.end);
        let mut current = self.start;
        let mut elapsed = Duration::from_millis(0);
        loop {
            self.events[current].print(current, elapsed);
            embassy_time::Timer::after(Duration::from_millis(16)).await;
            elapsed += Duration::from_micros(self.events[current].dt_us as u64);
            current = (current + 1) % TRACE_EVENTS;
            if current == self.end {
                break;
            }
        }
    }
}

pub async fn trace_task() {
    let data: &'static mut TraceData = make_static!(TraceData {
        start: 0,
        end: 0,
        events: [EMPTY_EVENT; TRACE_EVENTS],
    });

    loop {
        match TRACE.wait().await {
            TraceCommand::Clear => {
                log::info!("clearing trace");
                data.clear();
            }
            TraceCommand::Print => data.print().await,
            TraceCommand::Push(event) => data.push(event),
        }
    }
}
