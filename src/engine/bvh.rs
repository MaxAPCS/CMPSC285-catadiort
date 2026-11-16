#![cfg(feature = "bvh")]

use rayon::slice::ParallelSliceMut;
use smallvec::SmallVec;
use ultraviolet::Vec3;
use wide::{CmpGe, CmpLe, f32x4};

use super::{Primitive, ray};

const LEAF_SIZE: usize = 4;

pub(super) struct BVH<P: Primitive> {
    primitives: Box<[P]>,
    nodes: Vec<Node>,
}

impl<P: Primitive + Send> BVH<P> {
    pub fn create(primitives: Box<[P]>) -> Self {
        let mut primitives = primitives;
        let mut nodes = Vec::with_capacity(primitives.len().saturating_mul(2));
        if !primitives.is_empty() {
            let end = primitives.len();
            Self::build(&mut primitives, &mut nodes, 0, end);
        }
        Self { primitives, nodes }
    }

    fn build(primitives: &mut [P], nodes: &mut Vec<Node>, start: usize, end: usize) -> u32 {
        debug_assert!(start < end);

        let mut bounds = Bounds::empty();
        let mut centroid_bounds = Bounds::empty();
        for p in &primitives[start..end] {
            let (pmin, pmax) = p.bounds();
            bounds = bounds.union(Bounds {
                min: pmin,
                max: pmax,
            });
            centroid_bounds = centroid_bounds.expand(p.centroid());
        }

        let idx = nodes.len() as u32;
        nodes.push(Node {
            bounds,
            kind: NodeKind::Leaf {
                start: start as u32,
                len: (end - start) as u32,
            },
        });

        let leaves = end - start;
        if leaves <= LEAF_SIZE {
            return idx;
        }

        let extent = centroid_bounds.max - centroid_bounds.min;
        let axis = if extent.x >= extent.y && extent.x >= extent.z {
            0
        } else if extent.y >= extent.z {
            1
        } else {
            2
        };
        let midpoint = 0.5 * (centroid_bounds.min[axis] + centroid_bounds.max[axis]);

        // sort by centroids
        primitives[start..end].par_sort_by(|a, b| {
            a.centroid()[axis]
                .partial_cmp(&b.centroid()[axis])
                .unwrap_or(core::cmp::Ordering::Equal)
        });

        let mut split = primitives[start..end].partition_point(|p| p.centroid()[axis] < midpoint);
        // fall back
        if split == 0 || split == leaves {
            split = leaves / 2;
        }

        let lft = Self::build(primitives, nodes, start, start + split);
        let rgh = Self::build(primitives, nodes, start + split, end);
        nodes[idx as usize].kind = NodeKind::Internal { lft, rgh };
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
                    for prim in &self.primitives[start as usize..(start + len) as usize] {
                        prim.obstruct(ray, depth);
                    }
                }
                NodeKind::Internal { lft, rgh } => {
                    stack.push(lft);
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
            if node.bounds.intersectx4(rays).none() {
                continue;
            }
            match node.kind {
                NodeKind::Leaf { start, len } => {
                    for prim in &self.primitives[start as usize..(start + len) as usize] {
                        prim.intersect(rays, depth);
                    }
                }
                NodeKind::Internal { lft, rgh } => {
                    stack.push(lft);
                    stack.push(rgh);
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
    Leaf { start: u32, len: u32 },
    /// Child Indexes
    Internal { lft: u32, rgh: u32 },
}

#[derive(Clone, Copy)]
struct Bounds {
    min: Vec3,
    max: Vec3,
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
        (近 <= 远) && (0. < 远) && (近 < ray.max_t)
    }

    fn intersectx4(&self, ray: &ray::Rayx4) -> f32x4 {
        let (mut 近, mut 远) = (f32x4::splat(f32::NEG_INFINITY), f32x4::splat(f32::INFINITY));
        for axis in 0..3 {
            let tl = (self.min[axis] - ray.pos[axis]) / ray.dir[axis];
            let th = (self.max[axis] - ray.pos[axis]) / ray.dir[axis];
            近 = 近.max(tl.min(th));
            远 = 远.min(tl.max(th));
        }
        近.cmp_le(远) & 远.cmp_ge(f32x4::ZERO)
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
}
