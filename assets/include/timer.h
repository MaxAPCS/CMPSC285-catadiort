/*
 * timer.h: a stopwatch for the TIME line and the budget for a timed run
 * (HW2 handout: timing). Complete; nothing to
 * change. rtWorldEnd starts a Timer at its entry, reads it when the
 * acceleration structure is ready (build), starts another around the
 * pixel loop (render), and reads the first again after the image is
 * written (total).
 */
#ifndef TIMER_H
#define TIMER_H

#include <algorithm>
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstdio>
#include <mutex>
#include <numeric>
#include <random>
#include <thread>
#include <vector>

class Timer {
public:
    Timer() : start_(std::chrono::steady_clock::now()) {}

    /* Start over from now. */
    void reset() { start_ = std::chrono::steady_clock::now(); }

    /* Seconds since construction or the last reset. */
    double seconds() const
    {
        return std::chrono::duration<double>(std::chrono::steady_clock::now() - start_).count();
    }

private:
    std::chrono::steady_clock::time_point start_;
};

/* A timed run, --time-limit S (HW2 handout: timing; HW3 Suggested
 * Workflow, step 3). Make one from the option's seconds as the pixel loop
 * begins: under a limit a watchdog thread raises a flag after S seconds,
 * and the render stops where it is, even inside a pixel.
 * - Tiles of tileSize(16) pixels (1 under a limit, so what gets done is
 *   pixels scattered over the frame, a fair sample), handed out in the
 *   order tileOrder gives (a fixed shuffle under a limit).
 * - The nearest-hit and any-hit searches return "no hit" once timeUp(),
 *   a flag read cheap enough for every ray, so a pixel in progress ends
 *   within one ray even under brute force.
 * - The worker asks expired() before each tile and after each sample;
 *   once it is true, it calls cut(n) with n the samples of this pixel
 *   finished before (the one in progress went wrong when the searches
 *   stopped) and stops. done(1) after each finished pixel.
 * After the TIME line, report() prints
 *
 *     LIMIT <scene> seconds=<S> pixels=<done>/<total> samples=<done> estimated_render=<s>
 *
 * the full frame's render time scaled from the samples done, the sample
 * each stopped worker was in counted as half done (on average it was).
 * With no limit (0 seconds) expired() is never true, the tiles are 16 x 16
 * in the order 0, 1, 2, ... and report() prints nothing, so the image is
 * as it always was. A renderer whose searches never ask timeUp(), with
 * expired() asked only between pixels (or rows), still gets a fair
 * estimate: each worker finishes the pixel (or row) it is in, so its run
 * ends that much later.
 *
 * It also shows a long render is alive (--progress): call progress(total
 * pixels) before the loop and finish() after it, and done() prints a line
 * on stderr every two seconds with the pixels done, the time and about
 * how long is left; without --progress nothing is printed. */
class RenderBudget {
public:
    explicit RenderBudget(double seconds)
        : seconds_(seconds), done_(0), cutSamples_(0), cuts_(0), total_(0), nextTick_(0), ticked_(false), quit_(false)
    {
        stop_.store(false);
        if (seconds_ > 0) {
            watchdog_ = std::thread([this]() {
                std::unique_lock<std::mutex> lock(mutex_);
                const auto limit = std::chrono::microseconds((long long)(seconds_ * 1e6));
                if (!wake_.wait_for(lock, limit, [this]() { return quit_; })) {
                    stop_.store(true);
                }
            });
        }
    }

    ~RenderBudget()
    {
        if (watchdog_.joinable()) {
            {
                std::lock_guard<std::mutex> lock(mutex_);
                quit_ = true;
            }
            wake_.notify_one();
            watchdog_.join();
        }
        stop_.store(false);
    }

    RenderBudget(const RenderBudget &) = delete;
    RenderBudget &operator=(const RenderBudget &) = delete;

    /* Whether the time is up: for the nearest-hit and any-hit searches,
     * which return "no hit" once it is, so they need no budget at hand. */
    static bool timeUp() { return stop_.load(std::memory_order_relaxed); }

    bool active() const { return seconds_ > 0; }
    bool expired() const { return timeUp(); }
    long long pixelsDone() const { return done_.load(); }

    /* The tile's side: `size`, or 1 under a limit. */
    int tileSize(int size) const { return seconds_ > 0 ? 1 : size; }

    /* Pixels a worker finished, every sample of them; prints the progress
     * line when one is due. */
    void done(long long pixels)
    {
        const long long n = done_ += pixels;
        if (total_ > 0 && clock_.seconds() >= nextTick_.load()) {
            tick(n);
        }
    }

    /* A worker stopped inside a pixel: `samples` of it had finished. */
    void cut(long long samples)
    {
        cutSamples_ += samples;
        cuts_++;
    }

    /* --progress: report on stderr while the pixel loop runs. */
    void progress(long long totalPixels)
    {
        total_ = totalPixels;
        nextTick_ = 2.0;
    }

    /* After the loop: the final line, and the newline that ends it. */
    void finish()
    {
        if (total_ > 0 && ticked_) {
            nextTick_ = 0;
            tick(done_.load());
            std::fprintf(stderr, "\n");
        }
    }

    /* The order to hand out `tiles` tiles in. */
    std::vector<int> tileOrder(int tiles) const
    {
        std::vector<int> order((size_t)std::max(0, tiles));
        std::iota(order.begin(), order.end(), 0);
        if (seconds_ > 0) {
            std::shuffle(order.begin(), order.end(), std::mt19937(0x5eed));
        }
        return order;
    }

    /* The full frame is totalPixels * spp samples; done, the finished
     * pixels' and the cut ones' finished samples, plus half a sample per
     * cut for the one each stopped worker was in. */
    void report(const char *scene, double renderSeconds, long long totalPixels, int spp) const
    {
        if (!(seconds_ > 0)) {
            return;
        }
        const long long pixels = done_.load();
        const long long samples = pixels * (long long)spp + cutSamples_.load();
        const double credit = (double)samples + 0.5 * (double)cuts_.load();
        std::printf("LIMIT %s seconds=%.1f pixels=%lld/%lld samples=%lld estimated_render=%.1f\n", scene, seconds_,
                    pixels, totalPixels, samples,
                    credit > 0 ? renderSeconds * (double)totalPixels * (double)spp / credit : 0.0);
    }

private:
    /* One worker prints the line; the others find the next tick moved on. */
    void tick(long long n)
    {
        double due = nextTick_.load();
        const double now = clock_.seconds();
        if (now < due || !nextTick_.compare_exchange_strong(due, now + 2.0)) {
            return;
        }
        const double left = n > 0 ? now * (double)(total_ - n) / (double)n : 0.0;
        std::fprintf(stderr, "\r  render: %3d%%  %lld/%lld pixels  %.0f s, about %.0f s left   ",
                     (int)(100 * n / std::max(1LL, total_)), n, total_, now, left);
        std::fflush(stderr);
        ticked_ = true;
    }

    double seconds_;
    Timer clock_;
    std::atomic<long long> done_;       /* pixels finished */
    std::atomic<long long> cutSamples_; /* samples finished in the pixels cut short */
    std::atomic<long long> cuts_;       /* pixels cut short: one per worker stopped inside a pixel */
    long long total_;                   /* pixels in the frame, when --progress is on; else 0 */
    std::atomic<double> nextTick_;      /* when the next progress line is due, in clock_ seconds */
    std::atomic<bool> ticked_;

    /* The watchdog: raises stop_ after seconds_, unless the budget ends first. */
    std::thread watchdog_;
    std::mutex mutex_;
    std::condition_variable wake_;
    bool quit_;
    static inline std::atomic<bool> stop_{false}; /* one render at a time, so one flag for the program */
};

#endif /* TIMER_H */
