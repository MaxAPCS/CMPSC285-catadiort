#!/usr/bin/env python3
"""area_light_test.py: is your area light unbiased? (HW3)

    python tools/area_light_test.py [--rt PATH]

renders QuadLight (scenes/2_Cook/1_TestScenes/QuadLight.cpp: one square
lamp over a matte floor, nothing else) under cook with the independent
sampler at 4, 16, 64 and 256 spp, and compares each with the exact image
computed here. The irradiance a floor point receives from a polygonal
Lambertian lamp has a closed form (Lambert's formula): half the lamp's
radiance times the sum, over the polygon's edges, of the angle the edge
subtends at the point times the cosine between the floor's normal and
the plane through the point and the edge. compare.py --noise, a test of
the mean difference, decides PASS. The RMSE printed beside it should
roughly halve with every fourfold spp, but its ratios do not decide PASS.
A wrong pdf or a missing cosine shows as a bias the test detects; a very
small bias may escape it.

--exact-only writes the exact image (QuadLight_exact.pfm) and stops.
"""
import argparse
import os
import subprocess
import sys
import tempfile

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import compare  # noqa: E402  (tools/compare.py: compare_noise)

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def find_rt():
    """The renderer: build/gauchoRT, build/Release/gauchoRT or rust/target/release/gauchoRT,
    the first that exists under the kit root (--rt names another)."""
    for path in ["build/gauchoRT", "build/gauchoRT.exe", "build/Release/gauchoRT.exe", "build/Release/gauchoRT",
                 "rust/target/release/gauchoRT", "rust/target/release/gauchoRT.exe"]:
        if os.path.isfile(os.path.join(ROOT, path)):
            return os.path.join(ROOT, path)
    raise SystemExit("%s: no gauchoRT executable under build/ or rust/target/release/; "
                     "build it or name it with --rt" % os.path.basename(sys.argv[0]))

# The scene, as QuadLight.cpp sets it: change them together.
EYE, LOOKAT, UP, FOVY = np.array([0, 1.1, 1.8]), np.array([0, 0, -0.1]), np.array([0, 1, 0]), 40.0
LAMP_SIZE, LAMP_HEIGHT, LAMP_LE, FLOOR_ALBEDO = 0.5, 1.5, 32.0, 0.5


def exact_image(width, height, sub=4):
    """The floor's radiance, (h, w, 3), each pixel the average over sub x sub
    points in it (the renderer's box filter), the camera as camera.cpp builds it."""
    forward = (LOOKAT - EYE) / np.linalg.norm(LOOKAT - EYE)
    right = np.cross(forward, UP)
    right /= np.linalg.norm(right)
    up = np.cross(right, forward)
    tan_half = np.tan(np.radians(FOVY) / 2)
    aspect = width / height
    offsets = (np.arange(sub) + 0.5) / sub
    total = np.zeros((height, width))
    xs, ys = np.meshgrid(np.arange(width, dtype=np.float64), np.arange(height, dtype=np.float64))
    h = LAMP_SIZE / 2
    corners = np.array([[-h, LAMP_HEIGHT, h], [h, LAMP_HEIGHT, h], [h, LAMP_HEIGHT, -h], [-h, LAMP_HEIGHT, -h]])
    for oy in offsets:
        for ox in offsets:
            sx, sy = (xs + ox) / width, (ys + oy) / height
            d = (forward[None, None, :] + right[None, None, :] * ((2 * sx - 1) * tan_half * aspect)[..., None]
                 + up[None, None, :] * ((1 - 2 * sy) * tan_half)[..., None])
            d /= np.linalg.norm(d, axis=2, keepdims=True)
            t = -EYE[1] / d[..., 1]  # the floor, y = 0 (every pixel sees it)
            p = EYE[None, None, :] + d * t[..., None]
            r = corners[None, None, :, :] - p[:, :, None, :]
            r /= np.linalg.norm(r, axis=3, keepdims=True)
            s = np.zeros(t.shape)
            for i in range(4):
                a, b = r[:, :, i, :], r[:, :, (i + 1) % 4, :]
                angle = np.arccos(np.clip(np.sum(a * b, axis=2), -1, 1))
                g = np.cross(a, b)
                g /= np.linalg.norm(g, axis=2, keepdims=True)
                s += angle * g[..., 1]  # the floor's normal is +y
            irradiance = LAMP_LE * np.abs(s) / 2
            total += FLOOR_ALBEDO / np.pi * irradiance
    return np.repeat((total / (sub * sub))[..., None], 3, axis=2)


def write_pfm(path, img):
    h, w, _ = img.shape
    with open(path, "wb") as f:
        f.write(b"PF\n%d %d\n-1.0\n" % (w, h))
        f.write(img[::-1].astype("<f4").tobytes())


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--rt", help="your gauchoRT (default: the one under build/ or rust/target/release/)")
    ap.add_argument("--width", type=int, default=480)
    ap.add_argument("--height", type=int, default=270)
    ap.add_argument("--spp", default="4,16,64,256", help="the sample counts to test")
    ap.add_argument("--keep", help="a folder to keep the images in (default: a temporary one)")
    ap.add_argument("--exact-only", action="store_true", help="write QuadLight_exact.pfm and stop")
    a = ap.parse_args()
    a.rt = os.path.abspath(a.rt or find_rt())

    folder = a.keep or tempfile.mkdtemp(prefix="area_light_test_")
    os.makedirs(folder, exist_ok=True)
    exact = os.path.join(folder, "QuadLight_exact.pfm")
    write_pfm(exact, exact_image(a.width, a.height))
    print("exact image:", exact)
    if a.exact_only:
        return 0

    results = []
    for spp in [int(s) for s in a.spp.split(",")]:
        out = os.path.join(folder, "QuadLight_%d" % spp)
        r = subprocess.run([a.rt, "QuadLight", "--integrator", "cook", "--sampler", "independent", "--spp", str(spp),
                            "--width", str(a.width), "--height", str(a.height), "--pfm", "--out", out],
                           cwd=ROOT, capture_output=True, text=True)
        if not os.path.exists(out + ".pfm"):
            sys.exit("the render at %d spp wrote no PFM:\n%s" % (spp, r.stderr[-2000:]))
        passed = compare.compare_noise(out + ".pfm", exact)
        rmse = float(np.sqrt(np.mean((compare.read_pfm(out + ".pfm") - compare.read_pfm(exact)) ** 2)))
        results.append((spp, rmse, passed))
    print("\n  spp    RMSE      ratio to the previous (about 2 when unbiased)")
    for i, (spp, rmse, passed) in enumerate(results):
        ratio = "%.2f" % (results[i - 1][1] / rmse) if i else ""
        print("  %4d   %.5f   %s" % (spp, rmse, ratio))
    ok = all(p for _, _, p in results)
    print("\nPASS: no significant mean difference detected at the tested spp" if ok else "\nFAIL: the error does not behave like zero-mean noise")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
