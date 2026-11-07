#!/usr/bin/env python3
"""timing_table.py: the write-up's timing table, for the benchmark and hero
scenes (the Suggested Workflow's last step; the BVH handout, "Timing").

    python tools/timing_table.py [--scenes Dragon,Stadium] [--threads 8]
        [--time-limit 300] [--runs 1] [--no-brute] [--exe PATH] [--out timing_table.md]

Run it from the kit's root folder, after the hero scenes pass. For every
scene of the benchmark and hero tiers (scenes/1_Whitted/0_scene_table.cpp;
--scenes picks some), it runs your renderer at the benchmark's settings,
--integrator whitted --threads 8 --width 1920 --height 1080, three ways:
with the SAH builder (--bvh sah), with the midpoint builder (--bvh
midpoint; a renderer with one builder ignores the flag and repeats its
numbers), and brute force (--no-accel --time-limit S, five minutes by
default): brute force would take hours to days per frame, so the
renderer stops handing out pixels after S seconds (each thread finishes
the row it is on) and prints a LIMIT line whose estimated_render scales
the time spent to the whole frame; that estimate is the brute-force
column. It reads the TIME lines' build and render and the LIMIT line's
estimate, and writes a Markdown table (the raw numbers as JSON beside
it): one row per scene, build and render seconds for each builder, and
the brute-force render as an estimate, marked. --runs 3 takes the median
of three for each builder's cells, which is the benchmark's own rule for
build + render; brute force runs once either way. The images go to
timing/ beside this script's output, never to renders/. Expect it to
take a while: a little over five minutes of brute force per scene, so
start it and write meanwhile.
"""
import argparse
import json
import os
import pathlib
import re
import statistics
import subprocess
import sys
import time

ROOT = pathlib.Path.cwd()
TIERS = ("benchmark", "hero")
ENTRY = re.compile(r'\{\s*"([^"]+)"\s*,\s*\w+\s*,\s*"([^"]+)"\s*\}')
TIME = re.compile(r"^TIME (\S+) build=([\d.]+) render=([\d.]+) total=([\d.]+) threads=(\d+)", re.M)
LIMIT = re.compile(r"^LIMIT (\S+) seconds=([\d.]+) pixels=(\d+)/(\d+) samples=(\d+) estimated_render=([\d.]+)", re.M)
TIMEOUT = 3600  # seconds per run


# the scenes and the renderer -------------------------------------------------

def tier_scenes(root):
    """The scene names of the benchmark and hero tiers, in table order."""
    table = root / "scenes" / "1_Whitted" / "0_scene_table.cpp"
    if not table.exists():
        sys.exit("timing_table: %s not found; run from the kit's root folder" % table)
    return [name for name, tier in ENTRY.findall(table.read_text(encoding="utf-8")) if tier in TIERS]


def find_exe(root):
    """The built renderer: Visual Studio's, another generator's, or the Rust kit's."""
    for p in ("build/Release/gauchoRT.exe", "build/gauchoRT.exe", "build/gauchoRT",
              "rust/target/release/gauchoRT.exe", "rust/target/release/gauchoRT"):
        if (root / p).exists():
            return root / p
    sys.exit("timing_table: no built renderer found under build/ or rust/target/release/; pass --exe")


# one run ------------------------------------------------------------------

def run(exe, scene, extra, threads, out_base):
    """One render: (build, render, limit_estimate or None, note)."""
    cmd = [str(exe), scene, "--integrator", "whitted", "--threads", str(threads), "--width", "1920", "--height", "1080",
           "--out", str(out_base)] + extra
    t0 = time.time()
    try:
        r = subprocess.run(cmd, cwd=str(ROOT), capture_output=True, text=True, timeout=TIMEOUT)
    except subprocess.TimeoutExpired:
        return None, None, None, "did not finish in %d s" % TIMEOUT
    m = TIME.search(r.stdout)
    if not m:
        last = (r.stderr.strip().splitlines() or r.stdout.strip().splitlines() or ["no output"])[-1]
        return None, None, None, "no TIME line (%s)" % last[:80]
    build, render = float(m.group(2)), float(m.group(3))
    lim = LIMIT.search(r.stdout)
    estimate = float(lim.group(6)) if lim else None
    return build, render, estimate, "%.1f s wall" % (time.time() - t0)


def median_cell(values):
    vals = [v for v in values if v is not None]
    return statistics.median(vals) if vals else None


def fmt(v, est=False):
    if v is None:
        return "failed"
    s = "%.2f" % v if v < 100 else "%.0f" % v
    return s + " (est.)" if est else s


# main ---------------------------------------------------------------------

def main():
    ap = argparse.ArgumentParser(description="the timing table of the benchmark and hero scenes")
    ap.add_argument("--scenes", help="only these, comma-separated (the table wants all)")
    ap.add_argument("--threads", type=int, default=8)
    ap.add_argument("--time-limit", type=float, default=300, help="seconds for each brute-force run (default 300)")
    ap.add_argument("--runs", type=int, default=1, help="runs per builder cell, the median kept (the benchmark takes 3)")
    ap.add_argument("--no-brute", action="store_true", help="skip the brute-force column")
    ap.add_argument("--exe", help="the renderer (default: the one under build/ or rust/target/release/)")
    ap.add_argument("--out", default="timing_table.md")
    args = ap.parse_args()

    exe = pathlib.Path(args.exe) if args.exe else find_exe(ROOT)
    scenes = args.scenes.split(",") if args.scenes else tier_scenes(ROOT)
    out_dir = pathlib.Path(args.out).resolve().parent / "timing"
    out_dir.mkdir(exist_ok=True)

    modes = [("sah", ["--bvh", "sah"]), ("midpoint", ["--bvh", "midpoint"])]
    if not args.no_brute:
        modes.append(("brute", ["--no-accel", "--time-limit", str(args.time_limit)]))

    rows = []
    for scene in scenes:
        row = {"scene": scene}
        for mode, extra in modes:
            builds, renders, estimates = [], [], []
            for k in range(1 if mode == "brute" else args.runs):  # an estimate needs no median
                build, render, estimate, note = run(exe, scene, extra, args.threads, out_dir / ("%s_%s" % (scene, mode)))
                print("%-26s %-9s run %d: build %s render %s %s%s" % (
                    scene, mode, k + 1, fmt(build), fmt(render), ("estimate %s " % fmt(estimate)) if estimate else "", note),
                    file=sys.stderr, flush=True)
                builds.append(build)
                renders.append(render)
                estimates.append(estimate)
            row[mode] = {"build": median_cell(builds), "render": median_cell(renders),
                         "estimate": median_cell(estimates)}
        rows.append(row)

    lines = ["| Scene | SAH build | SAH render | midpoint build | midpoint render | brute force render |",
             "|---|---|---|---|---|---|"]
    for row in rows:
        cells = [row["scene"]]
        for mode in ("sah", "midpoint"):
            cells += [fmt(row[mode]["build"]), fmt(row[mode]["render"])]
        if "brute" in row:
            b = row["brute"]
            cells.append(fmt(b["estimate"], est=True) if b["estimate"] is not None else fmt(b["render"]))
        else:
            cells.append("not run")
        lines.append("| " + " | ".join(cells) + " |")
    lines.append("")
    lines.append("Seconds at %d threads, 1920 x 1080, %s run%s per builder cell%s; brute force one run, the LIMIT "
                 "line's estimate of the whole frame after %.0f s (est.)." % (args.threads, args.runs, "" if args.runs == 1 else "s",
                                                     "" if args.runs == 1 else " (the median)", args.time_limit))
    table = "\n".join(lines)
    pathlib.Path(args.out).write_text(table + "\n", encoding="utf-8")
    pathlib.Path(args.out).with_suffix(".json").write_text(json.dumps(rows, indent=1), encoding="utf-8")
    print(table)
    print("wrote %s and %s" % (args.out, pathlib.Path(args.out).with_suffix(".json")), file=sys.stderr)


if __name__ == "__main__":
    main()
