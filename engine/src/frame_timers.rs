use super::system::InfallibleSystem;
use idcontain::{IdSlab, Id};
use std::borrow::Cow;
use std::fmt::Write;
use std::mem;
use std::time::{Instant, Duration};

/// A handle for a frame timer, returned by `FrameTimers`.
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct FrameTimerId(Id<FrameTimer>);

/// Manages a set of frame timers which measure the elapsed time per frame of particular stages.
///
/// Timers are manipulated via `FrameTimerId`-s (obtained on creation) and are meant to be started
/// and stopped during a frame surrounding the different stages.
///
/// Periodically, a summary of all the timer averages is printed to the `info` log.
pub struct FrameTimers {
    timers: IdSlab<FrameTimer>,
    last_logged: Option<Instant>,
    log_buffer: String,

    total_timer: FrameTimerId,
    delta_time: f32,
}

impl FrameTimers {
    /// Returns the duration of the last frame in seconds.
    pub fn delta_time(&self) -> f32 {
        self.delta_time
    }

    /// Creates a new frame timer, returning its id.
    ///
    /// The `debug_name` is used when logging the periodic summary.
    pub fn new_stopped<S: Into<Cow<'static, str>>>(&mut self, debug_name: S) -> FrameTimerId {
        FrameTimerId(self.timers.insert(FrameTimer {
            debug_name: debug_name.into(),
            last_start: None,
            seconds_since_logged: 0.0,
            times_since_logged: 0.0,
        }))
    }

    /// Removes a timer, given its id.
    pub fn remove(&mut self, timer_id: FrameTimerId) {
        self.timers.remove(timer_id.0).expect("Invalid timer id.");
    }

    /// Starts a previously created frame timer.
    ///
    /// Starting an already started timer restarts it and returns the elapsed time since it was
    /// last started.
    pub fn start(&mut self, timer_id: FrameTimerId) -> Option<f32> {
        let &mut FrameTimer {
            ref mut last_start,
            ref mut seconds_since_logged,
            ref mut times_since_logged,
            ..
        } = &mut self.timers[timer_id.0];
        let current_time = Instant::now();
        mem::replace(last_start, Some(current_time)).map(|last_start| {
            let elapsed = duration_to_seconds(current_time.duration_since(last_start));
            *seconds_since_logged += elapsed;
            *times_since_logged += 1.0;
            elapsed
        })
    }

    /// Stops a previously created frame timer and returns the elapsed time in seconds.
    ///
    /// Stopping an already stopped timer is a no-op and will return `None` instead.
    pub fn stop(&mut self, timer_id: FrameTimerId) -> Option<f32> {
        let &mut FrameTimer {
            ref mut last_start,
            ref mut seconds_since_logged,
            ref mut times_since_logged,
            ..
        } = &mut self.timers[timer_id.0];
        mem::replace(last_start, None).map(|last_start| {
            let elapsed = duration_to_seconds(last_start.elapsed());
            *seconds_since_logged += elapsed;
            *times_since_logged += 1.0;
            elapsed
        })
    }

    /// Queries a frame timer and returns the elapsed time in seconds.
    ///
    /// Querying a stopped timer will return `None`.
    pub fn query(&self, timer_id: FrameTimerId) -> Option<f32> {
        self.timers[timer_id.0].last_start.map(|last_start| {
            duration_to_seconds(last_start.elapsed())
        })
    }

    fn maybe_log(&mut self) {
        let current_time = Instant::now();
        match self.last_logged.map(|last_logged| {
            current_time.duration_since(last_logged)
        }) {
            Some(duration) if duration.as_secs() >= 10 => {
                self.last_logged = Some(current_time);
            }
            None => {
                self.last_logged = Some(current_time);
                return;
            }
            Some(_) => return,
        };

        self.log_buffer.clear();
        for &mut FrameTimer {
            ref debug_name,
            ref mut seconds_since_logged,
            ref mut times_since_logged,
            ..
        } in &mut self.timers
        {
            let seconds_since_logged = mem::replace(seconds_since_logged, 0.0);
            let times_since_logged = mem::replace(times_since_logged, 0.0);
            let _ = write!(
                &mut self.log_buffer,
                "\n\t{}\t{:.2}/s (avg {:.2}ms)",
                debug_name,
                times_since_logged / seconds_since_logged,
                seconds_since_logged / times_since_logged * 1000.
            );
        }
        info!("Frame timer summary:{}", self.log_buffer);
    }
}

impl<'context> InfallibleSystem<'context> for FrameTimers {
    type Dependencies = ();

    fn debug_name() -> &'static str {
        "frame_timers"
    }

    fn create(_deps: ()) -> Self {
        let mut this = FrameTimers {
            timers: IdSlab::with_capacity(16),
            last_logged: None,
            log_buffer: String::with_capacity(512),

            total_timer: FrameTimerId(Id::invalid()),
            delta_time: 1.0 / 60.0,
        };
        let total_timer = this.new_stopped("frame");
        this.total_timer = total_timer;
        this
    }

    fn update(&mut self, _deps: ()) {
        let FrameTimers {
            total_timer,
            delta_time,
            ..
        } = *self;
        self.delta_time = self.start(total_timer).unwrap_or(delta_time);
        self.maybe_log();
    }
}

fn duration_to_seconds(duration: Duration) -> f32 {
    duration.as_secs() as f32 + (duration.subsec_nanos() as f32 * 1e-9f32)
}

struct FrameTimer {
    debug_name: Cow<'static, str>,
    last_start: Option<Instant>,

    seconds_since_logged: f32,
    times_since_logged: f32,
}
