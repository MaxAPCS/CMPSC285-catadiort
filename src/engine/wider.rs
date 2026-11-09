use ultraviolet::{DVec3x4, DVec4x4, Mat4, Mat4x4, Vec3, Vec3x4, Vec3x8, Vec4x4, Vec4x8};
use wide::{CmpGt, f32x4, f32x8, f64x4, i32x4, u32x4};

#[allow(non_upper_case_globals)]
pub const f32x4_NAN: f32x4 = f32x4::new([f32::NAN; _]);

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

#[inline]
#[allow(non_snake_case)]
pub fn Mat4x4_splat(mat4: &Mat4) -> Mat4x4 {
    Mat4x4 {
        cols: mat4.cols.map(|c| c.into()),
    }
}

#[inline]
pub fn copysign(vec: Vec3, sign: Vec3) -> Vec3 {
    if vec.dot(sign) > 0. { vec } else { -vec }
}

#[inline]
pub fn copysignx4(vec: Vec3x4, sign: Vec3x4) -> Vec3x4 {
    Vec3x4::blend(vec.dot(sign).cmp_gt(f32x4::ZERO), vec, -vec)
}

#[inline]
pub fn cast_f32x4(a: u32x4) -> f32x4 {
    f32x4::from_i32x4(bytemuck::cast::<u32x4, i32x4>(a))
}

#[inline]
pub fn cast_d(a: Vec3x4) -> DVec3x4 {
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

#[inline]
pub fn cast_m32(a: f64x4) -> f32x4 {
    let bits = a.move_mask();
    f32x4::from([
        f32::from_bits(if bits & 1 != 0 { u32::MAX } else { 0 }),
        f32::from_bits(if bits & 2 != 0 { u32::MAX } else { 0 }),
        f32::from_bits(if bits & 4 != 0 { u32::MAX } else { 0 }),
        f32::from_bits(if bits & 8 != 0 { u32::MAX } else { 0 }),
    ])
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

    pub fn peek(&self) -> Option<(u32, &T, &U)> {
        if self.n == 0 {
            None
        } else {
            Some((
                self.idxs[self.n - 1],
                unsafe { self.ones[self.n - 1].assume_init_ref() },
                unsafe { self.twos[self.n - 1].assume_init_ref() },
            ))
        }
    }
}

impl<T: Copy, U: bytemuck::Zeroable> Accum4<T, U> {
    pub fn flush(&mut self, pad_one: T) -> Option<([u32; 4], [T; 4], [U; 4])> {
        if self.n == 0 {
            return None;
        }
        for empty in &mut self.ones[self.n..4] {
            empty.write(pad_one);
        }
        for empty in &mut self.twos[self.n..4] {
            empty.write(U::zeroed());
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
