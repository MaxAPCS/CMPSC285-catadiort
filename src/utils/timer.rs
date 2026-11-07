//! timer.rs: a stopwatch for the TIME line and the budget for a timed run
//! (--time-limit, --progress). Complete; nothing to change; the twin of the
//! C++ kit's timer.h. rtWorldEnd starts a Timer at its entry, reads it when
//! the acceleration structure is ready (build), starts another around the
//! pixel loop (render), and reads the first again after the image is
//! written (total).

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

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

/// A timed run, --time-limit S. Make one from the option's seconds as the
/// pixel loop begins; hand the tiles out in the order tile_order gives
/// (shuffled under a limit, the same every run, so what gets done is a
/// fair sample of the frame); ask expired() before each tile and before
/// each row of one (brute force takes seconds per row); add what each
/// worker finished with done(n); and after the TIME line call report(),
/// which prints
///
///     LIMIT <scene> seconds=<S> pixels=<done>/<total> samples=<done x spp> estimated_render=<s>
///
/// the full frame's render time scaled from what was done. With no limit
/// (0 seconds) expired() is never true, the order is 0, 1, 2, ... and
/// report() prints nothing, so the image is as it always was.
///
/// It also shows a long render is alive (--progress): call progress(total
/// pixels) before the loop and finish() after it, and done() prints a line
/// on stderr every two seconds with the pixels done, the time and about
/// how long is left; without --progress nothing is printed.
pub struct RenderBudget {
    seconds: f64,
    clock: Timer,
    done: AtomicU64,
    total: u64,
    next_tick: AtomicU64, /* the next progress line's time, as f64 bits */
    ticked: AtomicBool,
}

impl RenderBudget {
    pub fn new(seconds: f64) -> RenderBudget {
        RenderBudget {
            seconds,
            clock: Timer::new(),
            done: AtomicU64::new(0),
            total: 0,
            next_tick: AtomicU64::new(0f64.to_bits()),
            ticked: AtomicBool::new(false),
        }
    }

    pub fn active(&self) -> bool {
        self.seconds > 0.0
    }

    pub fn expired(&self) -> bool {
        self.seconds > 0.0 && self.clock.seconds() >= self.seconds
    }

    pub fn pixels_done(&self) -> u64 {
        self.done.load(Ordering::Relaxed)
    }

    /// What a worker finished; prints the progress line when one is due.
    pub fn done(&self, pixels: u64) {
        let n = self.done.fetch_add(pixels, Ordering::Relaxed) + pixels;
        if self.total > 0
            && self.clock.seconds() >= f64::from_bits(self.next_tick.load(Ordering::Relaxed))
        {
            self.tick(n);
        }
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
    pub fn tile_order(&self, tiles: u64) -> Vec<u64> {
        let mut order: Vec<_> = (0..tiles).collect();
        if self.seconds > 0.0 {
            /* a small deterministic generator, so the sample is the same every run */
            let mut s: u64 = 0x5eed;
            for i in (1..tiles).rev() {
                s = s
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let j = (s >> 33) % (i + 1);
                order.swap(i as _, j as _);
            }
        }
        order
    }

    pub fn report(&self, scene: &str, render_seconds: f64, total_pixels: u64, spp: u32) {
        if !(self.seconds > 0.0) {
            return;
        }
        let pixels = self.done.load(Ordering::Relaxed);
        let estimate = if pixels > 0 {
            render_seconds * total_pixels as f64 / pixels as f64
        } else {
            0.0
        };
        println!(
            "LIMIT {} seconds={:.1} pixels={}/{} samples={} estimated_render={:.1}",
            scene,
            self.seconds,
            pixels,
            total_pixels,
            pixels * spp as u64,
            estimate
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
