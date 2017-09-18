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
///
/// Example
/// ---
///
/// ```
/// # fn prepare_frame() {}
/// # fn do_stage_1() {}
/// # fn prepare_stage2() {}
/// # fn do_stage_2a() {}
/// # fn do_stage_2b() {}
/// # fn end_frame() {}
/// # use engine::FrameTimers;
/// let mut timers = FrameTimers::new();
///
/// let total_timer = timers.new_stopped("frame");
/// let stage1_timer = timers.new_stopped("stage_1");
/// let stage2_timer = timers.new_stopped("stage_2");
/// let stage2a_timer = timers.new_stopped("stage_2a");
/// let stage2b_timer = timers.new_stopped("stage_2b");
///
/// for _ in 0..100 {
///     timers.start(total_timer);  // This timer is never explicitly stopped, so the call to start
///                                 // atomically measures the elapsed time and resets it.
///     prepare_frame();
///
///     // Measure stage1's time.
///     timers.start(stage1_timer);
///     do_stage_1();
///     timers.stop(stage1_timer);
///
///     // Measure stage2's time.
///     timers.start(stage2_timer);
///     prepare_stage2();
///
///     // Calls to start can be 'nested' or 'interleaved', timers are fully independent from each
///     // other.
///     timers.start(stage2a_timer);
///     do_stage_2a();
///     timers.stop(stage2a_timer);
///
///     timers.start(stage2b_timer);
///     do_stage_2b();
///     timers.stop(stage2b_timer);
///
///     timers.stop(stage2_timer);
///     end_frame();
/// }
/// ```
pub struct FrameTimers {
    timers: IdSlab<FrameTimer>,
    last_logged: Option<Instant>,
    log_buffer: String,
}

impl FrameTimers {
    /// Creates a new `FrameTimers`.
    pub fn new() -> Self {
        FrameTimers {
            timers: IdSlab::with_capacity(16),
            last_logged: None,
            log_buffer: String::with_capacity(512),
        }
    }

    /// Creates a new frame timer, returning its id.
    ///
    /// The `debug_name` is used when logging the periodic summary.
    ///
    /// Panics
    /// ---
    /// If too many timers are created (a very large amount, more likely pointing to an infinite
    /// loop creating them.
    pub fn new_stopped<S: Into<Cow<'static, str>>>(&mut self, debug_name: S) -> FrameTimerId {
        FrameTimerId(self.timers.insert(FrameTimer {
            debug_name: debug_name.into(),
            last_start: None,
            seconds_since_logged: 0.0,
            times_since_logged: 0.0,
        }))
    }

    // // Removes a timer, given its id.
    // pub fn remove(&mut self, timer_id: FrameTimerId) {
    //    self.timers.remove(timer_id.0).expect("Invalid timer id.");
    // }

    /// Starts a previously created frame timer.
    ///
    /// Starting an already started timer restarts it and returns the elapsed time since it was
    /// last started.
    pub fn start(&mut self, timer_id: FrameTimerId) -> Option<f32> {
        let time = {
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
        };
        self.maybe_log();
        time
    }

    /// Stops a previously created frame timer and returns the elapsed time in seconds.
    ///
    /// Stopping an already stopped timer is a no-op and will return `None` instead.
    pub fn stop(&mut self, timer_id: FrameTimerId) -> Option<f32> {
        let time = {
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
        };
        self.maybe_log();
        time
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

fn duration_to_seconds(duration: Duration) -> f32 {
    duration.as_secs() as f32 + (duration.subsec_nanos() as f32 * 1e-9f32)
}

struct FrameTimer {
    debug_name: Cow<'static, str>,
    last_start: Option<Instant>,

    seconds_since_logged: f32,
    times_since_logged: f32,
}
