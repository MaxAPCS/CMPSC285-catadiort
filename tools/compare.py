#!/usr/bin/env python3
"""compare.py: compare renders with the reference renders (spec, Section 8).

    python tools/compare.py A.png B.png        two images (PNG or PFM)
    python tools/compare.py renders/2_Cook reference_renders/2_Cook
                                               two folders: every image of
                                               the first that the second has
    python tools/compare.py --noise A.pfm REF.pfm
                                               a Monte Carlo render against a
                                               converged one
    python tools/compare.py --partial A.png B.png
                                               only the pixels A has rendered
                                               (the ones that are not black)
    python tools/compare.py --crop X Y W H A.png B.png
                                               only that window of both

Two images or two folders: the RMSE over the pixel values clamped to
[0, 1] (8-bit sRGB values scaled to [0, 1] for a PNG, the linear floats
for a PFM), PASS when it is at most the scene's threshold (THRESHOLDS
below, 0.02 for a scene not listed), and, for an image that fails, a
difference image beside it, <name>_diff.png: the largest channel
difference, four times brighter, so a difference of a quarter is white.
The benchmark uses the same function.

--partial compares only the pixels that are not black in A: while a
renderer is half built (the spheres drawn, the floor not yet), it checks
what exists against the reference and ignores the rest. --crop compares
one window of both images, X Y from the top-left corner, W H its size.

--noise: is A, rendered with random noise (the independent sampler),
the same image as the converged REF apart from that noise? The
difference A - REF must average to zero, over the whole image and over
every 16 x 16 block, within 4 standard errors measured from the
difference itself; noise shrinks with spp and so does the tolerance, so
a bias the size of the noise passes at low spp and fails once the noise
is smaller. A mistake that does not shrink with spp (a wrong pdf, a
missing term, light counted twice) fails at some spp; a correct renderer
passes at every spp. Both images must be PFMs (gauchoRT --pfm).

Exit status 0 when everything compared passes. Needs NumPy; reads PNGs
with Pillow when it is installed and with the decoder here otherwise.
"""
import argparse
import os
import struct
import sys
import zlib

import numpy as np

# RMSE thresholds by scene name (the file name without extension); others
# use DEFAULT_THRESHOLD.
THRESHOLDS = {}
DEFAULT_THRESHOLD = 0.02


# images ------------------------------------------------------------------

def read_pfm(path):
    """A PFM as an (h, w, 3) float array, top row first."""
    with open(path, "rb") as f:
        kind = f.readline().strip()
        w, h = map(int, f.readline().split())
        scale = float(f.readline())
        data = np.fromfile(f, dtype="<f4" if scale < 0 else ">f4")
    channels = 3 if kind == b"PF" else 1
    img = data.reshape(h, w, channels)[::-1].astype(np.float64)
    return np.repeat(img, 3, axis=2) if channels == 1 else img


def read_png(path):
    """An 8-bit PNG as an (h, w, 3) float array in [0, 1] (sRGB values)."""
    try:
        from PIL import Image
        return np.asarray(Image.open(path).convert("RGB"), dtype=np.float64) / 255
    except ImportError:
        pass
    data = open(path, "rb").read()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise SystemExit("%s: not a PNG" % path)
    pos, idat = 8, []
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos:pos + 4])
        tag, body = data[pos + 4:pos + 8], data[pos + 8:pos + 8 + length]
        pos += 12 + length
        if tag == b"IHDR":
            w, h, depth, ctype, _, _, interlace = struct.unpack(">IIBBBBB", body)
        elif tag == b"IDAT":
            idat.append(body)
        elif tag == b"IEND":
            break
    if depth != 8 or interlace != 0 or ctype not in (0, 2, 4, 6):
        raise SystemExit("%s: only 8-bit non-interlaced PNGs (install Pillow for others)" % path)
    bpp = {0: 1, 2: 3, 4: 2, 6: 4}[ctype]
    raw = np.frombuffer(zlib.decompress(b"".join(idat)), np.uint8).reshape(h, 1 + w * bpp)
    out = np.zeros((h, w * bpp), np.uint8)
    prev = [0] * (w * bpp)
    for y in range(h):
        kind, line = raw[y, 0], raw[y, 1:]
        if kind == 0:
            cur = line.astype(np.int64)
        elif kind == 1:  # Sub: a running sum along the row, per channel
            cur = line.reshape(w, bpp).astype(np.int64).cumsum(axis=0).reshape(-1) % 256
        elif kind == 2:  # Up
            cur = (line.astype(np.int64) + np.asarray(prev)) % 256
        else:  # Average and Paeth depend on the pixel just decoded: one at a time
            vals, cur_l = line.tolist(), [0] * (w * bpp)
            for i in range(w * bpp):
                a = cur_l[i - bpp] if i >= bpp else 0
                b = prev[i]
                if kind == 3:
                    cur_l[i] = (vals[i] + (a + b) // 2) & 255
                else:
                    c = prev[i - bpp] if i >= bpp else 0
                    p = a + b - c
                    pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                    pred = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
                    cur_l[i] = (vals[i] + pred) & 255
            cur = np.asarray(cur_l)
        out[y] = cur
        prev = cur.tolist()
    img = out.reshape(h, w, bpp)[:, :, :3] if bpp >= 3 else np.repeat(out.reshape(h, w, bpp)[:, :, :1], 3, axis=2)
    return img.astype(np.float64) / 255


def read_image(path):
    return read_pfm(path) if path.lower().endswith(".pfm") else read_png(path)


def write_png(path, rgb):
    """An 8-bit RGB PNG from an (h, w, 3) uint8 array, with zlib only."""
    h, w, _ = rgb.shape
    raw = b"".join(b"\x00" + rgb[y].tobytes() for y in range(h))

    def chunk(tag, body):
        return struct.pack(">I", len(body)) + tag + body + struct.pack(">I", zlib.crc32(tag + body) & 0xFFFFFFFF)

    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
                + chunk(b"IDAT", zlib.compress(raw, 6)) + chunk(b"IEND", b""))


# comparisons -------------------------------------------------------------

def compare_pair(a_path, b_path, threshold=None, diff=True, crop=None, partial=False):
    """RMSE of two images clamped to [0, 1], and PASS or FAIL. Returns
    (passed, rmse). crop = (x, y, w, h) compares that window only; partial
    compares only the pixels that are not black in A."""
    a, b = read_image(a_path), read_image(b_path)
    name = os.path.splitext(os.path.basename(a_path))[0]
    if a.shape != b.shape:
        print("%-40s FAIL  sizes differ: %dx%d and %dx%d" % (name, a.shape[1], a.shape[0], b.shape[1], b.shape[0]))
        return False, float("inf")
    if crop:
        x, y, w, h = crop
        a, b = a[y:y + h, x:x + w], b[y:y + h, x:x + w]
    a, b = np.clip(a, 0, 1), np.clip(b, 0, 1)
    err = (a - b) ** 2
    mask = a.max(axis=2) > 0 if partial else None
    if partial and not mask.any():
        print("%-40s FAIL  nothing rendered yet: every pixel of the image is black" % name)
        return False, float("inf")
    rmse = float(np.sqrt(err[mask].mean())) if partial else float(np.sqrt(np.mean(err)))
    note = " over the %d%% of the pixels rendered" % round(100 * mask.mean()) if partial else ""
    limit = THRESHOLDS.get(name, DEFAULT_THRESHOLD) if threshold is None else threshold
    passed = rmse <= limit
    print("%-40s %s  RMSE %.5f (threshold %.3f)%s" % (name, "PASS" if passed else "FAIL", rmse, limit, note))
    if diff and not passed:
        d = np.clip(np.abs(a - b).max(axis=2) * 4, 0, 1)
        if partial:
            d = np.where(mask, d, 0)
        write_png(os.path.splitext(a_path)[0] + "_diff.png",
                  np.repeat((d * 255 + 0.5).astype(np.uint8)[:, :, None], 3, axis=2))
    return passed, rmse


def compare_folders(a_dir, b_dir, threshold=None, crop=None, partial=False):
    """Every PNG or PFM under a_dir that b_dir has at the same place."""
    passed = failed = missing = 0
    for root, _, files in sorted(os.walk(a_dir)):
        for f in sorted(files):
            if not f.lower().endswith((".png", ".pfm")) or f.endswith("_diff.png"):
                continue
            a = os.path.join(root, f)
            b = os.path.join(b_dir, os.path.relpath(a, a_dir))
            if not os.path.isfile(b):
                missing += 1
                continue
            ok, _ = compare_pair(a, b, threshold, crop=crop, partial=partial)
            passed, failed = passed + ok, failed + (not ok)
    print("%d passed, %d failed, %d with no reference" % (passed, failed, missing))
    return failed == 0


def compare_noise(a_path, ref_path, block=16, z=4.0):
    """Whether A - REF is zero-mean noise, overall and per block."""
    if not (a_path.lower().endswith(".pfm") and ref_path.lower().endswith(".pfm")):
        raise SystemExit("--noise compares PFMs (render with --pfm)")
    a, ref = read_pfm(a_path), read_pfm(ref_path)
    if a.shape != ref.shape:
        raise SystemExit("--noise: the images differ in size")
    d = (a - ref).mean(axis=2)  # the three channels together
    rmse = float(np.sqrt(np.mean((a - ref) ** 2)))
    n = d.size
    sd = float(d.std())
    overall = float(d.mean()) / (sd / np.sqrt(n)) if sd > 0 else 0.0
    h, w = d.shape
    bh, bw = h // block, w // block
    blocks = d[:bh * block, :bw * block].reshape(bh, block, bw, block).swapaxes(1, 2).reshape(bh, bw, -1)
    bsd = blocks.std(axis=2)
    bmean = blocks.mean(axis=2)
    # A block with no noise at all (a light seen directly, the black sky)
    # must match exactly: any difference there is a bias.
    with np.errstate(divide="ignore", invalid="ignore"):
        bz = np.where(bsd > 0, bmean / (bsd / block), np.where(np.abs(bmean) > 1e-6, np.inf, 0.0))
    bad = int(np.count_nonzero(np.abs(bz) > z))
    # With independent noise, |z| > 4 happens in about 1 block in 16,000;
    # allow one in 1,000 so a large image does not fail by chance.
    allowed = max(1, bz.size // 1000)
    passed = abs(overall) <= z and bad <= allowed
    print("%s  RMSE %.5f  mean difference %.2e (%.1f standard errors)  %d of %d blocks beyond %g  %s"
          % (os.path.basename(a_path), rmse, float(d.mean()), overall, bad, bz.size, z, "PASS" if passed else "FAIL"))
    if bad > allowed:
        worst = np.unravel_index(np.argmax(np.abs(bz)), bz.shape)
        print("    the worst block starts at pixel (%d, %d), %.1f standard errors off"
              % (worst[1] * block, worst[0] * block, bz[worst]))
    return passed


def main():
    ap = argparse.ArgumentParser(description="Compare renders with the reference renders.")
    ap.add_argument("a", help="an image or a folder")
    ap.add_argument("b", help="the reference image or folder")
    ap.add_argument("--noise", action="store_true", help="A is a Monte Carlo render: is A - B zero-mean noise?")
    ap.add_argument("--threshold", type=float, help="the RMSE threshold for every image (default: per scene)")
    ap.add_argument("--crop", nargs=4, type=int, metavar=("X", "Y", "W", "H"),
                    help="compare only that window of both images (X Y the top-left corner, W H the size)")
    ap.add_argument("--partial", action="store_true",
                    help="compare only the pixels that are not black in A: what you have rendered so far")
    args = ap.parse_args()
    if args.noise:
        return 0 if compare_noise(args.a, args.b) else 1
    if os.path.isdir(args.a):
        return 0 if compare_folders(args.a, args.b, args.threshold, args.crop, args.partial) else 1
    return 0 if compare_pair(args.a, args.b, args.threshold, crop=args.crop, partial=args.partial)[0] else 1


if __name__ == "__main__":
    sys.exit(main())
