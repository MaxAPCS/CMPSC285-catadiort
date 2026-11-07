use ultraviolet::{DVec3x4, DVec4x4, Mat4, Vec3, Vec3x4, Vec3x8, Vec4x4, Vec4x8};
use wide::{CmpGt, CmpLt, f32x4, f32x8, f64x4, i32x4, u32x4};

#[allow(non_upper_case_globals)]
pub const f32x4_NAN: f32x4 = f32x4::new([f32::NAN; _]);
#[allow(non_upper_case_globals)]
pub const f64x4_NAN: f64x4 = f64x4::new([f64::NAN; _]);

#[inline]
pub fn extract_scale_sq(mat: &Mat4) -> Vec3 {
    let a = mat.as_array();

    // Column vectors of the upper-left 3×3 block:
    // col0 = (a[0], a[1], a[2])
    // col1 = (a[4], a[5], a[6])
    // col2 = (a[8], a[9], a[10])
    let sx = a[0] * a[0] + a[1] * a[1] + a[2] * a[2];
    let sy = a[4] * a[4] + a[5] * a[5] + a[6] * a[6];
    let sz = a[8] * a[8] + a[9] * a[9] + a[10] * a[10];

    Vec3::new(sx, sy, sz)
}

#[inline]
pub fn transform_pointx8(mat: &Mat4, p: &Vec3x8) -> Vec3x8 {
    let mut out = Vec4x8::zero();

    for (k, slot) in out.as_mut_array().iter_mut().enumerate() {
        *slot = p.x.mul_add(
            f32x8::splat(mat.cols[0][k]),
            p.y.mul_add(
                f32x8::splat(mat.cols[1][k]),
                p.z.mul_add(f32x8::splat(mat.cols[2][k]), f32x8::splat(mat.cols[3][k])),
            ),
        );
    }

    Vec3x8::from_homogeneous_point(out)
}

#[inline]
pub fn transform_pointx4(mat: &Mat4, p: &Vec3x4) -> Vec3x4 {
    let mut out = Vec4x4::zero();

    for (k, slot) in out.as_mut_array().iter_mut().enumerate() {
        *slot = p.x.mul_add(
            f32x4::splat(mat.cols[0][k]),
            p.y.mul_add(
                f32x4::splat(mat.cols[1][k]),
                p.z.mul_add(f32x4::splat(mat.cols[2][k]), f32x4::splat(mat.cols[3][k])),
            ),
        );
    }

    Vec3x4::from_homogeneous_point(out)
}

#[inline]
pub fn transform_pointx4d(mat: &Mat4, p: &Vec3x4) -> DVec3x4 {
    let p = cast_d(*p);
    let mut out = DVec4x4::zero();

    for (k, slot) in out.as_mut_array().iter_mut().enumerate() {
        *slot = p.x.mul_add(
            f64x4::splat(mat.cols[0][k] as _),
            p.y.mul_add(
                f64x4::splat(mat.cols[1][k] as _),
                p.z.mul_add(
                    f64x4::splat(mat.cols[2][k] as _),
                    f64x4::splat(mat.cols[3][k] as _),
                ),
            ),
        );
    }

    DVec3x4::from_homogeneous_point(out)
}

#[inline]
pub fn transform_vecx4(mat: &Mat4, v: Vec3x4) -> Vec3x4 {
    let mut out = Vec4x4::zero();

    for (k, slot) in out.as_mut_array().iter_mut().enumerate() {
        *slot = v.x.mul_add(
            f32x4::splat(mat.cols[0][k]),
            v.y.mul_add(
                f32x4::splat(mat.cols[1][k]),
                v.z * f32x4::splat(mat.cols[2][k]),
            ),
        );
    }

    Vec3x4::from_homogeneous_vector(out)
}

#[inline]
pub fn transform_vecx4d(mat: &Mat4, v: &Vec3x4) -> DVec3x4 {
    let v = cast_d(*v);
    let mut out = DVec4x4::zero();

    for (k, slot) in out.as_mut_array().iter_mut().enumerate() {
        *slot = v.x.mul_add(
            f64x4::splat(mat.cols[0][k] as _),
            v.y.mul_add(
                f64x4::splat(mat.cols[1][k] as _),
                v.z * f64x4::splat(mat.cols[2][k] as _),
            ),
        );
    }

    DVec3x4::from_homogeneous_vector(out)
}

/// Arguments:
/// - dir: normalized, either direction
/// - norm: normalized, away from surface
/// - eta: object ior / medium ior
/// Returns:
/// - dir_dot_norm: fresnel cosine term
/// - odir: normalized
pub fn refracted(dir: Vec3, mut norm: Vec3, mut eta: f32) -> (f32, Vec3) {
    let ondi = norm.dot(dir);

    let ndi = if ondi > 0. {
        norm = -norm;
        -ondi
    } else {
        eta = eta.recip();
        ondi
    };

    let k = 1.0 - eta * eta * (1.0 - ndi * ndi);
    if k < 0.0 {
        (ondi, const { Vec3::broadcast(f32::NAN) })
    } else {
        (ondi, dir * eta - (eta * ndi + k.sqrt()) * norm)
    }
}

/// Arguments:
/// - dir: normalized, either direction
/// - norm: normalized, away from surface
/// - eta: object ior / medium ior
/// Returns:
/// - dir_dot_norm: fresnel cosine term
/// - odir: normalized
pub fn refractedx4(dir: Vec3x4, norm: Vec3x4, eta: f32x4) -> (f32x4, Vec3x4) {
    let ondi = norm.dot(dir);

    let flip = ondi.cmp_gt(f32x4::ZERO);
    let ndi = flip.blend(-ondi, ondi);
    let norm = Vec3x4::blend(flip, -norm, norm);
    let eta = flip.blend(eta, eta.recip());

    let k = 1.0 - eta * eta * (1.0 - ndi * ndi);
    let tir = k.cmp_lt(f32x4::ZERO);
    let out = dir.mul_add(Vec3x4::broadcast(eta), -(eta * ndi + k.sqrt()) * norm);
    (
        ondi,
        Vec3x4::blend(tir, const { Vec3x4::broadcast(f32x4_NAN) }, out),
    )
}

#[inline]
pub fn cast_f32x4(a: u32x4) -> f32x4 {
    f32x4::from_i32x4(bytemuck::cast::<u32x4, i32x4>(a))
}

#[inline]
fn cast_d(a: Vec3x4) -> DVec3x4 {
    DVec3x4::from(a.as_array().map(cast_f64))
}

#[inline]
pub fn cast_s(a: DVec3x4) -> Vec3x4 {
    Vec3x4::from(a.as_array().map(cast_f32))
}

#[inline]
fn cast_f64(a: f32x4) -> f64x4 {
    f64x4::from(a.as_array_ref().map(|x| x as _))
}

#[inline]
pub fn cast_f32(a: f64x4) -> f32x4 {
    f32x4::from(a.as_array_ref().map(|x| x as _))
}

pub struct Accum4<T, U> {
    n: usize,
    idxs: [u32; 4],
    ones: [core::mem::MaybeUninit<T>; 4],
    twos: [core::mem::MaybeUninit<U>; 4],
}

impl<T, U> Accum4<T, U> {
    pub fn new() -> Self {
        Self {
            n: 0,
            idxs: [0; 4],
            ones: [const { core::mem::MaybeUninit::uninit() }; 4],
            twos: [const { core::mem::MaybeUninit::uninit() }; 4],
        }
    }

    /// Returns: is buffer full?
    pub fn push(&mut self, idx: u32, one: T, two: U) -> bool {
        debug_assert!(self.n < 4);
        self.idxs[self.n] = idx;
        self.ones[self.n].write(one);
        self.twos[self.n].write(two);
        self.n += 1;
        self.n == 4
    }

    pub fn pop(&mut self) -> ([u32; 4], [T; 4], [U; 4]) {
        debug_assert_eq!(self.n, 4);
        self.n = 0;
        let mut ones = [const { core::mem::MaybeUninit::uninit() }; 4];
        core::mem::swap(&mut self.ones, &mut ones);
        let mut twos = [const { core::mem::MaybeUninit::uninit() }; 4];
        core::mem::swap(&mut self.twos, &mut twos);
        (
            self.idxs,
            unsafe { core::mem::transmute_copy(&ones) },
            unsafe { core::mem::transmute_copy(&twos) },
        )
    }
}

impl<T: Copy, U: Clone> Accum4<T, U> {
    /// Flush: pad remaining slots by repeating the last value,
    /// then return the accumulated arrays. Resets the counter.
    pub fn flush(mut self, pad_one: T) -> Option<([u32; 4], [T; 4], [U; 4])> {
        if self.n == 0 {
            return None;
        }
        for empty in &mut self.ones[self.n..4] {
            empty.write(pad_one);
        }
        let last = self.twos[self.n - 1].as_ptr();
        for empty in &mut self.twos[self.n..4] {
            empty.write(unsafe { (*last).clone() });
        }
        self.n = 0;

        #[inline]
        unsafe fn assume_init_array<T, const N: usize>(
            arr: [core::mem::MaybeUninit<T>; N],
        ) -> [T; N] {
            unsafe { core::ptr::read(&arr as *const _ as *const [T; N]) }
        }

        Some((
            self.idxs,
            unsafe {
                assume_init_array(core::mem::replace(
                    &mut self.ones,
                    [const { core::mem::MaybeUninit::uninit() }; 4],
                ))
            },
            unsafe {
                assume_init_array(core::mem::replace(
                    &mut self.twos,
                    [const { core::mem::MaybeUninit::uninit() }; 4],
                ))
            },
        ))
    }
}

impl<T, U> Drop for Accum4<T, U> {
    fn drop(&mut self) {
        for i in 0..self.n {
            unsafe {
                self.ones[i].assume_init_drop();
                self.twos[i].assume_init_drop();
            }
        }
    }
}

pub struct Chunku32x4<I: Iterator<Item = (u32, u32)>> {
    i: I,
}

impl<I: Iterator<Item = (u32, u32)>> Iterator for Chunku32x4<I> {
    type Item = (u32x4, u32x4);

    fn next(&mut self) -> Option<Self::Item> {
        let x = [
            self.i.next()?,
            self.i.next()?,
            self.i.next()?,
            self.i.next()?,
        ];
        Some((x.map(|e| e.0).into(), x.map(|e| e.1).into()))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (l, r) = self.i.size_hint();
        (l / 4, r.map(|r| r / 4))
    }
}

impl<I: Iterator<Item = (u32, u32)>> From<I> for Chunku32x4<I> {
    fn from(i: I) -> Self {
        Self { i }
    }
}

impl<I: ExactSizeIterator<Item = (u32, u32)>> ExactSizeIterator for Chunku32x4<I> {}
