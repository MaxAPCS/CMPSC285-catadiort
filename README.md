# ECE/CS 285 HW3: a distributed ray tracer

## Build
`cargo build --release`

Many switches are gated behind compile flags. For brute-force, use `--no-default-features`. For each aov view, use the respective `-F aov___`.

Artifact can be found at [target/release/catadiort](target/release/catadiort)

Must be run with assets/ in working directory. Run `catadiort` with no arguments for help.

## Results

Platform: Linux x86_64

### Warps
All warps pass under all samplers.
```
disk                     N = 16960  chi2 = 1917.0  dof = 1986  p = 0.864  pdf error 4.0e-08  PASS
    histograms: warptest/disk.png
cosine_hemisphere        N = 16960  chi2 = 2214.5  dof = 2252  p = 0.71  pdf error 1.0e-07  PASS
    histograms: warptest/cosine_hemisphere.png
uniform_sphere           N = 16960  chi2 = 3200.4  dof = 3199  p = 0.49  pdf error 4.0e-08  PASS
    histograms: warptest/uniform_sphere.png
uniform_cone 0.8         N = 16960  chi2 = 3200.4  dof = 3199  p = 0.49  pdf error 6.3e-09  PASS
    histograms: warptest/uniform_cone_0.8.png
triangle                 N = 16960  chi2 = 1282.1  dof = 1274  p = 0.431  pdf error 0.0e+00  PASS
    histograms: warptest/triangle.png
phong 20                 N = 16960  chi2 = 1358.7  dof = 1364  p = 0.535  pdf error 1.7e-07  PASS
    histograms: warptest/phong_20.png
6 of 6 warps pass
```

### Area Lights
```
exact image: /tmp/area_light_test_5jueoid5/QuadLight_exact.pfm
QuadLight_4.pfm  RMSE 0.29258  mean difference 2.46e-01 (558.0 standard errors)  480 of 480 blocks beyond 4  FAIL
    the worst block starts at pixel (240, 144), 450.0 standard errors off
QuadLight_16.pfm  RMSE 0.29054  mean difference 2.46e-01 (571.8 standard errors)  480 of 480 blocks beyond 4  FAIL
    the worst block starts at pixel (240, 144), 910.1 standard errors off
QuadLight_64.pfm  RMSE 0.28994  mean difference 2.46e-01 (575.4 standard errors)  480 of 480 blocks beyond 4  FAIL
    the worst block starts at pixel (240, 144), 1556.2 standard errors off
QuadLight_256.pfm  RMSE 0.28982  mean difference 2.46e-01 (576.3 standard errors)  480 of 480 blocks beyond 4  FAIL
    the worst block starts at pixel (240, 144), 2067.1 standard errors off

  spp    RMSE      ratio to the previous (about 2 when unbiased)
     4   0.29258   
    16   0.29054   1.01
    64   0.28994   1.00
   256   0.28982   1.00

FAIL: the error does not behave like zero-mean noise
```

### Writeup
NEE is unbiased because of a strategic separation of concerns. All it adds is an incorporation of outside knowledge of the world to contribute bounces that would've otherwise been highly unlikely (lights far from specular reflection). The factor that makes this possible is its probability weighting- while finding unlikely directions, their contributions are still weighted by their probabilities, preserving the principles Monte Carlo integration relies on. Proper NEE also skips double-counting from secondary rays, in order to uphold conservation of energy, meaning each NEE hit fully represents that light's influence on this point.

The glossy lobe's weight is a complex approximation of a BRDF. It represents the energy that reaches a layer "under" the coat- the energy lost on this bounce. The two factors of the numerator are the incoming and outgoing loss, which is normalized by the average loss in the denominator. This amount is what albedo is intended to be multiplied by- the light that reaches its pigment. For the ambient branch, the incoming loss is the average loss as the ambient color is assumed to be arriving from every direction. This cancels the denominator.

Antialiasing is a primary scene of difference. Cook makes many samples per pixel, and is able to filter out the high frequencies Whitted misses. As a result, the center of the circle is a blurry grey instead of a stripy mess.
Depth of Field is also obvious- Cook's use of sampling means depth of field from many film and lens points is possible.
CornellBox also now has soft shadows.

## Footnotes

HW2's statement:
> This project in total was extremely challenging. I may have overscoped by attempting to complete the entire thing with SIMD, and spent a long time perfecting the architecture to ensure everything went smoothly. In particular, it was challenging to hunt down very specific bugs in NaN handling (which I used as a sentinel for empty lanes). I think that I had a good shot, but didn't end up having enough time to complete what I wanted to.

Once again, this has been extremely difficult. It was challenging working through debt from the previous week, and it also took a long time sifting through reference documents to find every little edge case. In particular, deciphering the illumination model was extraordinarily challenging, as the branching behavior of the ambient term was not clear (clear/opaque, coated/uncoated, smooth/rough), and the formulas for each were obfuscated and distant. I managed to recognize each pattern eventually, and I believe my implementation is now correct.

Whitted no longer matches for some scenes (regression).
The sampling order has not been preseved.
There is no environment light.
There is no caustic suppression nor specular term lobe filtering.

Time limits have a granularity of `RAY_CONCURRENCY`, currently 2^22. My parallel iterative structure does not complete a single pixel at a time, so perfect cutoff is impossible. However, progress is still reported, and is quite accurate- use that instead.

**HW2 Extra Credit:**
- Full SIMD Intersections (10)
- Mesh Instancing and Two-Layer BVH (10)
- Tinted Transparent Shadows (3)

**HW3 Extra Credit:**
- Sobol Sampler (5)
- Gaussian Pixel Filter (3) [compile flag `gaussian_weights`]

AI was used strictly to debug/check for errors. All code written by hand.
