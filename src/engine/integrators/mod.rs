mod cook;
mod whitted;

pub(crate) use cook::*;
pub(crate) use whitted::*;

pub(super) const RECAST_EPSILON: f32 = 1e-4;

pub(super) trait Integrator {
    /// Returns:
    ///  - contribs: (indexes, colors)
    ///  - children: [(index, weight, pos, dir)]
    fn integrate(
        world: &super::world::World,
        idxs: &[u32; 4],
        wghs: ultraviolet::Vec3x4,
        rays: &mut super::ray::Rayx4,
        depth: u16,
    ) -> (
        ([u32; 4], [ultraviolet::Vec3; 4]),
        smallvec::SmallVec<[(u32, ultraviolet::Vec3, ultraviolet::Vec3, ultraviolet::Vec3); 12]>,
    );
}
