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
#include <cstdio>
#include <numeric>
#include <random>
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

/* A timed run, --time-limit S (HW2 handout: timing). Make
 * one from the option's seconds as the pixel loop begins; hand the
 * tiles out in the order tileOrder gives (shuffled under a limit, the
 * same every run, so what gets done is a fair sample of the frame); ask
 * expired() before each tile and before each row of one (brute force
 * takes seconds per row); add what each worker finished with done(n);
 * and after the TIME line call report(), which prints
 *
 *     LIMIT <scene> seconds=<S> pixels=<done>/<total> samples=<done x spp> estimated_render=<s>
 *
 * the full frame's render time scaled from what was done. With no limit
 * (0 seconds) expired() is never true, the order is 0, 1, 2, ... and
 * report() prints nothing, so the image is as it always was.
 *
 * It also shows a long render is alive (--progress): call progress(total
 * pixels) before the loop and finish() after it, and done() prints a line
 * on stderr every two seconds with the pixels done, the time and about
 * how long is left; without --progress nothing is printed. */
class RenderBudget {
public:
    explicit RenderBudget(double seconds) : seconds_(seconds), done_(0), total_(0), nextTick_(0), ticked_(false) {}

    bool active() const { return seconds_ > 0; }
    bool expired() const { return seconds_ > 0 && clock_.seconds() >= seconds_; }
    long long pixelsDone() const { return done_.load(); }

    /* What a worker finished; prints the progress line when one is due. */
    void done(long long pixels)
    {
        const long long n = done_ += pixels;
        if (total_ > 0 && clock_.seconds() >= nextTick_.load())
            tick(n);
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
        if (seconds_ > 0)
            std::shuffle(order.begin(), order.end(), std::mt19937(0x5eed));
        return order;
    }

    void report(const char *scene, double renderSeconds, long long totalPixels, int spp) const
    {
        if (!(seconds_ > 0))
            return;
        const long long pixels = done_.load();
        std::printf("LIMIT %s seconds=%.1f pixels=%lld/%lld samples=%lld estimated_render=%.1f\n", scene, seconds_,
                    pixels, totalPixels, pixels * (long long)spp,
                    pixels > 0 ? renderSeconds * (double)totalPixels / (double)pixels : 0.0);
    }

private:
    /* One worker prints the line; the others find the next tick moved on. */
    void tick(long long n)
    {
        double due = nextTick_.load();
        const double now = clock_.seconds();
        if (now < due || !nextTick_.compare_exchange_strong(due, now + 2.0))
            return;
        const double left = n > 0 ? now * (double)(total_ - n) / (double)n : 0.0;
        std::fprintf(stderr, "\r  render: %3d%%  %lld/%lld pixels  %.0f s, about %.0f s left   ",
                     (int)(100 * n / std::max(1LL, total_)), n, total_, now, left);
        std::fflush(stderr);
        ticked_ = true;
    }

    double seconds_;
    Timer clock_;
    std::atomic<long long> done_;
    long long total_;              /* pixels in the frame, when --progress is on; else 0 */
    std::atomic<double> nextTick_; /* when the next progress line is due, in clock_ seconds */
    std::atomic<bool> ticked_;
};

#endif /* TIMER_H */
