//! Neighbor-derived water/lava currents and falling-fluid wall deflection.
use crate::fluid::{FluidKind, FluidState};
use horizon_core::{
    geometry::{Direction, Vec3},
    position::BlockPos,
};

/// Identity-preserving samples for native and registered fluid currents.
pub trait FlowSample: Copy {
    fn is_empty(self) -> bool;
    fn same_family(self, other: Self) -> bool;
    fn own_height(self) -> f32;
    fn falling(self) -> bool;
}
impl FlowSample for FluidState {
    fn is_empty(self) -> bool {
        self.kind() == FluidKind::Empty
    }
    fn same_family(self, other: Self) -> bool {
        self.kind().same_family(other.kind())
    }
    fn own_height(self) -> f32 {
        self.own_height()
    }
    fn falling(self) -> bool {
        self.falling()
    }
}
pub trait FluidFlowWorld<S: FlowSample = FluidState> {
    type Error;
    type Block: Copy;
    fn fluid(&mut self, position: BlockPos) -> Result<S, Self::Error>;
    fn block(&mut self, position: BlockPos) -> Result<Self::Block, Self::Error>;
    /// Resolved BLOCKS_FLUID_FLOW tag membership for the captured state.
    fn blocks_fluid_flow(&self, state: Self::Block) -> bool;
    /// Actual IceBlock class membership, including its subclasses.
    fn is_ice_class(&self, state: Self::Block) -> bool;
    fn is_face_sturdy(
        &mut self,
        state: Self::Block,
        position: BlockPos,
        direction: Direction,
    ) -> Result<bool, Self::Error>;
}
/// Native wall test reads both block and fluid before any early return. The
/// supplied face is the scan direction, rather than its opposite.
pub fn solid_face<W: FluidFlowWorld>(
    world: &mut W,
    kind: FluidKind,
    position: BlockPos,
    direction: Direction,
) -> Result<bool, W::Error> {
    let block = world.block(position)?;
    let fluid = world.fluid(position)?;
    if fluid.kind().same_family(kind) {
        return Ok(false);
    }
    if direction == Direction::Up {
        return Ok(true);
    }
    if world.is_ice_class(block) {
        return Ok(false);
    }
    world.is_face_sturdy(block, position, direction)
}
fn sample_solid_face<S: FlowSample, W: FluidFlowWorld<S>>(
    world: &mut W,
    state: S,
    position: BlockPos,
    direction: Direction,
) -> Result<bool, W::Error> {
    let block = world.block(position)?;
    let fluid = world.fluid(position)?;
    if fluid.same_family(state) {
        return Ok(false);
    }
    if direction == Direction::Up {
        return Ok(true);
    }
    if world.is_ice_class(block) {
        return Ok(false);
    }
    world.is_face_sturdy(block, position, direction)
}
fn affects_flow<S: FlowSample>(state: S, neighbor: S) -> bool {
    neighbor.is_empty() || neighbor.same_family(state)
}
const HORIZONTAL: [Direction; 4] = [
    Direction::North,
    Direction::East,
    Direction::South,
    Direction::West,
];
/// The height differences and directional products round as f32 before being
/// accumulated in f64. Falling currents normalize before and after the Y push.
pub fn flow_at<W: FluidFlowWorld>(
    world: &mut W,
    state: FluidState,
    position: BlockPos,
) -> Result<Vec3, W::Error> {
    flow_at_sample(world, state, position)
}
/// Shared current calculation; custom identities never alias water or lava.
pub fn flow_at_sample<S: FlowSample, W: FluidFlowWorld<S>>(
    world: &mut W,
    state: S,
    position: BlockPos,
) -> Result<Vec3, W::Error> {
    if state.is_empty() {
        return Ok(Vec3::ZERO);
    }
    let mut x = 0.0;
    let mut z = 0.0;
    for direction in HORIZONTAL {
        let [dx, dy, dz] = direction.normal();
        let p = position.offset(dx, dy, dz);
        let neighbor = world.fluid(p)?;
        if !affects_flow(state, neighbor) {
            continue;
        }
        let mut height = neighbor.own_height();
        let mut difference = 0.0f32;
        if height == 0.0 {
            let block = world.block(p)?;
            if !world.blocks_fluid_flow(block) {
                let below = world.fluid(p.offset(0, -1, 0))?;
                if affects_flow(state, below) {
                    height = below.own_height();
                    if height > 0.0 {
                        difference = state.own_height() - (height - 0.8888889f32);
                    }
                }
            }
        } else if height > 0.0 {
            difference = state.own_height() - height;
        }
        if difference != 0.0 {
            x += f64::from(dx as f32 * difference);
            z += f64::from(dz as f32 * difference);
        }
    }
    let mut result = Vec3::new(x, 0.0, z);
    if state.falling() {
        for direction in HORIZONTAL {
            let [dx, dy, dz] = direction.normal();
            let p = position.offset(dx, dy, dz);
            if sample_solid_face(world, state, p, direction)?
                || sample_solid_face(world, state, p.offset(0, 1, 0), direction)?
            {
                result = result.normalize().add_vector(Vec3::new(0.0, -6.0, 0.0));
                break;
            }
        }
    }
    Ok(result.normalize())
}

#[cfg(test)]
mod registered_sample_tests {
    use super::*;
    use std::convert::Infallible;

    #[derive(Clone, Copy)]
    struct Sample(u64, u8, bool);
    impl FlowSample for Sample {
        fn is_empty(self) -> bool {
            self.0 == 0
        }
        fn same_family(self, other: Self) -> bool {
            self.0 == other.0
        }
        fn own_height(self) -> f32 {
            self.1 as f32 / 9.0
        }
        fn falling(self) -> bool {
            self.2
        }
    }
    struct World {
        east: Sample,
        wall: bool,
    }
    impl FluidFlowWorld<Sample> for World {
        type Error = Infallible;
        type Block = bool;
        fn fluid(&mut self, p: BlockPos) -> Result<Sample, Infallible> {
            Ok(if p.x == 1 {
                self.east
            } else {
                Sample(41, 8, false)
            })
        }
        fn block(&mut self, _: BlockPos) -> Result<bool, Infallible> {
            Ok(self.wall)
        }
        fn blocks_fluid_flow(&self, block: bool) -> bool {
            block
        }
        fn is_ice_class(&self, _: bool) -> bool {
            false
        }
        fn is_face_sturdy(
            &mut self,
            block: bool,
            _: BlockPos,
            _: Direction,
        ) -> Result<bool, Infallible> {
            Ok(block)
        }
    }
    #[test]
    fn currents_preserve_custom_family_identity_and_falling_walls() {
        let origin = BlockPos::new(0, 0, 0);
        let mut world = World {
            east: Sample(41, 4, false),
            wall: false,
        };
        let flow = flow_at_sample(&mut world, Sample(41, 8, false), origin).unwrap();
        assert_eq!((flow.x, flow.y, flow.z), (1.0, 0.0, 0.0));
        world.east = Sample(42, 4, false);
        let flow = flow_at_sample(&mut world, Sample(41, 8, false), origin).unwrap();
        assert_eq!((flow.x, flow.y, flow.z), (0.0, 0.0, 0.0));
        world.wall = true;
        let flow = flow_at_sample(&mut world, Sample(41, 8, true), origin).unwrap();
        assert_eq!((flow.x, flow.y, flow.z), (0.0, -1.0, 0.0));
        let flow = flow_at_sample(&mut world, Sample(0, 0, true), origin).unwrap();
        assert_eq!((flow.x, flow.y, flow.z), (0.0, 0.0, 0.0));
    }
}
