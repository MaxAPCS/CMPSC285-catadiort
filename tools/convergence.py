#!/usr/bin/env python3
"""convergence.py: the HW3 convergence study (the "Samplers" handout).

    python tools/convergence.py               the study
    python tools/convergence.py --make-ref    the converged reference first

Renders the Convergence scene (scenes/2_Cook/1_TestScenes/Convergence.cpp)
with each required sampler (independent, stratified, halton; add sobol
with --samplers) at 1, 4, 16, 64, 256 and 1024 spp, seed 0, with --pfm,
and measures each image's RMSE (linear
radiance, all pixels and channels) against the converged reference,
reference_renders/2_Cook/1_TestScenes/Convergence_ref.pfm (16,384 spp,
independent; the kit's), or renders/2_Cook/1_TestScenes/Convergence_ref.pfm
if there is none (the instructor's repo; --make-ref renders it there,
about a minute). Then:

  - prints the table and each sampler's fitted slope of log RMSE against
    log spp, over all six counts and over the low (1 to 16) and high (64
    to 1024) halves, and writes it to DIR/convergence.csv;
  - plots the log-log curves with their slopes to DIR/convergence.png,
    if Matplotlib is installed;
  - assembles renders/2_Cook/2_StudyScenes/SamplerGallery.png: the 4 spp
    image of each sampler side by side, in the order above.

DIR is renders/2_Cook/2_StudyScenes/ (--out to change it). Expected: a
slope of about -0.5 for independent; stratified and sobol steeper at low
spp and flattening toward -0.5 at high spp; halton between.

The reference has noise of its own: an RMSE of about a quarter of the
independent sampler's at 1024 spp, as large as the low-discrepancy
samplers' error there, which would flatten their curves at the high end
for no reason of theirs. Since that noise is independent of the image
being measured, the two add in quadrature, and the RMSE reported is
sqrt(measured^2 - reference^2), with the reference's own estimated as
the independent sampler's 1 spp RMSE over sqrt(16,384).

Options: --rt PATH (default: build/gauchoRT, build/Release/gauchoRT or
rust/target/release/gauchoRT, the first that exists), --spp LIST, --samplers
LIST, --threads N. Run from the repo root. Needs NumPy; Matplotlib for
the plot.
"""
import argparse
import csv
import math
import os
import struct
import subprocess
import sys
import tempfile
import zlib

import numpy as np

SCENE = "Convergence"
# The converged reference: in a kit, the instructor's, which make_kit.py
# copies into reference_renders/; in the instructor's repo, and wherever
# --make-ref renders one, renders/. The kit's comes first.
REF_KIT = "reference_renders/2_Cook/1_TestScenes/Convergence_ref"
REF_MADE = "renders/2_Cook/1_TestScenes/Convergence_ref"
REF_SPP = 16384
SAMPLERS = ["independent", "stratified", "halton"]
SPPS = [1, 4, 16, 64, 256, 1024]
GALLERY_SPP = 4


def find_rt():
    for path in ["build/gauchoRT", "build/gauchoRT.exe", "build/Release/gauchoRT.exe", "build/Release/gauchoRT",
                 "rust/target/release/gauchoRT", "rust/target/release/gauchoRT.exe"]:
        if os.path.isfile(path):
            return os.path.abspath(path)  # Windows will not start a relative path with / in it
    raise SystemExit("convergence.py: no gauchoRT executable under build/ or rust/target/release/; "
                     "build it, run from the repo root, or name it with --rt")


def render(rt, out, spp, sampler, threads):
    """gauchoRT Convergence at spp with sampler, seed 0, into out.png and out.pfm."""
    cmd = [rt, SCENE, "--integrator", "cook", "--spp", str(spp), "--sampler", sampler, "--seed", "0", "--pfm",
           "--out", out] + (["--threads", str(threads)] if threads else [])
    result = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if result.returncode != 0:
        raise SystemExit("convergence.py: %s failed:\n%s" % (" ".join(cmd), result.stderr.decode(errors="replace")))


def read_pfm(path):
    with open(path, "rb") as f:
        f.readline()
        w, h = map(int, f.readline().split())
        scale = float(f.readline())
        data = np.fromfile(f, dtype="<f4" if scale < 0 else ">f4")
    return data.reshape(h, w, 3)[::-1].astype(np.float64)


def write_png(path, rgb):
    """An 8-bit RGB PNG from an (h, w, 3) uint8 array, with zlib only."""
    h, w, _ = rgb.shape
    raw = b"".join(b"\x00" + rgb[y].tobytes() for y in range(h))

    def chunk(tag, body):
        return struct.pack(">I", len(body)) + tag + body + struct.pack(">I", zlib.crc32(tag + body) & 0xFFFFFFFF)

    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
                + chunk(b"IDAT", zlib.compress(raw, 6)) + chunk(b"IEND", b""))


def to_srgb8(linear):
    """Linear radiance to 8-bit sRGB, clamped, as the renderer writes PNGs."""
    c = np.clip(linear, 0, 1)
    s = np.where(c <= 0.0031308, 12.92 * c, 1.055 * np.power(c, 1 / 2.4) - 0.055)
    return (s * 255 + 0.5).astype(np.uint8)


def slope(spps, errors):
    """The least-squares slope of log(error) against log(spp)."""
    x, y = np.log(np.asarray(spps, float)), np.log(np.asarray(errors, float))
    return float(np.polyfit(x, y, 1)[0])


def plot(path, spps, table):
    try:
        import matplotlib
        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
    except ImportError:
        print("Matplotlib is not installed: no plot (the table above and the CSV have the numbers)")
        return
    fig, ax = plt.subplots(figsize=(6.4, 4.8), dpi=120)
    for sampler, errors in table.items():
        ax.loglog(spps, errors, "o-", label="%s (slope %.2f)" % (sampler, slope(spps, errors)))
    # A slope -0.5 guide through independent's first point.
    first = table[next(iter(table))][0]
    ax.loglog(spps, [first * s ** -0.5 for s in spps], "k:", label="slope -0.5")
    ax.set_xlabel("samples per pixel")
    ax.set_ylabel("RMSE against the converged image, its noise removed")
    ax.set_title("Convergence, cook")
    ax.grid(True, which="both", alpha=0.3)
    ax.legend()
    fig.tight_layout()
    fig.savefig(path)
    print("plot: " + path)


def main():
    ap = argparse.ArgumentParser(description="The HW3 convergence study.")
    ap.add_argument("--make-ref", action="store_true", help="render the converged reference first")
    ap.add_argument("--rt", help="the gauchoRT executable")
    ap.add_argument("--spp", default=",".join(map(str, SPPS)), help="comma-separated spp counts")
    ap.add_argument("--samplers", default=",".join(SAMPLERS), help="comma-separated samplers")
    ap.add_argument("--threads", type=int, default=0)
    ap.add_argument("--out", default="renders/2_Cook/2_StudyScenes", help="where the CSV, plot and gallery go")
    args = ap.parse_args()
    rt = os.path.abspath(args.rt) if args.rt else find_rt()  # Windows will not start a relative path with / in it
    spps = [int(s) for s in args.spp.split(",")]
    samplers = args.samplers.split(",")
    os.makedirs(args.out, exist_ok=True)

    if args.make_ref:
        os.makedirs(os.path.dirname(REF_MADE), exist_ok=True)
        print("rendering the reference: %d spp, independent" % REF_SPP)
        render(rt, REF_MADE, REF_SPP, "independent", args.threads)
        REF = REF_MADE
    else:
        REF = next((r for r in (REF_KIT, REF_MADE) if os.path.isfile(r + ".pfm")), None)
    if REF is None:
        raise SystemExit("convergence.py: no %s.pfm or %s.pfm; run with --make-ref first" % (REF_KIT, REF_MADE))
    print("reference: %s.pfm" % REF)
    ref = read_pfm(REF + ".pfm")

    table = {}
    gallery = []
    with tempfile.TemporaryDirectory() as tmp:
        for sampler in samplers:
            errors = []
            for spp in spps:
                out = os.path.join(tmp, "%s_%d" % (sampler, spp))
                render(rt, out, spp, sampler, args.threads)
                img = read_pfm(out + ".pfm")
                errors.append(float(np.sqrt(np.mean((img - ref) ** 2))))
                if spp == GALLERY_SPP:
                    gallery.append(img)
            table[sampler] = errors

    # Two samplers with the same errors at every count rendered the same images: a renderer without one of
    # them (sobol is extra credit) fell back to another.
    names = list(table)
    for i, a in enumerate(names):
        for b in names[i + 1:]:
            if table[a] == table[b]:
                print("convergence.py: %s and %s gave the same images at every count; does your renderer "
                      "implement %s? (Without sobol, pass --samplers independent,stratified,halton.)" % (a, b, b))

    # The reference's own noise, subtracted in quadrature (see above).
    if "independent" in table and spps[0] == 1:
        ref_mse = table["independent"][0] ** 2 / REF_SPP
        table = {s: [math.sqrt(max(e * e - ref_mse, 1e-12)) for e in errs] for s, errs in table.items()}
        print("the reference's own noise, RMSE %.5f, subtracted in quadrature" % math.sqrt(ref_mse))
    print("RMSE against the %d spp reference" % REF_SPP)
    print("%-12s" % "spp" + "".join("%11d" % s for s in spps) + "   slope  (low, high)")
    for sampler, errors in table.items():
        lo = [i for i, s in enumerate(spps) if s <= 16]
        hi = [i for i, s in enumerate(spps) if s >= 64]
        lo_s = slope([spps[i] for i in lo], [errors[i] for i in lo]) if len(lo) > 1 else float("nan")
        hi_s = slope([spps[i] for i in hi], [errors[i] for i in hi]) if len(hi) > 1 else float("nan")
        print("%-12s" % sampler + "".join("%11.5f" % e for e in errors)
              + "   %5.2f  (%5.2f, %5.2f)" % (slope(spps, errors), lo_s, hi_s))
    with open(os.path.join(args.out, "convergence.csv"), "w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(["sampler"] + spps)
        for sampler, errors in table.items():
            writer.writerow([sampler] + ["%.6g" % e for e in errors])
    plot(os.path.join(args.out, "convergence.png"), spps, table)

    if len(gallery) == len(samplers):
        gap = np.full((gallery[0].shape[0], 4, 3), 255, np.uint8)
        strip = [to_srgb8(gallery[0])]
        for img in gallery[1:]:
            strip += [gap, to_srgb8(img)]
        path = os.path.join(args.out, "SamplerGallery.png")
        write_png(path, np.concatenate(strip, axis=1))
        print("gallery (%d spp, %s): %s" % (GALLERY_SPP, ", ".join(samplers), path))
    return 0


if __name__ == "__main__":
    sys.exit(main())
