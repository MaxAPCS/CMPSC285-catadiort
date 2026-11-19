use ultraviolet::{DVec3x4, DVec4x4, Mat4, Vec3, Vec3x4, Vec4x4};
use wide::{f32x4, f64x4, i32x4, u32x4};

#[allow(non_upper_case_globals)]
pub const f32x4_NAN: f32x4 = f32x4::new([f32::NAN; _]);

pub fn gaussian_weights(size: usize) -> Box<[f32]> {
    if size == 0 {
        return Box::new([1.]);
    }

    let center = size as f32 / 2.;
    let sigma = size as f32 / 6.;
    let neg_two_sig_sq_inv = -(2. * sigma * sigma).recip();

    let mut weights = vec![0.; size * size].into_boxed_slice();
    let mut sum = 0.0;
    for y in 0..size {
        for x in 0..size {
            let (dx, dy) = (x as f32 - center, y as f32 - center);
            let w = ((dx * dx + dy * dy) * neg_two_sig_sq_inv).exp();
            weights[y * size + x] = w;
            sum += w;
        }
    }
    let inv_sum = sum.recip();
    for w in weights.iter_mut() {
        *w *= inv_sum;
    }
    weights
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
pub fn copysign(vec: Vec3, sign: Vec3) -> Vec3 {
    if vec.dot(sign) >= 0. { vec } else { -vec }
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

pub struct Chunku32x4<const COUNT: usize, I: Iterator<Item = [u32; COUNT]>> {
    i: I,
}

impl<const COUNT: usize, I: Iterator<Item = [u32; COUNT]>> Iterator for Chunku32x4<COUNT, I> {
    type Item = [u32x4; COUNT];

    fn next(&mut self) -> Option<Self::Item> {
        let x = [
            self.i.next()?,
            self.i.next()?,
            self.i.next()?,
            self.i.next()?,
        ];
        Some(core::array::from_fn(|i| x.map(|e| e[i]).into()))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (l, r) = self.i.size_hint();
        (l / 4, r.map(|r| r / 4))
    }
}

impl<const COUNT: usize, I: Iterator<Item = [u32; COUNT]>> From<I> for Chunku32x4<COUNT, I> {
    fn from(i: I) -> Self {
        Self { i }
    }
}

impl<const COUNT: usize, I: Iterator<Item = [u32; COUNT]>> ExactSizeIterator
    for Chunku32x4<COUNT, I>
{
}
