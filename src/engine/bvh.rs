#![cfg(feature = "bvh")]

use bytemuck::Zeroable;
use morton_encoding::morton_encode;
use rayon::{
    iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator},
    slice::ParallelSliceMut,
};
use ultraviolet::Vec3;
use wide::{CmpLt, f32x4};

use crate::engine::{Primitive, ray};

pub(super) struct BVH<T: Primitive> {
    root: usize,
    nodes: Box<[Node]>,
    prims: Box<[T]>,
    bounds: (Vec3, Vec3),
}

impl<T: Primitive> Primitive for BVH<T> {
    fn bounds(&self) -> (Vec3, Vec3) {
        self.bounds
    }

    fn centroid(&self) -> Vec3 {
        (self.bounds.0 + self.bounds.1) / 2.
    }

    fn intersect(&self, rays: &mut ray::Rayx4, depth: u16) {
        let mut stack: smallvec::SmallVec<[_; 64]> =
            smallvec::smallvec![(self.root, f32::NEG_INFINITY, f32::INFINITY)];
        let mut best = f32x4::splat(f32::INFINITY);

        while let Some((idx, t0, _t1)) = stack.pop() {
            if best.cmp_lt(t0).all() {
                continue;
            }
            match self.nodes[idx] {
                Node::Inner(node) => {
                    let (tl0, tl1) = self.nodes[node.lftidx].intersects(rays);
                    let (tr0, tr1) = self.nodes[node.rghidx].intersects(rays);
                    let (tl0, tl1) = (tl0.as_array_ref(), tl1.as_array_ref());
                    let (tr0, tr1) = (tr0.as_array_ref(), tr1.as_array_ref());
                    let best = best.as_array_ref();
                    for i in 0..4 {
                        // farther first
                        if tl0[i] <= tr0[i] {
                            if tr0[i] <= tr1[i] && best[i] > tr0[i] {
                                stack.push((node.rghidx, tr0[i], tr1[i]));
                            }
                            if tl0[i] <= tl1[i] && best[i] > tl0[i] {
                                stack.push((node.lftidx, tl0[i], tl1[i]));
                            }
                        } else {
                            if tl0[i] <= tl1[i] && best[i] > tl0[i] {
                                stack.push((node.lftidx, tl0[i], tl1[i]));
                            }
                            if tr0[i] <= tr1[i] && best[i] > tr0[i] {
                                stack.push((node.rghidx, tr0[i], tr1[i]));
                            }
                        }
                    }
                }
                Node::Leaf(node) => {
                    for prim in &self.prims[node.firstidx..(node.firstidx + node.len)] {
                        prim.intersect(rays, depth);
                        best = rays.closest_t();
                    }
                }
            };
        }
    }

    fn obstruct(&self, ray: &ray::ShadowRay, depth: u16) -> bool {
        let mut stack: smallvec::SmallVec<[_; 64]> = smallvec::smallvec![self.root];

        while let Some(idx) = stack.pop() {
            match self.nodes[idx] {
                Node::Inner(node) => {
                    let (tl0, tl1) = self.nodes[node.lftidx].shadows(ray);
                    let (tr0, tr1) = self.nodes[node.rghidx].shadows(ray);
                    // farther first
                    if tl0 <= tr0 {
                        if tr0 <= tr1 {
                            stack.push(node.rghidx);
                        }
                        if tl0 <= tl1 {
                            stack.push(node.lftidx);
                        }
                    } else {
                        if tl0 <= tl1 {
                            stack.push(node.lftidx);
                        }
                        if tr0 <= tr1 {
                            stack.push(node.rghidx);
                        }
                    }
                }
                Node::Leaf(node) => {
                    for prim in &self.prims[node.firstidx..(node.firstidx + node.len)] {
                        if prim.obstruct(ray, depth) {
                            return true;
                        };
                    }
                }
            }
        }
        false
    }
}

impl<T: Primitive + Sync> BVH<T> {
    pub fn create(mut prims: Box<[T]>) -> Self {
        let n = prims.len();
        if n == 0 {
            return Self {
                root: 0,
                nodes: Box::new([]),
                prims,
                bounds: (Vec3::broadcast(f32::NAN), Vec3::broadcast(f32::NAN)),
            };
        }
        if n == 1 {
            let bounds = prims[0].bounds();
            return Self {
                root: 0,
                nodes: vec![Node::Leaf(LeafNode {
                    bounds,
                    firstidx: 0,
                    len: 1,
                })]
                .into_boxed_slice(),
                prims,
                bounds,
            };
        }

        let mut bounds = (
            Vec3::broadcast(f32::INFINITY),
            Vec3::broadcast(f32::NEG_INFINITY),
        );

        let (mut nodes, root) = {
            let mut things: Vec<_> = prims
                .iter()
                .map(|primitive| {
                    let b = primitive.bounds();
                    bounds = (
                        Vec3::min_by_component(bounds.0, b.0),
                        Vec3::max_by_component(bounds.1, b.1),
                    );
                    (b, 0)
                })
                .collect();

            let pbounds = (bounds.0 - Vec3::one(), bounds.1 + Vec3::one());

            things
                .par_iter_mut()
                .enumerate()
                .for_each(|(i, (_, code))| {
                    let p = (prims[i].centroid() - pbounds.0) / (pbounds.1 - pbounds.0);
                    *code = morton_encode(p.as_array().map(|c| (c * u16::MAX as f32) as u16))
                });

            let mut sortidx: Vec<_> = (0..n).collect();
            sortidx.par_sort_by_key(|&i| things[i].1);

            reorder(&mut prims, &sortidx);
            reorder(&mut things, &sortidx);

            let mut nodes = vec![Node::Leaf(LeafNode::zeroed()); 2 * n - 1].into_boxed_slice();
            let (leaves, internals) = nodes.split_at_mut(n);

            leaves.par_iter_mut().enumerate().for_each(|(i, nd)| {
                let Node::Leaf(nd) = nd else { unreachable!() };
                nd.firstidx = i;
                nd.len = 1;
                nd.bounds = things[i].0;
            });

            fn find_l(things: &[((Vec3, Vec3), u64)], i: usize, base: u32) -> usize {
                let mut lo = 0;
                let mut hi = i;
                while lo < hi {
                    let m = (lo + hi) / 2;
                    if delta(things[m].1, things[i].1) > base {
                        hi = m;
                    } else {
                        lo = m + 1;
                    }
                }
                lo
            }

            fn find_r(things: &[((Vec3, Vec3), u64)], i: usize, base: u32) -> usize {
                let mut lo = i + 1;
                let mut hi = things.len() - 1;
                while lo < hi {
                    let m = (lo + hi + 1) / 2;
                    if delta(things[i].1, things[m].1) >= base {
                        lo = m;
                    } else {
                        hi = m - 1;
                    }
                }
                lo
            }

            fn find_split(things: &[((Vec3, Vec3), u64)], a: usize, b: usize) -> usize {
                let thresh = delta(things[a].1, things[b].1);
                let mut lo = a;
                let mut hi = b;
                while lo < hi {
                    let m = (lo + hi) / 2;
                    if delta(things[a].1, things[m].1) <= thresh {
                        hi = m;
                    } else {
                        lo = m + 1;
                    }
                }
                if lo == a { a } else { lo - 1 }
            }

            internals.par_iter_mut().enumerate().for_each(|(i, nd)| {
                let base = delta(things[i].1, things[i + 1].1);
                let l = find_l(&things, i, base);
                let r = find_r(&things, i, base);
                *nd = Node::Inner(InnerNode {
                    bounds: (Vec3::broadcast(f32::NAN), Vec3::broadcast(f32::NAN)),
                    lftidx: if l == i {
                        l
                    } else {
                        n + find_split(&things, l, i)
                    },
                    rghidx: if i + 1 == r {
                        r
                    } else {
                        n + find_split(&things, i + 1, r)
                    },
                });
            });

            // leftmost argmin
            let root = n + unsafe {
                (0..n - 1)
                    .min_by_key(|&p| delta(things[p].1, things[p + 1].1))
                    .unwrap_unchecked()
            };

            (nodes, root)
        };

        fn compute_bounds(nodes: &mut [Node], root: usize) -> (Vec3, Vec3) {
            let mut stack: Vec<(usize, bool)> = vec![(root, false)];
            let mut bounds: Vec<(Vec3, Vec3)> = Vec::new();

            while let Some((idx, visited)) = stack.pop() {
                if !visited {
                    match nodes[idx] {
                        Node::Leaf(leaf) => bounds.push(leaf.bounds),
                        Node::Inner(inner) => {
                            stack.push((idx, true));
                            stack.push((inner.rghidx, false));
                            stack.push((inner.lftidx, false));
                        }
                    }
                } else {
                    // children have been processed, their bounds are on the stack
                    let rb = bounds.pop().unwrap();
                    let lb = bounds.pop().unwrap();
                    let b = (
                        Vec3::min_by_component(lb.0, rb.0),
                        Vec3::max_by_component(lb.1, rb.1),
                    );
                    if let Node::Inner(inner) = &mut nodes[idx] {
                        inner.bounds = b;
                    }
                    bounds.push(b);
                }
            }

            bounds.pop().unwrap()
        }

        compute_bounds(&mut nodes, root);

        Self {
            root,
            nodes,
            prims,
            bounds,
        }
    }
}

/// Returns: number of identical leading bits
#[inline]
fn delta(a: u64, b: u64) -> u32 {
    (a ^ b).leading_zeros()
}

fn reorder<T>(items: &mut [T], perm: &[usize]) {
    debug_assert_eq!(items.len(), perm.len());
    let mut visited = vec![false; items.len()];
    for i in 0..items.len() {
        if visited[i] || perm[i] == i {
            visited[i] = true;
            continue;
        }
        let mut j = i;
        while perm[j] != i {
            items.swap(j, perm[j]);
            visited[j] = true;
            j = perm[j];
        }
        visited[j] = true;
    }
}

#[derive(Clone, Copy)]
enum Node {
    Inner(InnerNode),
    Leaf(LeafNode),
}

impl Node {
    const fn bounds(&self) -> (Vec3, Vec3) {
        match self {
            Node::Inner(inner_node) => inner_node.bounds,
            Node::Leaf(leaf_node) => leaf_node.bounds,
        }
    }

    fn intersects(&self, ray: &ray::Rayx4) -> (f32x4, f32x4) {
        let (min, max) = self.bounds();
        let (mut 近, mut 远) = (f32x4::splat(f32::NEG_INFINITY), f32x4::splat(f32::INFINITY));
        for axis in 0..3 {
            let tl = (min[axis] - ray.pos[axis]) / ray.dir[axis];
            let th = (max[axis] - ray.pos[axis]) / ray.dir[axis];
            近 = 近.max(tl.min(th));
            远 = 远.min(tl.max(th));
        }
        (近, 远)
    }

    fn shadows(&self, ray: &ray::ShadowRay) -> (f32, f32) {
        let (min, max) = self.bounds();
        let (mut 近, mut 远) = (f32::NEG_INFINITY, f32::INFINITY);
        for axis in 0..3 {
            let tl = (min[axis] - ray.pos[axis]) / ray.dir[axis];
            let th = (max[axis] - ray.pos[axis]) / ray.dir[axis];
            近 = 近.max(tl.min(th));
            远 = 远.min(tl.max(th));
        }
        (近, 远)
    }
}

#[derive(Clone, Copy)]
struct InnerNode {
    bounds: (Vec3, Vec3),
    lftidx: usize,
    rghidx: usize,
}

#[derive(Clone, Copy, bytemuck::Zeroable)]
struct LeafNode {
    bounds: (Vec3, Vec3),
    firstidx: usize,
    len: usize,
}
