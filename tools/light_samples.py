#!/usr/bin/env python3
"""light_samples.py: eye rays against shadow rays (HW3), after Hanrahan.

    python tools/light_samples.py [--rt PATH]

renders ShadowRays (scenes/2_Cook/1_TestScenes/ShadowRays.cpp: a square
lamp over a square card over a floor) under cook at 1, 4 and 16 spp,
each with --light-samples 1, 4 and 16, and lays the nine images out as
one picture, spp across and N down, each with its render time:
renders/2_Cook/2_StudyScenes/LightSamples.png (--out to change it). The
penumbra's noise depends on the shadow rays per pixel, spp times N, so
the images along a diagonal look alike; their times differ, since a
light sample is one shadow ray and a camera sample a whole path.
"""
import argparse
import os
import re
import subprocess
import sys
import tempfile

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import compare  # noqa: E402  (tools/compare.py: read_png, write_png)

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TIME = re.compile(r"TIME \S+ build=\S+ render=([0-9.]+)")


def find_rt():
    """The renderer: build/gauchoRT, build/Release/gauchoRT or rust/target/release/gauchoRT,
    the first that exists under the kit root (--rt names another)."""
    for path in ["build/gauchoRT", "build/gauchoRT.exe", "build/Release/gauchoRT.exe", "build/Release/gauchoRT",
                 "rust/target/release/gauchoRT", "rust/target/release/gauchoRT.exe"]:
        if os.path.isfile(os.path.join(ROOT, path)):
            return os.path.join(ROOT, path)
    raise SystemExit("%s: no gauchoRT executable under build/ or rust/target/release/; "
                     "build it or name it with --rt" % os.path.basename(sys.argv[0]))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--rt", help="your gauchoRT (default: the one under build/ or rust/target/release/)")
    ap.add_argument("--width", type=int, default=480)
    ap.add_argument("--height", type=int, default=360)
    ap.add_argument("--counts", default="1,4,16", help="the spp and the N to try")
    ap.add_argument("--out", default=os.path.join(ROOT, "renders", "2_Cook", "2_StudyScenes", "LightSamples.png"))
    a = ap.parse_args()
    a.rt = os.path.abspath(a.rt or find_rt())
    counts = [int(c) for c in a.counts.split(",")]
    pad, label = 6, 22
    w, h = a.width, a.height
    sheet = np.zeros((len(counts) * (h + label + pad), len(counts) * (w + pad), 3), dtype=np.uint8)
    captions = []
    with tempfile.TemporaryDirectory() as tmp:
        for row, n in enumerate(counts):
            for col, spp in enumerate(counts):
                out = os.path.join(tmp, "s%d_n%d" % (spp, n))
                r = subprocess.run([a.rt, "ShadowRays", "--integrator", "cook", "--spp", str(spp), "--light-samples", str(n),
                                    "--width", str(w), "--height", str(h), "--out", out],
                                   cwd=ROOT, capture_output=True, text=True)
                t = TIME.search(r.stderr) or TIME.search(r.stdout)
                if not os.path.exists(out + ".png") or not t:
                    sys.exit("ShadowRays at %d spp, N = %d did not render:\n%s" % (spp, n, r.stderr[-2000:]))
                img = (np.clip(compare.read_png(out + ".png"), 0, 1) * 255 + 0.5).astype(np.uint8)
                y, x = row * (h + label + pad) + label, col * (w + pad)
                sheet[y:y + h, x:x + w] = img[:, :, :3]
                caption = "%d spp, N = %d: %d shadow ray%s a pixel, %.2f s" % (spp, n, spp * n, "" if spp * n == 1 else "s",
                                                                                  float(t.group(1)))
                captions.append((x + 4, y - label + 4, caption))
                print(caption)
    os.makedirs(os.path.dirname(a.out), exist_ok=True)
    try:
        from PIL import Image, ImageDraw
        im = Image.fromarray(sheet)
        draw = ImageDraw.Draw(im)
        for x, y, text in captions:
            draw.text((x, y), text, fill=(255, 255, 255))
        im.save(a.out)
    except ImportError:
        compare.write_png(a.out, sheet)  # no captions without Pillow; they are printed above
    print("wrote", a.out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
