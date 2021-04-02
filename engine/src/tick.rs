use super::context::ControlFlow;
use super::system::InfallibleSystem;
use crate::internal_derive::DependenciesFrom;
use std::time::{Duration, Instant};

pub struct Config {
    pub timestep: f32,
}

#[derive(DependenciesFrom)]
pub struct Dependencies<'context> {
    config: &'context Config,
    _control_flow: &'context mut ControlFlow,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct TickIndex(pub u64);

impl Tick {
    #[inline]
    pub fn is_frame(&self) -> bool {
        self.is_frame
    }

    #[inline]
    pub fn timestep(&self) -> f32 {
        self.timestep
    }

    #[inline]
    pub fn index(&self) -> TickIndex {
        self.index
    }

    #[inline]
    pub fn drift(&self) -> f32 {
        self.drift
    }

    #[inline]
    pub fn slept(&self) -> f32 {
        self.slept
    }

    #[inline]
    pub fn seconds_since_tick(&self, index: TickIndex) -> f32 {
        if index.0 < self.index.0 {
            (self.index.0 - index.0) as f32 * self.timestep
        } else {
            (index.0 - self.index.0) as f32 * (-self.timestep)
        }
    }
}

pub struct Tick {
    timestep: f32,
    index: TickIndex,

    drift: f32,
    slept: f32,
    last_time: Option<Instant>,
    is_frame: bool,
}

impl<'context> InfallibleSystem<'context> for Tick {
    type Dependencies = Dependencies<'context>;

    fn debug_name() -> &'static str {
        "tick"
    }

    fn create(deps: Self::Dependencies) -> Self {
        Tick {
            timestep: deps.config.timestep,
            index: TickIndex(0),

            drift: 0.0,
            slept: 0.0,
            last_time: None,
            is_frame: true,
        }
    }

    fn update(&mut self, deps: Self::Dependencies) {
        let current_time = Instant::now();
        let last_time = if let Some(instant) = self.last_time {
            instant
        } else {
            self.last_time = Some(current_time);
            return;
        };

        // Accumulate drift: real_time - simulation_time
        let real_delta = duration_to_seconds(current_time.duration_since(last_time));
        self.drift += real_delta - self.timestep;

        // If we just renderered a frame, but simulation is still ahead of real time by more than
        // one timestep, sleep to get back in sync.
        self.slept = 0.0;
        self.is_frame = self.drift < self.timestep;

        if self.drift < self.timestep {
            let sleep_seconds = (self.timestep - self.drift).max(0.0);
            let sleep_duration = duration_from_seconds(sleep_seconds);
            let sleep_until = current_time + sleep_duration;

            // Sleep for the entire time minus one millisecond.
            deps._control_flow.sleep_until = Some(sleep_until);

            self.slept = sleep_seconds;
        }
        self.last_time = Some(current_time);

        // Update the deterministic tick index.
        self.index.0 += 1;
    }
}

fn duration_to_seconds(duration: Duration) -> f32 {
    duration.as_secs() as f32 + (duration.subsec_nanos() as f32 * 1e-9f32)
}

fn duration_from_seconds(seconds: f32) -> Duration {
    Duration::new(seconds as u64, ((seconds - seconds.floor()) * 1e9) as u32)
}
