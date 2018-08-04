use super::system::InfallibleSystem;
use std::thread;
use std::time::{Duration, Instant};

pub struct Config {
    pub timestep: f32,
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
    type Dependencies = &'context Config;

    fn debug_name() -> &'static str {
        "tick"
    }

    fn create(config: &Config) -> Self {
        Tick {
            timestep: config.timestep,
            index: TickIndex(0),

            drift: 0.0,
            slept: 0.0,
            last_time: None,
            is_frame: true,
        }
    }

    fn update(&mut self, _: &Config) {
        let mut current_time = Instant::now();
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
        if self.drift < -self.timestep {
            let sleep_duration = duration_from_seconds(-self.drift - self.timestep + 1e-3);
            let sleep_until = current_time + sleep_duration;

            // Sleep for the entire time minus one millisecond.
            thread::sleep(sleep_duration - Duration::from_millis(1));

            // Busy wait remaining time (approx 1ms).
            let new_current_time = loop {
                let new_current_time = Instant::now();
                if new_current_time >= sleep_until {
                    break new_current_time;
                }
                thread::yield_now();
            };

            // Update drift with newly waited time.
            self.slept = duration_to_seconds(new_current_time.duration_since(current_time));
            self.drift += self.slept;

            // Report the amount slept.
            current_time = new_current_time;
        } else {
            self.slept = 0.0;
        }
        self.last_time = Some(current_time);

        // Render a frame this tick iff the drift is less than one timestep.
        self.is_frame = self.drift <= self.timestep;

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
