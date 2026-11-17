#![cfg(feature = "bvh")]

use core::num::NonZeroU32;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use smallvec::SmallVec;
use std::sync::LazyLock;
use ultraviolet::Vec3;
use wide::{CmpGe, CmpLe, CmpLt, f32x4};

use super::{Primitive, ray};
use crate::{engine::wider, options::OPTIONS};

const LEAF_SIZE: usize = 4;
const BOX_PAD: f32 = 1.0 + 4.0 * f32::EPSILON;
const NUM_BINS: usize = 16;
static SAH: LazyLock<bool> = LazyLock::new(|| OPTIONS.read().unwrap().bvh == "sah");

pub(super) struct BVH<P: Primitive> {
    primitives: Box<[P]>,
    nodes: Vec<Node>,
}

impl<P: Primitive + Send> BVH<P> {
    pub fn create(mut primitives: Box<[P]>) -> Self {
        let len = primitives.len();
        let mut nodes = Vec::with_capacity(len.saturating_mul(2));
        if !primitives.is_empty() {
            let mut prim_geom: Box<[_]> = primitives
                .par_iter_mut()
                .map(|prim| (Bounds::from(prim.bounds()), prim.centroid()))
                .collect();
            Self::build(&mut primitives, &mut prim_geom, &mut nodes, 0, len);
        }
        Self { primitives, nodes }
    }

    fn build(
        primitives: &mut [P],
        prim_geom: &mut [(Bounds, Vec3)],
        nodes: &mut Vec<Node>,
        start: usize,
        end: usize,
    ) -> u32 {
        debug_assert!(start < end);

        let mut full_bounds = Bounds::empty();
        let mut centroid_bounds = Bounds::empty();
        for i in start..end {
            full_bounds = full_bounds.union(prim_geom[i].0);
            centroid_bounds = centroid_bounds.expand(prim_geom[i].1);
        }

        let leaves = end - start;
        let idx = nodes.len() as u32;
        nodes.push(Node {
            bounds: full_bounds,
            kind: NodeKind::Leaf {
                start: start as u32,
                len: unsafe { NonZeroU32::new_unchecked(leaves as _) },
            },
        });

        if leaves <= LEAF_SIZE {
            return idx;
        }

        let (axis, split) = split_primitive(
            &mut primitives[start..end],
            &mut prim_geom[start..end],
            &centroid_bounds,
        );

        let lft = Self::build(primitives, prim_geom, nodes, start, start + split);
        let rgh = Self::build(primitives, prim_geom, nodes, start + split, end);
        debug_assert_eq!(lft, idx + 1);
        nodes[idx as usize].kind = NodeKind::Internal {
            rgh,
            axis: Axis::from(axis),
        };
        idx
    }
}

impl<P: Primitive> Primitive for BVH<P> {
    #[inline]
    fn bounds(&self) -> (Vec3, Vec3) {
        self.nodes.first().map_or(
            (Vec3::broadcast(f32::NAN), Vec3::broadcast(f32::NAN)),
            |n| n.bounds.bounds(),
        )
    }

    #[inline]
    fn centroid(&self) -> Vec3 {
        self.nodes
            .first()
            .map_or(Vec3::broadcast(f32::NAN), |n| n.bounds.centroid())
    }

    fn obstruct(&self, ray: &mut ray::ShadowRay, depth: u16) {
        if self.nodes.is_empty() {
            return;
        }

        let mut stack: SmallVec<[u32; 64]> = SmallVec::from_slice(&[0]);
        while let Some(idx) = stack.pop() {
            let node = &self.nodes[idx as usize];
            if !node.bounds.intersect(ray) {
                continue;
            }
            match node.kind {
                NodeKind::Leaf { start, len } => {
                    let start = start as usize;
                    for prim in &self.primitives[start..start + len.get() as usize] {
                        prim.obstruct(ray, depth);
                    }
                }
                NodeKind::Internal { rgh, .. } => {
                    stack.push(idx + 1);
                    stack.push(rgh);
                }
            }
        }
    }

    fn intersect(&self, rays: &mut ray::Rayx4, depth: u16) {
        if self.nodes.is_empty() {
            return;
        }

        let mut stack: SmallVec<[u32; 64]> = SmallVec::from_slice(&[0]);
        while let Some(idx) = stack.pop() {
            let node = &self.nodes[idx as usize];
            let ixt = node.bounds.intersectx4(rays);
            if ixt.cmp_lt(rays.max_t).none() {
                continue;
            }
            match node.kind {
                NodeKind::Leaf { start, len } => {
                    let start = start as usize;
                    for prim in &self.primitives[start..start + len.get() as usize] {
                        prim.intersect(rays, depth);
                    }
                }
                NodeKind::Internal { rgh, axis } => {
                    if rays.dir[axis as usize].as_array_ref()[0] >= 0. {
                        stack.push(rgh);
                        stack.push(idx + 1);
                    } else {
                        stack.push(idx + 1);
                        stack.push(rgh);
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Node {
    kind: NodeKind,
    bounds: Bounds,
}

#[derive(Clone, Copy)]
enum NodeKind {
    /// Primitive Index Start & Length
    Leaf { start: u32, len: NonZeroU32 },
    /// Child Indexes
    Internal { rgh: u32, axis: Axis },
}

#[derive(Clone, Copy)]
struct Bounds {
    min: Vec3,
    max: Vec3,
}

#[repr(u8)]
#[derive(Clone, Copy)]
enum Axis {
    X = 0,
    Y = 1,
    Z = 2,
}

impl From<usize> for Axis {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::X,
            1 => Self::Y,
            2 => Self::Z,
            _ => panic!(),
        }
    }
}

impl Bounds {
    fn intersect(&self, ray: &ray::ShadowRay) -> bool {
        let (mut 近, mut 远) = (f32::NEG_INFINITY, f32::INFINITY);
        for axis in 0..3 {
            let tl = (self.min[axis] - ray.pos[axis]) / ray.dir[axis];
            let th = (self.max[axis] - ray.pos[axis]) / ray.dir[axis];
            近 = 近.max(tl.min(th));
            远 = 远.min(tl.max(th));
        }
        远 *= BOX_PAD;
        (近 <= 远) && (0. < 远) && (近 < ray.max_t)
    }

    /// Returns: near hit t, or NaN
    fn intersectx4(&self, ray: &ray::Rayx4) -> f32x4 {
        let (mut 近, mut 远) = (f32x4::splat(f32::NEG_INFINITY), f32x4::splat(f32::INFINITY));
        for axis in 0..3 {
            let tl = (self.min[axis] - ray.pos[axis]) / ray.dir[axis];
            let th = (self.max[axis] - ray.pos[axis]) / ray.dir[axis];
            近 = 近.max(tl.min(th));
            远 = 远.min(tl.max(th));
        }
        远 *= f32x4::splat(BOX_PAD);
        let m = 近.cmp_le(远) & 远.cmp_ge(f32x4::ZERO);
        m.blend(近, wider::f32x4_NAN)
    }

    #[inline]
    fn empty() -> Self {
        Self {
            min: Vec3::broadcast(f32::INFINITY),
            max: Vec3::broadcast(f32::NEG_INFINITY),
        }
    }

    #[inline]
    fn union(self, other: Self) -> Self {
        Self {
            min: self.min.min_by_component(other.min),
            max: self.max.max_by_component(other.max),
        }
    }

    #[inline]
    fn expand(self, p: Vec3) -> Self {
        Self {
            min: self.min.min_by_component(p),
            max: self.max.max_by_component(p),
        }
    }

    #[inline]
    fn bounds(&self) -> (Vec3, Vec3) {
        (self.min, self.max)
    }

    #[inline]
    fn centroid(&self) -> Vec3 {
        (self.min + self.max) / 2.
    }

    #[inline]
    fn surface(&self) -> f32 {
        let Vec3 { x, y, z } = self.max - self.min;
        2. * (x * y + y * z + z * x)
    }
}

impl From<(Vec3, Vec3)> for Bounds {
    fn from((min, max): (Vec3, Vec3)) -> Self {
        let m = min.abs().max_by_component(max.abs());
        let e = m * (16. * f32::EPSILON);
        Self {
            min: min - e,
            max: max + e,
        }
    }
}
#[inline]
fn split_primitive<P: Primitive>(
    primitives: &mut [P],
    prim_geom: &mut [(Bounds, Vec3)],
    centroid_bounds: &Bounds,
) -> (usize, usize) {
    if *SAH && let Some(split) = sah_split(primitives, prim_geom, centroid_bounds) {
        return split;
    }
    midpoint_split(primitives, prim_geom, centroid_bounds)
}

fn midpoint_split<P: Primitive>(
    primitives: &mut [P],
    prim_geom: &mut [(Bounds, Vec3)],
    centroid_bounds: &Bounds,
) -> (usize, usize) {
    let total = primitives.len();
    let centroid_extent = centroid_bounds.max - centroid_bounds.min;
    let axis = if centroid_extent.x >= centroid_extent.y && centroid_extent.x >= centroid_extent.z {
        0
    } else if centroid_extent.y >= centroid_extent.z {
        1
    } else {
        2
    };

    let midpoint = 0.5 * (centroid_bounds.min[axis] + centroid_bounds.max[axis]);
    let mut split =
        pair_partition_unstable_by(primitives, prim_geom, |_, geom| geom.1[axis] < midpoint);
    if split == 0 || split == total {
        split = total / 2;
    }
    (axis, split)
}

fn sah_split<P: Primitive>(
    primitives: &mut [P],
    prim_geom: &mut [(Bounds, Vec3)],
    centroid_bounds: &Bounds,
) -> Option<(usize, usize)> {
    let total = primitives.len() as u32;
    let centroid_extent = centroid_bounds.max - centroid_bounds.min;
    let axis = if centroid_extent.x >= centroid_extent.y && centroid_extent.x >= centroid_extent.z {
        0
    } else if centroid_extent.y >= centroid_extent.z {
        1
    } else {
        2
    };

    let min = centroid_bounds.min[axis];
    let extent = centroid_bounds.max[axis] - min;
    if extent <= 0.0 || !extent.is_finite() {
        return None;
    }

    let bin =
        |pos: Vec3| (((pos[axis] - min) * (NUM_BINS as f32 / extent)) as usize).min(NUM_BINS - 1);

    let mut bin_bounds = [Bounds::empty(); NUM_BINS];
    let mut bin_counts = [0u32; NUM_BINS];
    for (bounds, centroid) in prim_geom.iter() {
        let bin = bin(*centroid);
        bin_bounds[bin] = bin_bounds[bin].union(*bounds);
        bin_counts[bin] += 1;
    }

    let mut best_cost = f32::INFINITY;
    let mut split_bin = 0;

    let mut right_area = [0.; NUM_BINS];
    let mut right_acc = Bounds::empty();
    for i in (1..NUM_BINS).rev() {
        right_acc = right_acc.union(bin_bounds[i]);
        right_area[i] = right_acc.surface();
    }

    let mut left_count = 0;
    let mut left_acc = Bounds::empty();
    for i in 1..NUM_BINS {
        left_acc = left_acc.union(bin_bounds[i - 1]);
        left_count += bin_counts[i - 1];

        let right_count = total - left_count;
        if left_count == 0 || right_count == 0 {
            continue;
        }

        let cost = left_acc.surface() * left_count as f32 + right_area[i] * right_count as f32;
        if cost < best_cost {
            best_cost = cost;
            split_bin = i;
        }
    }

    if !best_cost.is_finite() {
        return None;
    }

    let split = pair_partition_unstable_by(primitives, prim_geom, |_, geom| {
        let bin = bin(geom.1);
        bin < split_bin
    });
    if split == 0 || split == total as _ {
        return None;
    }
    Some((axis, split))
}

fn pair_partition_unstable_by<T, U, F>(a: &mut [T], b: &mut [U], mut pred: F) -> usize
where
    F: FnMut(&T, &U) -> bool,
{
    debug_assert_eq!(a.len(), b.len(), "slices must be the same length");

    let mut lo = 0;
    let mut hi = a.len();
    while lo < hi {
        if pred(&a[lo], &b[lo]) {
            lo += 1;
        } else {
            hi -= 1;
            if lo < hi {
                // fix order
                a.swap(lo, hi);
                b.swap(lo, hi);
            }
        }
    }
    lo
}
