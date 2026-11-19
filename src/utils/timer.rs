//! timer.rs: a stopwatch for the TIME line and the budget for a timed run
//! (--time-limit, --progress). Complete; nothing to change; the twin of the
//! C++ kit's timer.h. rtWorldEnd starts a Timer at its entry, reads it when
//! the acceleration structure is ready (build), starts another around the
//! pixel loop (render), and reads the first again after the image is
//! written (total).

#![allow(dead_code)] // given complete: your renderer uses the rest as it grows

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Whether the time is up: one render at a time, so one flag for the program.
static TIME_UP: AtomicBool = AtomicBool::new(false);

pub struct Timer {
    start: Instant,
}

impl Timer {
    pub fn new() -> Timer {
        Timer {
            start: Instant::now(),
        }
    }
    /// Start over from now.
    pub fn reset(&mut self) {
        self.start = Instant::now();
    }
    /// Seconds since construction or the last reset.
    pub fn seconds(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }
}

impl Default for Timer {
    fn default() -> Self {
        Timer::new()
    }
}

/// A timed run, --time-limit S (HW2 handout: timing; HW3 Suggested
/// Workflow, step 3). Make one from the option's seconds as the pixel loop
/// begins: under a limit a watchdog thread raises a flag after S seconds,
/// and the render stops where it is, even inside a pixel.
/// - Tiles of tile_size(16) pixels (1 under a limit, so what gets done is
///   pixels scattered over the frame, a fair sample), handed out in the
///   order tile_order gives (a fixed shuffle under a limit).
/// - The nearest-hit and any-hit searches return "no hit" once
///   RenderBudget::time_up(), a flag read cheap enough for every ray, so a
///   pixel in progress ends within one ray even under brute force.
/// - The worker asks expired() before each tile and after each sample;
///   once it is true, it calls cut(n) with n the samples of this pixel
///   finished before (the one in progress went wrong when the searches
///   stopped) and stops. done(1) after each finished pixel.
/// After the TIME line, report() prints
///
///     LIMIT <scene> seconds=<S> pixels=<done>/<total> samples=<done> estimated_render=<s>
///
/// the full frame's render time scaled from the samples done, the sample
/// each stopped worker was in counted as half done (on average it was).
/// With no limit (0 seconds) expired() is never true, the tiles are 16 x 16
/// in the order 0, 1, 2, ... and report() prints nothing, so the image is
/// as it always was. A renderer whose searches never ask time_up(), with
/// expired() asked only between pixels (or rows), still gets a fair
/// estimate: each worker finishes the pixel (or row) it is in, so its run
/// ends that much later.
///
/// It also shows a long render is alive (--progress): call progress(total
/// pixels) before the loop and finish() after it, and done() prints a line
/// on stderr every two seconds with the pixels done, the time and about
/// how long is left; without --progress nothing is printed.
pub struct RenderBudget {
    seconds: f64,
    clock: Timer,
    done: AtomicU64,        /* pixels finished */
    cut_samples: AtomicU64, /* samples finished in the pixels cut short */
    cuts: AtomicI64,        /* pixels cut short: one per worker stopped inside a pixel */
    total: u64,
    next_tick: AtomicU64, /* the next progress line's time, as f64 bits */
    ticked: AtomicBool,
    /* the watchdog: raises TIME_UP after `seconds`, unless the budget is dropped first */
    watchdog: Option<(Arc<(Mutex<bool>, Condvar)>, JoinHandle<()>)>,
}

impl RenderBudget {
    pub fn new(seconds: f64) -> RenderBudget {
        TIME_UP.store(false, Ordering::Relaxed);
        let watchdog = if seconds > 0.0 {
            let quit = Arc::new((Mutex::new(false), Condvar::new()));
            let theirs = Arc::clone(&quit);
            let limit = Duration::from_secs_f64(seconds);
            let handle = std::thread::spawn(move || {
                let (lock, wake) = &*theirs;
                let guard = lock.lock().unwrap();
                let (_guard, result) = wake
                    .wait_timeout_while(guard, limit, |quit| !*quit)
                    .unwrap();
                if result.timed_out() {
                    TIME_UP.store(true, Ordering::Relaxed);
                }
            });
            Some((quit, handle))
        } else {
            None
        };
        RenderBudget {
            seconds,
            clock: Timer::new(),
            done: AtomicU64::new(0),
            cut_samples: AtomicU64::new(0),
            cuts: AtomicI64::new(0),
            total: 0,
            next_tick: AtomicU64::new(0f64.to_bits()),
            ticked: AtomicBool::new(false),
            watchdog,
        }
    }

    /// Whether the time is up: for the nearest-hit and any-hit searches,
    /// which return "no hit" once it is, so they need no budget at hand.
    pub fn time_up() -> bool {
        TIME_UP.load(Ordering::Relaxed)
    }

    pub fn active(&self) -> bool {
        self.seconds > 0.0
    }

    pub fn expired(&self) -> bool {
        RenderBudget::time_up()
    }

    pub fn pixels_done(&self) -> u64 {
        self.done.load(Ordering::Relaxed)
    }

    /// The tile's side: `size`, or 1 under a limit.
    pub fn tile_size(&self, size: usize) -> usize {
        if self.seconds > 0.0 { 1 } else { size }
    }

    /// Pixels a worker finished, every sample of them; prints the progress
    /// line when one is due.
    pub fn done(&self, pixels: u64) {
        let n = self.done.fetch_add(pixels, Ordering::Relaxed) + pixels;
        if self.total > 0
            && self.clock.seconds() >= f64::from_bits(self.next_tick.load(Ordering::Relaxed))
        {
            self.tick(n);
        }
    }

    /// A worker stopped inside a pixel: `samples` of it had finished.
    pub fn cut(&self, samples: u64) {
        self.cut_samples.fetch_add(samples, Ordering::Relaxed);
        self.cuts.fetch_add(1, Ordering::Relaxed);
    }

    /// --progress: report on stderr while the pixel loop runs.
    pub fn progress(&mut self, total_pixels: u64) {
        self.total = total_pixels;
        self.next_tick.store(2.0f64.to_bits(), Ordering::Relaxed);
    }

    /// After the loop: the final line, and the newline that ends it.
    pub fn finish(&self) {
        if self.total > 0 && self.ticked.load(Ordering::Relaxed) {
            self.next_tick.store(0f64.to_bits(), Ordering::Relaxed);
            self.tick(self.done.load(Ordering::Relaxed));
            eprintln!();
        }
    }

    /// The order to hand out `tiles` tiles in: 0, 1, 2 ... without a limit,
    /// a fixed shuffle (the same every run) under one.
    pub fn tile_order(&self, tiles: usize) -> Vec<usize> {
        let mut order: Vec<usize> = (0..tiles).collect();
        if self.seconds > 0.0 {
            /* a small deterministic generator, so the sample is the same every run */
            let mut s: u64 = 0x5eed;
            for i in (1..tiles).rev() {
                s = s
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let j = ((s >> 33) as usize) % (i + 1);
                order.swap(i, j);
            }
        }
        order
    }

    /// The full frame is total_pixels * spp samples; done, the finished
    /// pixels' and the cut ones' finished samples, plus half a sample per
    /// cut for the one each stopped worker was in.
    pub fn report(&self, scene: &str, render_time: f64, total_pixels: u64, spp: i32) {
        if !(self.seconds > 0.0) {
            return;
        }
        let pixels = self.done.load(Ordering::Relaxed);
        let samples = pixels * spp as u64 + self.cut_samples.load(Ordering::Relaxed);
        let credit = samples as f64 + 0.5 * self.cuts.load(Ordering::Relaxed) as f64;
        let estimate = if credit > 0.0 {
            render_time * total_pixels as f64 * spp as f64 / credit
        } else {
            0.0
        };
        println!(
            "LIMIT {} seconds={:.1} pixels={}/{} samples={} estimated_render={:.1}",
            scene, self.seconds, pixels, total_pixels, samples, estimate
        );
    }

    /// One worker prints the line; the others find the next tick moved on.
    fn tick(&self, n: u64) {
        let due = self.next_tick.load(Ordering::Relaxed);
        let now = self.clock.seconds();
        if now < f64::from_bits(due) {
            return;
        }
        if self
            .next_tick
            .compare_exchange(
                due,
                (now + 2.0).to_bits(),
                Ordering::Relaxed,
                Ordering::Relaxed,
            )
            .is_err()
        {
            return;
        }
        let left = if n > 0 {
            now * (self.total - n) as f64 / n as f64
        } else {
            0.0
        };
        eprint!(
            "\r  render: {:3}%  {}/{} pixels  {:.0} s, about {:.0} s left   ",
            100 * n / self.total.max(1),
            n,
            self.total,
            now,
            left
        );
        self.ticked.store(true, Ordering::Relaxed);
    }
}

impl Drop for RenderBudget {
    fn drop(&mut self) {
        if let Some((quit, handle)) = self.watchdog.take() {
            let (lock, wake) = &*quit;
            *lock.lock().unwrap() = true;
            wake.notify_one();
            let _ = handle.join();
        }
        TIME_UP.store(false, Ordering::Relaxed);
    }
}
