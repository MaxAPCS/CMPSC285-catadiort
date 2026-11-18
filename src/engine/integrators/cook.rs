use crate::engine::integrators::Integrator;

pub struct Cook;

impl Integrator for Cook {
    #[inline]

    fn integrate(
        world: &crate::engine::world::World,
        idxs: &[u32; 4],
        wghs: ultraviolet::Vec3x4,
        rays: &mut crate::engine::ray::Rayx4,
        depth: u16,
    ) -> (
        ([u32; 4], [ultraviolet::Vec3; 4]),
        smallvec::SmallVec<[(u32, ultraviolet::Vec3, ultraviolet::Vec3, ultraviolet::Vec3); 12]>,
    ) {
        todo!()
    }
}
