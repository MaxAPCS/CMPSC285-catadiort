#!/usr/bin/env python3
"""warptest.py: the chi-square test of the HW3 warps (the "Warps" handout).

Runs `gauchoRT --warptest WARP N [PARAM]` (or reads a saved dump of it), bins
the N points, integrates the warp's analytic density over every bin to
get the expected counts, and runs Pearson's chi-square test. It also
checks that every point lies in the warp's domain and that the pdf the
renderer printed beside each point is the analytic density there, so a
warp passes only when its samples and its pdf agree with the formula.

    python tools/warptest.py cosine_hemisphere
    python tools/warptest.py uniform_cone 0.8
    python tools/warptest.py phong 20 -n 1000000
    python tools/warptest.py ggx 0.25        HW4: GGX's visible normals
    python tools/warptest.py all            the six warps, one PARAM each
    python tools/warptest.py --selftest     the test itself, on warps written
                                            here in NumPy: the correct ones
                                            must pass, the broken ones fail
    python tools/warptest.py disk -n 256 --sampler stratified --scatter
                                            draw 256 points, no test: how the
                                            warp carries a sampler's strata
    python tools/warptest.py all --view     each warp's points as red spheres on
                                            its domain, drawn by your renderer
                                            (the scene WarpView, under cook)

Options: -n N (samples, default 1,000,000), --seed S, --rt PATH (the
executable; default: the first of build/gauchoRT, build/Release/gauchoRT,
rust/target/release/gauchoRT that exists), --input FILE (a dump saved from
`gauchoRT --warptest ... > FILE` instead of running the renderer), --outdir DIR
(default warptest/), --alpha A (significance level, default 0.01),
--sampler NAME (passed on to the renderer, whose dump is then that
sampler's N samples of pixel (0, 0); the test wants independent points,
the renderer's default), --scatter (draw the points on the warp's domain,
DIR/<warp>_scatter[_<sampler>].png, instead of testing them), --view (no
test: write the dump to warptest/<warp>.txt, where the scene WarpView reads
it, and render the scene under cook, DIR/<warp>_view.png, which matches
reference_renders/2_Cook/1_TestScenes/WarpView_<warp>.png when the warp
draws the reference's points; 1,024 points of the stratified sampler
unless -n and --sampler say otherwise), --dump (the same dump, and no
render).

Binning. The disk is binned on a 50 x 50 grid over [-1, 1]^2 and the
triangle on a 50 x 50 grid over its (b0, b1) square. Directions are
binned on a grid in (z, phi), z = cos(theta): 40 rows of equal height in
z and 80 columns of equal width in phi, so every bin has the same solid
angle (d omega = dz dphi). The rows cover the z range where the warp puts
all but 0.1% of its mass; the rest, below, is one more bin, so a narrow
Phong lobe is binned as finely as a wide one. Bins expected to hold fewer
than 5 points are pooled before the test, as Pearson's approximation
needs. Needs NumPy; SciPy, if present, computes the p-value (otherwise
the regularized incomplete gamma function here does, to the same digits).

For each warp it writes DIR/<warp>[_<param>].png: three panels, the
observed counts, the expected counts (the same color scale), and the
residual (observed - expected) / sqrt(expected) from blue (-4) through
white to red (+4). A correct warp shows two panels alike and a residual
of fine noise; a broken one shows the pattern of its mistake.

Reading the result. At the default alpha of 0.01 a correct warp fails
one run in a hundred by chance, with p a little below 0.01; rerun it
with another --seed. A real mistake fails at every seed, nearly always
with p below 1e-6, and its residual panel shows a pattern rather than
noise.

Exit status 0 when every warp tested passes.
"""
import argparse
import math
import re
import os
import struct
import subprocess
import sys
import zlib

import numpy as np

try:
    from scipy.stats import chi2 as _scipy_chi2
except ImportError:
    _scipy_chi2 = None

TWO_PI = 2 * math.pi

# The six warps of HW3, with the PARAM `all` tests each at; ggx (HW4,
# GGX's visible normals, PARAM alpha) is tested by name only.
WARPS = ["disk", "cosine_hemisphere", "uniform_sphere", "uniform_cone", "triangle", "phong"]
DEFAULT_PARAM = {"uniform_cone": 0.8, "phong": 20.0, "ggx": 0.25}
VIEW_FRAMES = WARPS + ["ggx"]  # a warp's frame number in the scene WarpView, in its table's order


# domains ----------------------------------------------------------------

class Domain:
    """What the test needs to know about one warp: how to turn its points
    into 2D bin coordinates, which points are legal, and its analytic
    density in those coordinates."""

    # kind "plane": coordinates (a, b) on [a0, a1] x [b0, b1], density per area
    # kind "sphere": coordinates (phi, z), z in [zmin, 1], density per steradian


def make_domain(warp, param):
    d = Domain()
    d.warp, d.param = warp, param
    if warp == "disk":
        d.kind = "plane"
        d.bounds = (-1.0, 1.0, -1.0, 1.0)
        d.coords = lambda p: (p[:, 0], p[:, 1])
        d.legal = lambda p: (np.abs(p[:, 2]) <= 1e-7) & (p[:, 0] ** 2 + p[:, 1] ** 2 <= 1 + 1e-6)
        d.density = lambda x, y: np.where(x * x + y * y <= 1, 1 / math.pi, 0.0)
    elif warp == "triangle":
        # Barycentrics (b0, b1, b2); the density is over the (b0, b1) plane,
        # where the triangle b0, b1 >= 0, b0 + b1 <= 1 has area 1/2.
        d.kind = "plane"
        d.bounds = (0.0, 1.0, 0.0, 1.0)
        d.coords = lambda p: (p[:, 0], p[:, 1])
        d.legal = lambda p: np.all(p >= -1e-6, axis=1) & (np.abs(p.sum(axis=1) - 1) <= 1e-5)

        def density(x, y):
            s = x + y
            # The diagonal runs exactly through some of the integration
            # points; each of those is half inside.
            return 2.0 * np.where(np.abs(s - 1) < 1e-9, 0.5, (s < 1).astype(float))

        d.density = density
    else:
        d.kind = "sphere"
        if warp == "cosine_hemisphere":
            d.zmin = 0.0
            d.density = lambda phi, z: np.maximum(z, 0) / math.pi
        elif warp == "uniform_sphere":
            d.zmin = -1.0
            d.density = lambda phi, z: np.full_like(z, 1 / (4 * math.pi))
        elif warp == "uniform_cone":
            c = float(np.float32(param))  # the renderer reads PARAM as a float
            if not -1 <= c < 1:
                raise SystemExit("uniform_cone: cosThetaMax must be in [-1, 1), not %g" % c)
            d.zmin = c
            d.density = lambda phi, z: np.where(z >= c, 1 / (TWO_PI * (1 - c)), 0.0)
        elif warp == "phong":
            n = float(np.float32(param))
            if n < 0:
                raise SystemExit("phong: the exponent must be at least 0, not %g" % n)
            d.zmin = 0.0
            d.density = lambda phi, z: (n + 1) / TWO_PI * np.maximum(z, 0) ** n
        elif warp == "ggx":
            # HW4: GGX's visible normals for wo along the normal, which are
            # distributed as D(wm) cos(theta_m).
            a = float(np.float32(param))
            if not 0 < a <= 1:
                raise SystemExit("ggx: alpha must be in (0, 1], not %g" % a)
            d.zmin = 0.0
            d.density = lambda phi, z: (a * a / (math.pi * ((a * a - 1) * np.maximum(z, 0) ** 2 + 1) ** 2)
                                        * np.maximum(z, 0))
        else:
            raise SystemExit("warptest.py: no warp '%s' (one of %s, or all)" % (warp, ", ".join(WARPS)))
        d.coords = lambda p: (np.mod(np.arctan2(p[:, 1], p[:, 0]), TWO_PI), polar_z(p))
        d.pdf_coords = lambda p: (np.mod(np.arctan2(p[:, 1], p[:, 0]), TWO_PI), np.clip(p[:, 2], -1, 1))
        d.legal = lambda p: (np.abs(np.sqrt((p[:, :3] ** 2).sum(axis=1)) - 1) <= 1e-5) & (p[:, 2] >= d.zmin - 1e-6)
        d.zgrid, d.cdf = marginal_cdf(d)
        d.zcut = z_cut(d)
    return d


def polar_z(p):
    """cos(theta) of each direction for binning. Near the pole a float z
    is too coarse (its spacing, 6e-8, is a sizable part of a row in a
    narrow cone or lobe), while x and y keep full relative precision, so
    there z is recomputed from sin^2(theta) = x^2 + y^2."""
    z = np.clip(p[:, 2], -1, 1)
    r2 = p[:, 0] ** 2 + p[:, 1] ** 2
    return np.where(z > 0.7, np.sqrt(np.maximum(0, 1 - r2)), z)


def marginal_cdf(d):
    """The cumulative distribution of z, from zmin up to 1, of a direction
    warp's density, which depends on z alone (every warp here is
    symmetric about +z): the marginal 2 pi density(z) integrated by the
    trapezoid rule on points packed geometrically toward the pole, so a
    lobe a hundred-thousandth wide in z (GGX at alpha 0.01) is resolved.
    Returns (z ascending, cdf)."""
    s = np.concatenate([[0.0], np.geomspace(1e-13, 1 - d.zmin, 400000)])  # 1 - z, from the pole down
    z = 1 - s
    marginal = d.density(np.zeros_like(z), z) * TWO_PI
    from_pole = np.concatenate([[0.0], np.cumsum(0.5 * (marginal[1:] + marginal[:-1]) * np.diff(s))])
    return z[::-1], (from_pole[-1] - from_pole)[::-1]


def z_cut(d):
    """The z below which the warp puts 0.1% of its mass: rows are laid out
    from there to 1, and everything below is one extra bin. When that
    leaves out less than 2% of the z range, the rows cover all of it."""
    zc = float(np.interp(0.001 * d.cdf[-1], d.cdf, d.zgrid))
    return d.zmin if zc - d.zmin < 0.02 * (1 - d.zmin) else zc


def integrate(density, a0, a1, na, b0, b1, nb, sub=16):
    """The integral of density(a, b) over each cell of an na x nb grid on
    [a0, a1] x [b0, b1], by the midpoint rule on sub x sub points per
    cell. Returns an (nb, na) array."""
    a = a0 + (np.arange(na * sub) + 0.5) * (a1 - a0) / (na * sub)
    b = b0 + (np.arange(nb * sub) + 0.5) * (b1 - b0) / (nb * sub)
    A, B = np.meshgrid(a, b)
    cell = (a1 - a0) / (na * sub) * (b1 - b0) / (nb * sub)
    return density(A, B).reshape(nb, sub, na, sub).sum(axis=(1, 3)) * cell


def histograms(d, points):
    """Observed and expected counts: the grid as (rows, columns) arrays,
    plus, for directions below the row range, one extra bin. Returns
    (observed grid, expected grid, observed extra, expected extra, the
    total of the density, for the normalization check)."""
    n = len(points)
    a, b = d.coords(points)
    if d.kind == "plane":
        a0, a1, b0, b1 = d.bounds
        na = nb = 50
        expected = integrate(d.density, a0, a1, na, b0, b1, nb) * n
        ia = np.clip(((a - a0) / (a1 - a0) * na).astype(int), 0, na - 1)
        ib = np.clip(((b - b0) / (b1 - b0) * nb).astype(int), 0, nb - 1)
        observed = np.zeros((nb, na))
        np.add.at(observed, (ib, ia), 1)
        return observed, expected, 0.0, 0.0, expected.sum() / n
    nphi, nz = 80, 40
    zc = d.zcut
    # Each row's mass from the marginal cdf, shared evenly by its columns
    # (the density does not depend on phi).
    F = np.interp(zc + (1 - zc) * np.arange(nz + 1) / nz, d.zgrid, d.cdf)
    expected = np.repeat((np.diff(F) * n / nphi)[:, None], nphi, axis=1)
    extra_expected = float(np.interp(zc, d.zgrid, d.cdf)) * n if zc > d.zmin else 0.0
    phi, z = a, b
    below = z < zc if zc > d.zmin else np.zeros(len(z), bool)  # without an extra bin, row 0 takes the edge
    iphi = np.clip((phi / TWO_PI * nphi).astype(int), 0, nphi - 1)
    iz = np.clip(((z - zc) / (1 - zc) * nz).astype(int), 0, nz - 1)
    observed = np.zeros((nz, nphi))
    np.add.at(observed, (iz[~below], iphi[~below]), 1)
    # Rows run up in z; flip them so the pole (z = 1) is the top row.
    return observed[::-1], expected[::-1], float(below.sum()), extra_expected, (expected.sum() + extra_expected) / n


# statistics -------------------------------------------------------------

def chi2_sf(x, dof):
    """P(chi-square with dof degrees of freedom >= x): the regularized
    upper incomplete gamma function Q(dof / 2, x / 2), by its series below
    a + 1 and its continued fraction above (Numerical Recipes, 6.2)."""
    if _scipy_chi2 is not None:
        return float(_scipy_chi2.sf(x, dof))
    return gamma_q(dof / 2.0, x / 2.0)


def gamma_q(a, x):
    if x <= 0:
        return 1.0
    front = math.exp(-x + a * math.log(x) - math.lgamma(a))
    if x < a + 1:
        term = total = 1.0 / a
        ap = a
        for _ in range(100000):
            ap += 1
            term *= x / ap
            total += term
            if abs(term) < abs(total) * 1e-16:
                break
        return max(0.0, 1.0 - total * front)
    tiny = 1e-300
    b = x + 1 - a
    c = 1 / tiny
    dd = 1 / b
    h = dd
    for i in range(1, 100000):
        an = -i * (i - a)
        b += 2
        dd = an * dd + b
        dd = tiny if abs(dd) < tiny else dd
        c = b + an / c
        c = tiny if abs(c) < tiny else c
        dd = 1 / dd
        delta = dd * c
        h *= delta
        if abs(delta - 1) < 1e-16:
            break
    return front * h


def pearson(observed, expected, min_expected=5.0):
    """Pearson's chi-square over bins pooled, from the least expected up,
    until each pool expects at least min_expected. Returns (statistic,
    degrees of freedom, points in bins whose expected count is zero)."""
    o = np.asarray(observed, float).ravel()
    e = np.asarray(expected, float).ravel()
    empty = e <= 1e-9
    stray = float(o[empty].sum())
    o, e = o[~empty], e[~empty]
    order = np.argsort(e, kind="stable")
    o, e = o[order], e[order]
    pooled_o, pooled_e = [], []
    acc_o = acc_e = 0.0
    for oi, ei in zip(o, e):
        acc_o += oi
        acc_e += ei
        if acc_e >= min_expected:
            pooled_o.append(acc_o)
            pooled_e.append(acc_e)
            acc_o = acc_e = 0.0
    if acc_e > 0:  # what is left joins the last pool
        if pooled_e:
            pooled_o[-1] += acc_o
            pooled_e[-1] += acc_e
        else:
            pooled_o.append(acc_o)
            pooled_e.append(acc_e)
    po, pe = np.array(pooled_o), np.array(pooled_e)
    return float(((po - pe) ** 2 / pe).sum()), len(pe) - 1, stray


# the test ------------------------------------------------------------------

def run_test(d, data, alpha, png):
    """Test one dump (an (N, 4) array of x y z pdf). Prints a report and
    returns True when the warp passes."""
    points, pdf = data[:, :3], data[:, 3]
    n = len(points)
    label = d.warp + ("" if d.param is None else " %g" % d.param)
    problems = []

    legal = d.legal(points)
    if not legal.all():
        bad = points[~legal][0]
        problems.append("%d points outside the domain (the first: %g %g %g)" % ((~legal).sum(), *bad))
    points, pdf = points[legal], pdf[legal]

    # The pdf column against the density at the point. For a direction near
    # the pole the float z the renderer printed and the z its x and y give
    # can differ in the last bits, and a sharp lobe's density with them; a
    # renderer may compute its pdf from either, so the closer one counts.
    a, b = d.coords(points)
    analytic = d.density(a, b)
    if hasattr(d, "pdf_coords"):
        raw = d.density(*d.pdf_coords(points))
        analytic = np.where(np.abs(pdf - raw) < np.abs(pdf - analytic), raw, analytic)
    zero = analytic <= 0
    if np.any(pdf[zero] != 0):
        problems.append("%d points with a nonzero pdf where the density is zero" % np.count_nonzero(pdf[zero]))
    rel = np.abs(pdf[~zero] - analytic[~zero]) / analytic[~zero]
    worst = float(rel.max()) if rel.size else 0.0
    if worst > 1e-3:
        i = int(np.argmax(rel))
        problems.append("the pdf column is not the density: off by %.3g%% at worst (printed %g, density %g)"
                        % (100 * worst, pdf[~zero][i], analytic[~zero][i]))

    observed, expected, extra_o, extra_e, total = histograms(d, points)
    if abs(total - 1) > 1e-3:
        problems.append("the analytic density integrates to %.6f, not 1 (a bug in warptest.py)" % total)
    obs = np.append(observed.ravel(), extra_o) if extra_e > 0 else observed.ravel()
    exp = np.append(expected.ravel(), extra_e) if extra_e > 0 else expected.ravel()
    stat, dof, stray = pearson(obs, exp)
    if stray > 0:
        problems.append("%d points in bins where the density is zero" % stray)
    p = chi2_sf(stat, dof)
    if p < alpha:
        problems.append("chi-square %.1f with %d degrees of freedom: p = %.3g < %g" % (stat, dof, p, alpha))

    ok = not problems
    print("%-24s N = %d  chi2 = %.1f  dof = %d  p = %.3g  pdf error %.1e  %s"
          % (label, n, stat, dof, p, worst, "PASS" if ok else "FAIL"))
    for msg in problems:
        print("    " + msg)
    if len(problems) == 1 and p < alpha and p > 1e-6:
        print("    p is small but not tiny: at alpha = %g, one correct warp in %d fails by chance."
              % (alpha, round(1 / alpha)))
        print("    Rerun with another --seed. A real mistake fails at every seed, usually with p near 0.")
    if png:
        write_panels(png, observed, expected)
        print("    histograms: " + png)
    return ok


# the picture ------------------------------------------------------------

def colormap(t, stops):
    """t in [0, 1] through piecewise-linear RGB stops, as uint8."""
    stops = np.asarray(stops, float)
    x = np.clip(t, 0, 1) * (len(stops) - 1)
    i = np.minimum(x.astype(int), len(stops) - 2)
    f = (x - i)[..., None]
    return (stops[i] * (1 - f) + stops[i + 1] * f).round().astype(np.uint8)


VIRIDIS = [(68, 1, 84), (59, 82, 139), (33, 145, 140), (94, 201, 98), (253, 231, 37)]
DIVERGING = [(59, 76, 192), (240, 240, 240), (180, 4, 38)]


def write_panels(path, observed, expected):
    """observed | expected | residual, each bin a block of pixels."""
    rows, cols = observed.shape
    k = max(1, 320 // max(rows, cols))
    top = max(observed.max(), expected.max(), 1e-9)
    with np.errstate(divide="ignore", invalid="ignore"):
        z = np.where(expected > 0, (observed - expected) / np.sqrt(expected), np.where(observed > 0, 4.0, 0.0))
    panels = [colormap(observed / top, VIRIDIS), colormap(expected / top, VIRIDIS),
              colormap((z + 4) / 8, DIVERGING)]
    panels = [np.repeat(np.repeat(p, k, axis=0), k, axis=1) for p in panels]
    gap = np.full((rows * k, 8, 3), 32, np.uint8)
    image = np.concatenate([panels[0], gap, panels[1], gap, panels[2]], axis=1)
    write_png(path, image)


def write_scatter(path, d, points, size=400):
    """The points as dots on the warp's domain, in the test's own
    coordinates (the disk's (x, y), the triangle's (b0, b1), a direction's
    (phi, z), which are equal-area): a look at how the warp carries a
    sampler's strata, no test."""
    a, b = d.coords(points)
    if d.kind == "plane":
        a0, a1, b0, b1 = d.bounds
    else:
        a0, a1, b0, b1 = 0.0, TWO_PI, d.zmin, 1.0
    image = np.full((size, size, 3), 255, np.uint8)
    if d.warp == "disk":  # the unit circle, for the eye
        t = np.linspace(0, TWO_PI, 4 * size)
        cx = np.clip(((np.cos(t) + 1) / 2 * (size - 1)).astype(int), 0, size - 1)
        cy = np.clip(((1 - np.sin(t)) / 2 * (size - 1)).astype(int), 0, size - 1)
        image[cy, cx] = 170
    x = np.clip(((a - a0) / (a1 - a0) * (size - 1)).astype(int), 0, size - 1)
    y = np.clip(((b1 - b) / (b1 - b0) * (size - 1)).astype(int), 0, size - 1)
    for dx in (-1, 0, 1):
        for dy in (-1, 0, 1):
            image[np.clip(y + dy, 0, size - 1), np.clip(x + dx, 0, size - 1)] = 0
    write_png(path, image)


def write_png(path, rgb):
    """An 8-bit RGB PNG from an (h, w, 3) uint8 array, with zlib only."""
    h, w, _ = rgb.shape
    raw = b"".join(b"\x00" + rgb[y].tobytes() for y in range(h))

    def chunk(tag, data):
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
                + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


# the renderer ---------------------------------------------------------

def find_rt():
    for path in ["target/release/catadiort", "target/release/catadiort.exe"]:
        if os.path.isfile(path):
            return os.path.abspath(path)  # Windows will not start a relative path with / in it
    raise SystemExit("warptest.py: no executable at target/release/catadiort or target/release/catadiort.exe; "
                     "build it, run from the repo root, or name it with --rt")


def parse_dump(text):
    # The renderer prints floats with 9 significant digits, which read back
    # exactly as float32: rounding them there recovers the very values it
    # computed with (as a double, a printed cosThetaMax can land a hair
    # below the float it was).
    values = np.array(text.split(), dtype=np.float64).astype(np.float32).astype(np.float64)
    if values.size % 4:
        raise SystemExit("warptest.py: the dump is not lines of four numbers")
    return values.reshape(-1, 4)


def dump_from_rt(rt, warp, n, param, seed, sampler=None):
    cmd = [rt, "--warptest", warp, str(n)] + ([] if param is None else [repr(param)]) + ["--seed", str(seed)]
    if sampler:
        cmd += ["--sampler", sampler]
    try:
        result = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    except OSError as e:
        raise SystemExit("warptest.py: cannot run %s: %s" % (rt, e))
    if result.returncode != 0:
        raise SystemExit("warptest.py: %s failed:\n%s" % (" ".join(cmd), result.stderr.decode(errors="replace")))
    return parse_dump(result.stdout)


def write_dump(warp, data):
    """The dump where the scene WarpView reads it, as the renderer printed it: 9 significant digits."""
    os.makedirs("warptest", exist_ok=True)
    np.savetxt(os.path.join("warptest", warp + ".txt"), data, fmt="%.9g")


def render_view(rt, warp, outdir):
    """This warp's frame of the scene WarpView under cook, which has its reference: the scene is lit by a
    directional light and the ambient term, which cook draws as whitted did, so it matches from the
    stratified sampler on."""
    os.makedirs(outdir, exist_ok=True)
    out = os.path.join(outdir, warp + "_view")
    cmd = [rt, "WarpView", "--integrator", "cook", "--frames", str(VIEW_FRAMES.index(warp)), "--out", out]
    try:
        result = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    except OSError as e:
        raise SystemExit("warptest.py: cannot run %s: %s" % (rt, e))
    if result.returncode != 0 or not os.path.exists(out + ".png"):
        raise SystemExit("warptest.py: %s wrote no image:\n%s" % (" ".join(cmd), result.stderr.decode(errors="replace")))
    return out + ".png"


def png_name(outdir, warp, param, suffix=""):
    os.makedirs(outdir, exist_ok=True)
    tag = warp + ("" if param is None else "_%g" % param) + suffix
    return os.path.join(outdir, tag + ".png")


# self-test ---------------------------------------------------------------

def numpy_warps(rng, n):
    """The warps written again in NumPy, correct and broken, each as
    (name, warp, param, data, should pass)."""
    u, v = rng.random(n), rng.random(n)

    def concentric(u, v):
        a, b = 2 * u - 1, 2 * v - 1
        first = np.abs(a) > np.abs(b)
        with np.errstate(divide="ignore", invalid="ignore"):
            r = np.where(first, a, b)
            t = np.where(first, (math.pi / 4) * (b / a), math.pi / 2 - (math.pi / 4) * (a / b))
        r = np.where((a == 0) & (b == 0), 0, r)
        t = np.nan_to_num(t)
        return r * np.cos(t), r * np.sin(t)

    def direction(z, phi):
        s = np.sqrt(np.maximum(0, 1 - z * z))
        return np.stack([s * np.cos(phi), s * np.sin(phi), z], axis=1)

    def with_pdf(p, pdf):
        return np.column_stack([p, np.broadcast_to(pdf, (len(p),))])

    out = []
    x, y = concentric(u, v)
    out.append(("disk", "disk", None, with_pdf(np.stack([x, y, 0 * x], 1), 1 / math.pi), True))
    p = np.stack([x, y, np.sqrt(np.maximum(0, 1 - x * x - y * y))], 1)
    out.append(("cosine_hemisphere", "cosine_hemisphere", None, with_pdf(p, p[:, 2] / math.pi), True))
    p = direction(1 - 2 * u, TWO_PI * v)
    out.append(("uniform_sphere", "uniform_sphere", None, with_pdf(p, 1 / (4 * math.pi)), True))
    c = 0.8
    p = direction(1 - u * (1 - c), TWO_PI * v)
    out.append(("uniform_cone 0.8", "uniform_cone", c, with_pdf(p, 1 / (TWO_PI * (1 - c))), True))
    s = np.sqrt(u)
    p = np.stack([1 - s, v * s, s - v * s], 1)
    out.append(("triangle", "triangle", None, with_pdf(p, 2.0), True))
    for n_exp in (0.0, 20.0, 800.0):
        p = direction(u ** (1 / (n_exp + 1)), TWO_PI * v)
        out.append(("phong %g" % n_exp, "phong", n_exp,
                    with_pdf(p, (n_exp + 1) / TWO_PI * p[:, 2] ** n_exp), True))

    # Broken: each should fail, and the note says what is wrong with it.
    p = direction(u, TWO_PI * v)  # uniform hemisphere
    out.append(("uniform hemisphere passed off as cosine", "cosine_hemisphere", None,
                with_pdf(p, p[:, 2] / math.pi), False))
    r, t = u, TWO_PI * v  # r uniform instead of sqrt(u): crowded at the center
    out.append(("disk with r = u", "disk", None, with_pdf(np.stack([r * np.cos(t), r * np.sin(t), 0 * r], 1),
                                                         1 / math.pi), False))
    p = np.stack([u * (1 - v), v, 1 - u * (1 - v) - v], 1)  # folded square, not uniform
    out.append(("triangle by b0 = u (1 - v), b1 = v", "triangle", None, with_pdf(p, 2.0), False))
    p = direction(u ** (1 / 20.0), TWO_PI * v)  # exponent off by one
    out.append(("phong 20 with cos = u^(1/n)", "phong", 20.0,
                with_pdf(p, 21 / TWO_PI * p[:, 2] ** 20), False))
    p = direction(1 - u * (1 - c), TWO_PI * v)  # right samples, wrong pdf
    out.append(("cone 0.8 with pdf 1 / (2 pi (1 + c))", "uniform_cone", c,
                with_pdf(p, 1 / (TWO_PI * (1 + c))), False))
    p = direction(1 - u * (1 - c), math.pi * v)  # phi over half the circle
    out.append(("cone 0.8 with phi = pi v", "uniform_cone", c, with_pdf(p, 1 / (TWO_PI * (1 - c))), False))
    return out


def selftest(args):
    rng = np.random.default_rng(args.seed)
    wrong = 0
    for name, warp, param, data, should_pass in numpy_warps(rng, args.n):
        print("[%s] expect %s" % (name, "PASS" if should_pass else "FAIL"))
        png = png_name(args.outdir, "selftest_" + re.sub(r"[^A-Za-z0-9.]+", "_", name).strip("_"), None)
        passed = run_test(make_domain(warp, param), data, args.alpha, png)
        if passed != should_pass:
            wrong += 1
            print("    ^ the test got this one WRONG")
    # The p-value without SciPy, against SciPy's when it is here.
    if _scipy_chi2 is not None:
        worst = 0.0
        for dof in (1, 10, 99, 1000, 3000):
            for x in (0.5 * dof, dof, 1.1 * dof, 1.3 * dof, 2 * dof):
                ours, theirs = gamma_q(dof / 2, x / 2), float(_scipy_chi2.sf(x, dof))
                worst = max(worst, abs(ours - theirs) / max(theirs, 1e-300) if theirs > 1e-12 else abs(ours - theirs))
        print("p-value without SciPy: worst relative difference from SciPy %.1e" % worst)
        if worst > 1e-6:
            wrong += 1
    print("self-test: %s" % ("all as expected" if wrong == 0 else "%d WRONG" % wrong))
    return wrong == 0


# main --------------------------------------------------------------------

def main():
    ap = argparse.ArgumentParser(description="Chi-square test of the HW3 warps (the Warps handout).")
    ap.add_argument("warp", nargs="?", help="a warp (%s), or all" % ", ".join(WARPS))
    ap.add_argument("param", nargs="?", type=float, help="uniform_cone's cosThetaMax or phong's exponent")
    ap.add_argument("-n", type=int, help="samples (default 1,000,000; 1,024 with --view or --dump)")
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--rt", help="the gauchoRT executable")
    ap.add_argument("--input", help="a saved dump of gauchoRT --warptest instead of running rt")
    ap.add_argument("--outdir", default="warptest", help="where the histogram PNGs go (default warptest/)")
    ap.add_argument("--alpha", type=float, default=0.01, help="significance level (default 0.01)")
    ap.add_argument("--selftest", action="store_true", help="test the test on warps written here")
    ap.add_argument("--sampler", help="the renderer's sampler for the dump (its default, independent, unless given)")
    ap.add_argument("--scatter", action="store_true",
                    help="draw the points on the warp's domain instead of testing them (to see a sampler's strata)")
    ap.add_argument("--view", action="store_true",
                    help="draw the points as red spheres on the warp's domain with your renderer, the scene WarpView")
    ap.add_argument("--dump", action="store_true", help="write the dump WarpView reads to warptest/, and no render")
    args = ap.parse_args()
    viewing = args.view or args.dump
    if args.n is None:
        args.n = 1024 if viewing else 1000000
    if viewing and not args.sampler:
        args.sampler = "stratified"  # an even set of points, on which a warp's clump, seam or hole stands out

    if args.selftest:
        return 0 if selftest(args) else 1
    if not args.warp:
        ap.error("name a warp, all, or --selftest")

    if args.warp == "all":
        if args.input:
            ap.error("--input holds one warp's dump; name the warp")
        jobs = [(w, DEFAULT_PARAM.get(w)) for w in WARPS]
    else:
        param = args.param
        if args.warp in DEFAULT_PARAM and param is None:
            param = DEFAULT_PARAM[args.warp]
        if args.warp not in DEFAULT_PARAM:
            param = None
        jobs = [(args.warp, param)]

    rt = None if args.input and not args.view else (os.path.abspath(args.rt) if args.rt else find_rt())
    failed = 0
    for warp, param in jobs:
        d = make_domain(warp, param)
        if args.input:
            with open(args.input, "rb") as f:
                data = parse_dump(f.read())
        else:
            data = dump_from_rt(rt, warp, args.n, param, args.seed, args.sampler)
        label = warp + ("" if param is None else " %g" % param)
        if viewing:
            write_dump(warp, data)
            where = render_view(rt, warp, args.outdir) if args.view else os.path.join("warptest", warp + ".txt")
            print("%-24s N = %d  %s: %s" % (label, len(data), "view" if args.view else "dump", where))
            continue
        if args.scatter:
            png = png_name(args.outdir, warp, param, "_scatter" + ("_" + args.sampler if args.sampler else ""))
            write_scatter(png, d, data[:, :3])
            print("%-24s N = %d  scatter: %s" % (label, len(data), png))
            continue
        if args.sampler and args.sampler != "independent":
            print("%-24s note: the test assumes independent points; a %s dump is too even for it (p near 1)"
                  % (label, args.sampler))
        if not run_test(d, data, args.alpha, png_name(args.outdir, warp, param)):
            failed += 1
    if len(jobs) > 1 and not viewing and not args.scatter:
        print("%d of %d warps pass" % (len(jobs) - failed, len(jobs)))
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
