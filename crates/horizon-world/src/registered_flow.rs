// Inverted spread semantics adapted from Mantle, Copyright (c) 2022 SlimeKnights.
// MIT License. Full notice and source are in THIRD_PARTY_NOTICES.md.
// Changed: explicit family direction in the existing host simulation and guards.
//! Flow transitions for registered liquids. Policy and world effects are
//! explicit: no registered liquid inherits vanilla lava behavior by default.
//! The ordering follows the existing `level::fluids` FlowingFluid algorithm.
pub mod level;

use crate::{
    compression::invalid,
    registered_fluids::{Catalog, RegisteredFluidId, RegisteredFluidState, RegisteredFluids},
    registry::BlockStateId,
};
use horizon_core::{geometry::Direction, position::BlockPos};
use std::{collections::HashMap, io};

const HORIZONTAL: [Direction; 4] = [
    Direction::North,
    Direction::South,
    Direction::West,
    Direction::East,
];
// Scope this cache to one spread decision: no result survives world mutation.
type SlopeCache = HashMap<(BlockPos, u8, u8), u16>;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerticalFlow {
    #[default]
    Down,
    Up,
}
impl VerticalFlow {
    pub fn direction(self) -> Direction {
        match self {
            Self::Down => Direction::Down,
            Self::Up => Direction::Up,
        }
    }
}
#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowPolicy {
    pub drop_off: u8,
    pub slope_distance: u8,
    pub tick_delay: i32,
    pub source_conversion: bool,
    #[serde(default)]
    pub vertical_direction: VerticalFlow,
}
/// The authoritative adapter owns loaded-world checks, occlusion, protection,
/// drops/interactions, and scheduling. `replace` must compare the expected
/// state and only return true after the world change has committed.
pub trait RegisteredFlowWorld {
    fn block(&self, pos: BlockPos) -> Option<BlockStateId>;
    fn air(&self) -> BlockStateId;
    fn solid(&self, state: BlockStateId) -> bool;
    /// Whether the block can contain any liquid, independently of its identity.
    fn can_hold_any(&self, pos: BlockPos, state: BlockStateId) -> bool;
    /// Whether it accepts this exact prospective source/flowing identity.
    fn can_hold(&self, pos: BlockPos, state: BlockStateId, fluid: RegisteredFluidId) -> bool;
    fn pass_wall(
        &self,
        from: BlockPos,
        state: BlockStateId,
        direction: Direction,
        target: BlockStateId,
    ) -> bool;
    fn can_replace(
        &self,
        pos: BlockPos,
        state: BlockStateId,
        fluid: RegisteredFluidId,
        direction: Direction,
    ) -> bool;
    fn allow_spread(&self, from: BlockPos, to: BlockPos) -> bool;
    fn allow_level_change(
        &self,
        pos: BlockPos,
        old: RegisteredFluidState,
        next: Option<RegisteredFluidState>,
    ) -> bool;
    fn replace(&mut self, pos: BlockPos, expected: BlockStateId, next: BlockStateId) -> bool;
    fn schedule(&mut self, pos: BlockPos, fluid: RegisteredFluidId, delay: i32);
}
pub struct RegisteredFlow<'a, 'r> {
    fluids: Catalog<'a, 'r>,
    policies: HashMap<RegisteredFluidId, FlowPolicy>,
}
impl<'a, 'r> RegisteredFlow<'a, 'r> {
    pub fn new(
        fluids: &'a RegisteredFluids<'r>,
        policies: impl IntoIterator<Item = (RegisteredFluidId, FlowPolicy)>,
    ) -> io::Result<Self> {
        Self::with_catalog(Catalog::Borrowed(fluids), policies)
    }
    pub fn new_shared(
        fluids: std::sync::Arc<RegisteredFluids<'r>>,
        policies: impl IntoIterator<Item = (RegisteredFluidId, FlowPolicy)>,
    ) -> io::Result<Self> {
        Self::with_catalog(Catalog::Shared(fluids), policies)
    }
    fn with_catalog(
        fluids: Catalog<'a, 'r>,
        policies: impl IntoIterator<Item = (RegisteredFluidId, FlowPolicy)>,
    ) -> io::Result<Self> {
        let mut result = Self {
            fluids,
            policies: HashMap::new(),
        };
        for (id, policy) in policies {
            let source = result.fluids.source(id)?;
            if !(1..=8).contains(&policy.drop_off)
                || policy.slope_distance > 8
                || policy.tick_delay <= 0
            {
                return Err(invalid("invalid registered liquid flow policy"));
            }
            if result.policies.insert(source, policy).is_some() {
                return Err(invalid("duplicate liquid flow policy"));
            }
        }
        Ok(result)
    }
    pub fn tick_delay(&self, id: RegisteredFluidId) -> io::Result<i32> {
        Ok(self.policy(id)?.tick_delay)
    }
    pub fn vertical_direction(&self, id: RegisteredFluidId) -> io::Result<VerticalFlow> {
        Ok(self.policy(id)?.vertical_direction)
    }
    fn policy(&self, id: RegisteredFluidId) -> io::Result<FlowPolicy> {
        self.policies
            .get(&self.fluids.source(id)?)
            .copied()
            .ok_or_else(|| invalid("liquid has no flow policy"))
    }
    fn same(&self, id: RegisteredFluidId, other: RegisteredFluidState) -> bool {
        self.fluids.same_family(id, other.id()).unwrap_or(false)
    }
    fn source_at(&self, id: RegisteredFluidId, state: BlockStateId) -> bool {
        self.fluids
            .from_block(state)
            .is_some_and(|other| other.is_source() && self.same(id, other))
    }
    fn flowing(&self, id: RegisteredFluidId, amount: u8, falling: bool) -> RegisteredFluidState {
        self.fluids
            .state(
                self.fluids.flowing(id).expect("validated fluid identity"),
                amount,
                falling,
            )
            .expect("validated fluid level")
    }
    fn new_liquid<W: RegisteredFlowWorld>(
        &self,
        world: &W,
        id: RegisteredFluidId,
        pos: BlockPos,
        state: BlockStateId,
        policy: FlowPolicy,
    ) -> Option<RegisteredFluidState> {
        let mut highest = 0;
        let mut sources = 0;
        for direction in HORIZONTAL {
            let Some(neighbor) = world.block(rel(pos, direction)) else {
                continue;
            };
            if let Some(fluid) = self.fluids.from_block(neighbor) {
                if self.same(id, fluid) && world.pass_wall(pos, state, direction, neighbor) {
                    highest = highest.max(fluid.amount());
                    sources += u8::from(fluid.is_source());
                }
            }
        }
        if sources >= 2 && policy.source_conversion {
            if let Some(support) = world.block(rel(pos, policy.vertical_direction.direction())) {
                if world.solid(support) || self.source_at(id, support) {
                    return self
                        .fluids
                        .state(self.fluids.source(id).ok()?, 8, false)
                        .ok();
                }
            }
        }
        let supply_direction = policy.vertical_direction.direction().opposite();
        if let Some(supply) = world.block(rel(pos, supply_direction)) {
            if self
                .fluids
                .from_block(supply)
                .is_some_and(|fluid| self.same(id, fluid))
                && world.pass_wall(pos, state, supply_direction, supply)
            {
                return Some(self.flowing(id, 8, true));
            }
        }
        let amount = highest.saturating_sub(policy.drop_off);
        (amount > 0).then(|| self.flowing(id, amount, false))
    }
    fn maybe_pass<W: RegisteredFlowWorld>(
        &self,
        world: &W,
        id: RegisteredFluidId,
        pos: BlockPos,
        state: BlockStateId,
        direction: Direction,
        target: BlockStateId,
    ) -> bool {
        !self.source_at(id, target)
            && world.can_hold_any(rel(pos, direction), target)
            && world.pass_wall(pos, state, direction, target)
    }
    fn pass<W: RegisteredFlowWorld>(
        &self,
        world: &W,
        id: RegisteredFluidId,
        pos: BlockPos,
        state: BlockStateId,
        direction: Direction,
        target: BlockStateId,
    ) -> bool {
        self.maybe_pass(world, id, pos, state, direction, target)
            && world.can_hold(
                rel(pos, direction),
                target,
                self.fluids.flowing(id).expect("validated fluid identity"),
            )
    }
    fn hole<W: RegisteredFlowWorld>(
        &self,
        world: &W,
        id: RegisteredFluidId,
        pos: BlockPos,
        state: BlockStateId,
        policy: FlowPolicy,
    ) -> bool {
        let direction = policy.vertical_direction.direction();
        let target_pos = rel(pos, direction);
        let Some(target) = world.block(target_pos) else {
            return false;
        };
        world.pass_wall(pos, state, direction, target)
            && (self
                .fluids
                .from_block(target)
                .is_some_and(|fluid| self.same(id, fluid))
                || (world.can_hold_any(target_pos, target)
                    && world.can_hold(
                        target_pos,
                        target,
                        self.fluids.flowing(id).expect("validated fluid identity"),
                    )))
    }
    fn slope<W: RegisteredFlowWorld>(
        &self,
        world: &W,
        id: RegisteredFluidId,
        pos: BlockPos,
        state: BlockStateId,
        distance: u8,
        from: Direction,
        policy: FlowPolicy,
        cache: &mut SlopeCache,
    ) -> u16 {
        let key = (pos, distance, from as u8);
        if let Some(&result) = cache.get(&key) {
            return result;
        }
        let mut lowest = 1000;
        for direction in HORIZONTAL {
            if direction == from {
                continue;
            }
            let next_pos = rel(pos, direction);
            let Some(next) = world.block(next_pos) else {
                continue;
            };
            if self.pass(world, id, pos, state, direction, next) {
                if self.hole(world, id, next_pos, next, policy) {
                    cache.insert(key, u16::from(distance));
                    return u16::from(distance);
                }
                if distance < policy.slope_distance {
                    lowest = lowest.min(self.slope(
                        world,
                        id,
                        next_pos,
                        next,
                        distance + 1,
                        direction.opposite(),
                        policy,
                        cache,
                    ));
                }
            }
        }
        cache.insert(key, lowest);
        lowest
    }
    /// Neighbor scheduling also occurs after pickup/removal, so flowing cells
    /// cannot remain permanently behind after their source is collected.
    pub fn changed<W: RegisteredFlowWorld>(&self, world: &mut W, pos: BlockPos) {
        for at in std::iter::once(pos).chain(
            [
                Direction::Down,
                Direction::Up,
                Direction::North,
                Direction::South,
                Direction::West,
                Direction::East,
            ]
            .map(|d| rel(pos, d)),
        ) {
            if let Some(fluid) = world
                .block(at)
                .and_then(|state| self.fluids.from_block(state))
            {
                if let Ok(policy) = self.policy(fluid.id()) {
                    world.schedule(at, fluid.id(), policy.tick_delay);
                }
            }
        }
    }
    fn commit<W: RegisteredFlowWorld>(
        &self,
        world: &mut W,
        pos: BlockPos,
        old: BlockStateId,
        next: BlockStateId,
    ) -> bool {
        if old == next || !world.replace(pos, old, next) {
            return false;
        }
        self.changed(world, pos);
        true
    }
    fn spread_to<W: RegisteredFlowWorld>(
        &self,
        world: &mut W,
        pos: BlockPos,
        target: BlockStateId,
        fluid: RegisteredFluidState,
        direction: Direction,
    ) -> bool {
        if !world.allow_spread(rel(pos, direction.opposite()), pos)
            || !world.can_replace(pos, target, fluid.id(), direction)
        {
            return false;
        }
        let next = self.fluids.block(fluid).expect("catalog-owned state");
        self.commit(world, pos, target, next)
    }
    fn sides<W: RegisteredFlowWorld>(
        &self,
        world: &mut W,
        pos: BlockPos,
        state: BlockStateId,
        fluid: RegisteredFluidState,
        policy: FlowPolicy,
    ) {
        if !fluid.falling() && fluid.amount() <= policy.drop_off {
            return;
        }
        let mut lowest = 1000;
        let mut targets = Vec::new();
        let mut slopes = SlopeCache::new();
        for direction in HORIZONTAL {
            let at = rel(pos, direction);
            let Some(target) = world.block(at) else {
                continue;
            };
            if !self.maybe_pass(world, fluid.id(), pos, state, direction, target) {
                continue;
            }
            let Some(next) = self.new_liquid(world, fluid.id(), at, target, policy) else {
                continue;
            };
            if !world.can_hold(at, target, next.id()) {
                continue;
            }
            let distance = if self.hole(world, fluid.id(), at, target, policy) {
                0
            } else {
                self.slope(
                    world,
                    fluid.id(),
                    at,
                    target,
                    1,
                    direction.opposite(),
                    policy,
                    &mut slopes,
                )
            };
            if distance < lowest {
                targets.clear();
            }
            if distance <= lowest {
                if world.can_replace(at, target, next.id(), direction) {
                    targets.push((direction, at, target, next));
                }
                lowest = distance;
            }
        }
        targets.sort_by_key(|(direction, _, _, _)| *direction as u8);
        for (direction, at, target, next) in targets {
            self.spread_to(world, at, target, next, direction);
        }
    }
    /// Execute only ticks whose saved identity still matches the cell. Stale
    /// ticks after block replacement never act on a different liquid family.
    pub fn tick<W: RegisteredFlowWorld>(
        &self,
        world: &mut W,
        pos: BlockPos,
        expected: RegisteredFluidId,
    ) -> io::Result<()> {
        let policy = self.policy(expected)?;
        let Some(mut state) = world.block(pos) else {
            return Ok(());
        };
        let Some(mut fluid) = self.fluids.from_block(state) else {
            return Ok(());
        };
        if fluid.id() != expected {
            return Ok(());
        }
        if !fluid.is_source() {
            let next = self.new_liquid(world, fluid.id(), pos, state, policy);
            if next != Some(fluid) {
                if !world.allow_level_change(pos, fluid, next) {
                    return Ok(());
                }
                let block = next
                    .map(|value| self.fluids.block(value).expect("catalog-owned state"))
                    .unwrap_or_else(|| world.air());
                if !self.commit(world, pos, state, block) {
                    return Ok(());
                }
                let Some(next) = next else {
                    return Ok(());
                };
                state = block;
                fluid = next;
            }
        }
        let direction = policy.vertical_direction.direction();
        let target_pos = rel(pos, direction);
        if let Some(target) = world.block(target_pos) {
            if self.maybe_pass(world, fluid.id(), pos, state, direction, target) {
                if let Some(next) = self.new_liquid(world, fluid.id(), target_pos, target, policy) {
                    if world.can_hold(target_pos, target, next.id())
                        && world.can_replace(target_pos, target, next.id(), direction)
                    {
                        self.spread_to(world, target_pos, target, next, direction);
                        let sources = HORIZONTAL
                            .into_iter()
                            .filter_map(|d| world.block(rel(pos, d)))
                            .filter(|&s| self.source_at(fluid.id(), s))
                            .count();
                        if sources >= 3 {
                            self.sides(world, pos, state, fluid, policy);
                        }
                        return Ok(());
                    }
                }
            }
        }
        if fluid.is_source() || !self.hole(world, fluid.id(), pos, state, policy) {
            self.sides(world, pos, state, fluid, policy);
        }
        Ok(())
    }
    /// Place exactly one source cell; inventory consumption belongs to the
    /// caller and is permitted only after true is returned.
    pub fn place_source<W: RegisteredFlowWorld>(
        &self,
        world: &mut W,
        pos: BlockPos,
        id: RegisteredFluidId,
    ) -> io::Result<bool> {
        let policy = self.policy(id)?;
        let source = self.fluids.source(id)?;
        let Some(old) = world.block(pos) else {
            return Ok(false);
        };
        if !world.can_hold_any(pos, old)
            || !world.can_hold(pos, old, source)
            || !world.can_replace(pos, old, source, policy.vertical_direction.direction())
        {
            return Ok(false);
        }
        let next = self.fluids.block(self.fluids.state(source, 8, false)?)?;
        Ok(self.commit(world, pos, old, next))
    }
    /// Flowing cells are never a full bucket. Returns the collected source
    /// identity only after the authoritative removal succeeds.
    pub fn pickup_source<W: RegisteredFlowWorld>(
        &self,
        world: &mut W,
        pos: BlockPos,
    ) -> io::Result<Option<RegisteredFluidId>> {
        let Some(old) = world.block(pos) else {
            return Ok(None);
        };
        let Some(fluid) = self.fluids.from_block(old) else {
            return Ok(None);
        };
        self.policy(fluid.id())?;
        if !fluid.is_source() {
            return Ok(None);
        }
        Ok(self
            .commit(world, pos, old, world.air())
            .then_some(fluid.id()))
    }
}
fn rel(pos: BlockPos, direction: Direction) -> BlockPos {
    let (x, y, z) = match direction {
        Direction::Down => (0, -1, 0),
        Direction::Up => (0, 1, 0),
        Direction::North => (0, 0, -1),
        Direction::South => (0, 0, 1),
        Direction::West => (-1, 0, 0),
        Direction::East => (1, 0, 0),
    };
    BlockPos::new(
        pos.x.wrapping_add(x),
        pos.y.wrapping_add(y),
        pos.z.wrapping_add(z),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registered_fluids::LiquidFamily;
    use crate::registry::BlockRegistry;
    use crate::{nbt::Tag, registered_fluids::scheduling::RegisteredFluidScheduler};
    use horizon_core::{position::ChunkPos, ticks::TickPriority};
    use serde_json::json;
    use std::cell::Cell;
    fn registry() -> BlockRegistry {
        let mut states = vec![
            json!(["minecraft:air", true, false, "", "minecraft:air"]),
            json!(["test:stone", false, false, "", "test:stone"]),
        ];
        let mut defaults = serde_json::Map::new();
        defaults.insert("minecraft:air".into(), json!(0));
        defaults.insert("test:stone".into(), json!(1));
        for name in ["test:one", "test:two"] {
            defaults.insert(name.into(), json!(states.len()));
            for level in 0..16 {
                states.push(json!([name,false,true,"",{"id":name,"properties":{"level":level.to_string()}}]));
            }
        }
        BlockRegistry::from_facts_json(
            json!({"states":states,"defaults":defaults})
                .to_string()
                .as_bytes(),
        )
        .unwrap()
    }
    fn catalog(registry: &BlockRegistry) -> RegisteredFluids<'_> {
        RegisteredFluids::new(
            registry,
            ["one", "two"].map(|name| LiquidFamily {
                source: format!("test:{name}"),
                flowing: format!("test:flowing_{name}"),
                block: format!("test:{name}"),
            }),
        )
        .unwrap()
    }
    fn policy() -> FlowPolicy {
        FlowPolicy {
            drop_off: 1,
            slope_distance: 4,
            tick_delay: 5,
            source_conversion: false,
            vertical_direction: VerticalFlow::Down,
        }
    }
    struct World<'a, 'r> {
        fluids: &'a RegisteredFluids<'r>,
        cells: HashMap<BlockPos, BlockStateId>,
        ticks: RegisteredFluidScheduler<'a, 'r>,
        game_time: i64,
        deny: Option<BlockPos>,
        floor: bool,
        acceptance: HashMap<BlockPos, RegisteredFluidId>,
        reads: Cell<usize>,
    }
    impl<'a, 'r> World<'a, 'r> {
        fn new(fluids: &'a RegisteredFluids<'r>) -> Self {
            let mut ticks = RegisteredFluidScheduler::new(fluids);
            for x in -1..=0 {
                for z in -1..=0 {
                    ticks
                        .add_chunk(ChunkPos::new(x, z), &Tag::List(vec![]), 0)
                        .unwrap();
                }
            }
            Self {
                fluids,
                cells: HashMap::new(),
                ticks,
                game_time: 0,
                deny: None,
                floor: true,
                acceptance: HashMap::new(),
                reads: Cell::new(0),
            }
        }
        fn run(&mut self, flow: &RegisteredFlow<'_, '_>) {
            for _ in 0..1000 {
                self.game_time += 5;
                self.ticks
                    .begin_tick(self.game_time, 65536, |_| true)
                    .unwrap();
                while let Some((pos, id)) = self.ticks.next_due().unwrap() {
                    flow.tick(self, pos, id).unwrap();
                }
                self.ticks.finish_tick().unwrap();
                if self.ticks.count() == 0 {
                    return;
                }
            }
            panic!("liquid did not settle within bounded fixture");
        }
    }
    impl RegisteredFlowWorld for World<'_, '_> {
        fn block(&self, pos: BlockPos) -> Option<BlockStateId> {
            self.reads.set(self.reads.get() + 1);
            if !(-12..=12).contains(&pos.x)
                || !(-12..=12).contains(&pos.z)
                || !(0..=4).contains(&pos.y)
            {
                return None;
            }
            Some(
                self.cells
                    .get(&pos)
                    .copied()
                    .unwrap_or(BlockStateId(u32::from(self.floor && pos.y == 0))),
            )
        }
        fn air(&self) -> BlockStateId {
            BlockStateId(0)
        }
        fn solid(&self, state: BlockStateId) -> bool {
            state == BlockStateId(1)
        }
        fn can_hold_any(&self, _pos: BlockPos, state: BlockStateId) -> bool {
            !self.solid(state)
        }
        fn can_hold(&self, pos: BlockPos, state: BlockStateId, id: RegisteredFluidId) -> bool {
            self.acceptance
                .get(&pos)
                .is_none_or(|accepted| *accepted == id)
                && (state == BlockStateId(0)
                    || self
                        .fluids
                        .from_block(state)
                        .is_some_and(|other| self.fluids.same_family(id, other.id()).unwrap()))
        }
        fn pass_wall(
            &self,
            _pos: BlockPos,
            state: BlockStateId,
            _direction: Direction,
            target: BlockStateId,
        ) -> bool {
            !self.solid(state) && !self.solid(target)
        }
        fn can_replace(
            &self,
            pos: BlockPos,
            state: BlockStateId,
            id: RegisteredFluidId,
            _direction: Direction,
        ) -> bool {
            let _ = pos;
            state == BlockStateId(0)
                || self
                    .fluids
                    .from_block(state)
                    .is_some_and(|other| self.fluids.same_family(id, other.id()).unwrap())
        }
        fn allow_spread(&self, _from: BlockPos, to: BlockPos) -> bool {
            self.deny != Some(to)
        }
        fn allow_level_change(
            &self,
            pos: BlockPos,
            _old: RegisteredFluidState,
            _next: Option<RegisteredFluidState>,
        ) -> bool {
            self.deny != Some(pos)
        }
        fn replace(&mut self, pos: BlockPos, old: BlockStateId, next: BlockStateId) -> bool {
            if self.deny == Some(pos) || self.block(pos) != Some(old) {
                return false;
            }
            self.cells.insert(pos, next);
            true
        }
        fn schedule(&mut self, pos: BlockPos, id: RegisteredFluidId, delay: i32) {
            assert_eq!(delay, 5);
            self.ticks
                .schedule(self.game_time, pos, id, delay, TickPriority::Normal)
                .unwrap();
        }
    }
    #[test]
    fn placement_flow_pickup_and_source_removal_drain_cycle() {
        let registry = registry();
        let fluids = catalog(&registry);
        let id = fluids.id("test:one").unwrap();
        let flow = RegisteredFlow::new(&fluids, [(id, policy())]).unwrap();
        let mut world = World::new(&fluids);
        let pos = BlockPos::new(0, 1, 0);
        assert!(flow.place_source(&mut world, pos, id).unwrap());
        world.run(&flow);
        let edge = BlockPos::new(7, 1, 0);
        let outside = BlockPos::new(8, 1, 0);
        assert_eq!(
            fluids
                .from_block(world.block(edge).unwrap())
                .unwrap()
                .amount(),
            1
        );
        assert_eq!(world.block(outside), Some(BlockStateId(0)));
        assert_eq!(flow.pickup_source(&mut world, edge).unwrap(), None);
        assert_eq!(flow.pickup_source(&mut world, pos).unwrap(), Some(id));
        world.run(&flow);
        assert!(
            world
                .cells
                .values()
                .all(|state| fluids.from_block(*state).is_none())
        );
    }
    #[test]
    fn downward_flow_precedes_horizontal_spread() {
        let registry = registry();
        let fluids = catalog(&registry);
        let id = fluids.id("test:one").unwrap();
        let flow = RegisteredFlow::new(&fluids, [(id, policy())]).unwrap();
        let mut world = World::new(&fluids);
        let pos = BlockPos::new(0, 3, 0);
        flow.place_source(&mut world, pos, id).unwrap();
        flow.tick(&mut world, pos, id).unwrap();
        assert!(
            fluids
                .from_block(world.block(BlockPos::new(0, 2, 0)).unwrap())
                .unwrap()
                .falling()
        );
        assert_eq!(world.block(BlockPos::new(1, 3, 0)), Some(BlockStateId(0)));
        world.run(&flow);
        assert!(
            fluids
                .from_block(world.block(BlockPos::new(1, 1, 0)).unwrap())
                .is_some()
        );
    }
    #[test]
    fn inverted_flow_rises_spreads_under_a_ceiling_and_drains_after_pickup() {
        let registry = registry();
        let fluids = catalog(&registry);
        let id = fluids.id("test:one").unwrap();
        let rules = FlowPolicy {
            vertical_direction: VerticalFlow::Up,
            ..policy()
        };
        let flow = RegisteredFlow::new(&fluids, [(id, rules)]).unwrap();
        let mut world = World::new(&fluids);
        for x in -12..=12 {
            for z in -12..=12 {
                world.cells.insert(BlockPos::new(x, 4, z), BlockStateId(1));
            }
        }
        let origin = BlockPos::new(0, 1, 0);
        flow.place_source(&mut world, origin, id).unwrap();
        flow.tick(&mut world, origin, id).unwrap();
        let rising = fluids
            .from_block(world.block(BlockPos::new(0, 2, 0)).unwrap())
            .unwrap();
        assert!(rising.falling()); // The persisted column flag is direction-neutral.
        assert_eq!(rising.amount(), 8);
        assert_eq!(world.block(BlockPos::new(1, 1, 0)), Some(BlockStateId(0)));
        world.run(&flow);
        assert!(
            fluids
                .from_block(world.block(BlockPos::new(1, 3, 0)).unwrap())
                .is_some()
        );
        assert_eq!(world.block(BlockPos::new(0, 4, 0)), Some(BlockStateId(1)));
        assert_eq!(
            flow.pickup_source(&mut world, BlockPos::new(0, 2, 0))
                .unwrap(),
            None
        );
        assert_eq!(flow.pickup_source(&mut world, origin).unwrap(), Some(id));
        world.run(&flow);
        assert!(
            world
                .cells
                .values()
                .all(|state| fluids.from_block(*state).is_none())
        );
    }
    #[test]
    fn upward_slope_chooses_ceiling_holes_and_conversion_uses_ceiling_support() {
        let registry = registry();
        let fluids = catalog(&registry);
        let id = fluids.id("test:one").unwrap();
        let mut rules = FlowPolicy {
            vertical_direction: VerticalFlow::Up,
            ..policy()
        };
        let flow = RegisteredFlow::new(&fluids, [(id, rules)]).unwrap();
        let mut world = World::new(&fluids);
        for x in -12..=12 {
            for z in -12..=12 {
                world.cells.insert(BlockPos::new(x, 3, z), BlockStateId(1));
            }
        }
        world.cells.insert(BlockPos::new(1, 3, 0), BlockStateId(0));
        let origin = BlockPos::new(0, 2, 0);
        flow.place_source(&mut world, origin, id).unwrap();
        flow.tick(&mut world, origin, id).unwrap();
        assert!(
            fluids
                .from_block(world.block(BlockPos::new(1, 2, 0)).unwrap())
                .is_some()
        );
        for at in [
            BlockPos::new(-1, 2, 0),
            BlockPos::new(0, 2, -1),
            BlockPos::new(0, 2, 1),
        ] {
            assert_eq!(world.block(at), Some(BlockStateId(0)));
        }
        rules.source_conversion = true;
        let flow = RegisteredFlow::new(&fluids, [(id, rules)]).unwrap();
        let mut world = World::new(&fluids);
        flow.place_source(&mut world, BlockPos::new(-1, 2, 0), id)
            .unwrap();
        flow.place_source(&mut world, BlockPos::new(1, 2, 0), id)
            .unwrap();
        world.cells.insert(BlockPos::new(0, 1, 0), BlockStateId(1));
        assert!(
            !flow
                .new_liquid(&world, id, origin, BlockStateId(0), rules)
                .unwrap()
                .is_source()
        );
        world.cells.insert(BlockPos::new(0, 3, 0), BlockStateId(1));
        assert!(
            flow.new_liquid(&world, id, origin, BlockStateId(0), rules)
                .unwrap()
                .is_source()
        );
    }
    #[test]
    fn vertical_direction_is_per_family_and_unloaded_upper_cells_are_not_written() {
        let registry = registry();
        let fluids = catalog(&registry);
        let rising = fluids.id("test:one").unwrap();
        let falling = fluids.id("test:two").unwrap();
        let flow = RegisteredFlow::new(
            &fluids,
            [
                (
                    rising,
                    FlowPolicy {
                        vertical_direction: VerticalFlow::Up,
                        ..policy()
                    },
                ),
                (falling, policy()),
            ],
        )
        .unwrap();
        let mut world = World::new(&fluids);
        for (id, x, target_y, untouched_y) in [(rising, 0, 3, 1), (falling, 5, 1, 3)] {
            let origin = BlockPos::new(x, 2, 0);
            flow.place_source(&mut world, origin, id).unwrap();
            flow.tick(&mut world, origin, id).unwrap();
            let fluid = fluids
                .from_block(world.block(BlockPos::new(x, target_y, 0)).unwrap())
                .unwrap();
            assert!(fluids.same_family(id, fluid.id()).unwrap());
            assert_eq!(
                world.block(BlockPos::new(x, untouched_y, 0)),
                Some(BlockStateId(0))
            );
        }
        let ceiling = BlockPos::new(0, 4, 0);
        flow.place_source(&mut world, ceiling, rising).unwrap();
        flow.tick(&mut world, ceiling, rising).unwrap();
        assert!(!world.cells.contains_key(&BlockPos::new(0, 5, 0)));
    }
    #[test]
    fn direct_spread_checks_computed_flowing_identity() {
        let registry = registry();
        let fluids = catalog(&registry);
        let source = fluids.id("test:one").unwrap();
        let flowing = fluids.flowing(source).unwrap();
        let flow = RegisteredFlow::new(&fluids, [(source, policy())]).unwrap();
        for (target, accepted, should_spread) in [
            (BlockPos::new(0, 2, 0), source, false),
            (BlockPos::new(0, 2, 0), flowing, true),
            (BlockPos::new(1, 1, 0), source, false),
            (BlockPos::new(1, 1, 0), flowing, true),
        ] {
            let mut world = World::new(&fluids);
            world.acceptance.insert(target, accepted);
            let origin = if target.y == 2 {
                BlockPos::new(0, 3, 0)
            } else {
                BlockPos::new(0, 1, 0)
            };
            flow.place_source(&mut world, origin, source).unwrap();
            flow.tick(&mut world, origin, source).unwrap();
            assert_eq!(
                fluids.from_block(world.block(target).unwrap()).is_some(),
                should_spread
            );
        }
    }
    #[test]
    fn slope_and_holes_query_flowing_not_origin_identity() {
        let registry = registry();
        let fluids = catalog(&registry);
        let source = fluids.id("test:one").unwrap();
        let flowing = fluids.flowing(source).unwrap();
        let flow = RegisteredFlow::new(&fluids, [(source, policy())]).unwrap();
        let mut world = World::new(&fluids);
        let origin = BlockPos::new(0, 1, 0);
        let target = BlockPos::new(1, 1, 0);
        let below = BlockPos::new(1, 0, 0);
        world.cells.insert(below, BlockStateId(0));
        world.acceptance.insert(target, flowing);
        world.acceptance.insert(below, flowing);
        assert!(flow.pass(
            &world,
            source,
            origin,
            BlockStateId(0),
            Direction::East,
            BlockStateId(0)
        ));
        assert!(flow.hole(&world, source, target, BlockStateId(0), policy()));
        world.acceptance.insert(target, source);
        world.acceptance.insert(below, source);
        assert!(!flow.pass(
            &world,
            source,
            origin,
            BlockStateId(0),
            Direction::East,
            BlockStateId(0)
        ));
        assert!(!flow.hole(&world, source, target, BlockStateId(0), policy()));
    }
    #[test]
    fn computed_source_conversion_uses_source_acceptance() {
        let registry = registry();
        let fluids = catalog(&registry);
        let source = fluids.id("test:one").unwrap();
        let mut rules = policy();
        rules.source_conversion = true;
        let flow = RegisteredFlow::new(&fluids, [(source, rules)]).unwrap();
        let mut world = World::new(&fluids);
        let west = BlockPos::new(-1, 1, 0);
        let east = BlockPos::new(1, 1, 0);
        let middle = BlockPos::new(0, 1, 0);
        flow.place_source(&mut world, west, source).unwrap();
        flow.place_source(&mut world, east, source).unwrap();
        world.acceptance.insert(middle, source);
        flow.tick(&mut world, west, source).unwrap();
        assert!(
            fluids
                .from_block(world.block(middle).unwrap())
                .unwrap()
                .is_source()
        );
    }
    #[test]
    fn nearest_downward_hole_wins_over_flat_spread() {
        let registry = registry();
        let fluids = catalog(&registry);
        let id = fluids.id("test:one").unwrap();
        let flow = RegisteredFlow::new(&fluids, [(id, policy())]).unwrap();
        let mut world = World::new(&fluids);
        world.cells.insert(BlockPos::new(1, 0, 0), BlockStateId(0));
        let pos = BlockPos::new(0, 1, 0);
        flow.place_source(&mut world, pos, id).unwrap();
        flow.tick(&mut world, pos, id).unwrap();
        assert!(
            fluids
                .from_block(world.block(BlockPos::new(1, 1, 0)).unwrap())
                .is_some()
        );
        for at in [
            BlockPos::new(-1, 1, 0),
            BlockPos::new(0, 1, -1),
            BlockPos::new(0, 1, 1),
        ] {
            assert_eq!(world.block(at), Some(BlockStateId(0)));
        }
    }
    #[test]
    fn extended_slope_finds_far_holes_without_reusing_stale_results() {
        let registry = registry();
        let fluids = catalog(&registry);
        let id = fluids.id("test:one").unwrap();
        for (distance, finds_hole) in [(4, false), (8, true)] {
            let mut rules = policy();
            rules.slope_distance = distance;
            let flow = RegisteredFlow::new(&fluids, [(id, rules)]).unwrap();
            let mut world = World::new(&fluids);
            let origin = BlockPos::new(0, 1, 0);
            let hole = BlockPos::new(9, 0, 0);
            world.cells.insert(hole, BlockStateId(0));
            flow.place_source(&mut world, origin, id).unwrap();
            flow.tick(&mut world, origin, id).unwrap();
            assert!(
                fluids
                    .from_block(world.block(BlockPos::new(1, 1, 0)).unwrap())
                    .is_some()
            );
            for at in [
                BlockPos::new(-1, 1, 0),
                BlockPos::new(0, 1, -1),
                BlockPos::new(0, 1, 1),
            ] {
                assert_eq!(
                    fluids.from_block(world.block(at).unwrap()).is_some(),
                    !finds_hole
                );
            }
            // Close the hole between ticks. Cached decisions must not survive.
            world.cells.insert(hole, BlockStateId(1));
            flow.tick(&mut world, origin, id).unwrap();
            assert!(
                fluids
                    .from_block(world.block(BlockPos::new(-1, 1, 0)).unwrap())
                    .is_some()
            );
        }
    }
    #[test]
    fn extended_slope_flat_search_is_bounded_and_excessive_policy_is_rejected() {
        let registry = registry();
        let fluids = catalog(&registry);
        let id = fluids.id("test:one").unwrap();
        let mut rules = policy();
        rules.slope_distance = 8;
        let flow = RegisteredFlow::new(&fluids, [(id, rules)]).unwrap();
        let mut world = World::new(&fluids);
        let origin = BlockPos::new(0, 1, 0);
        flow.place_source(&mut world, origin, id).unwrap();
        world.reads.set(0);
        flow.tick(&mut world, origin, id).unwrap();
        assert!(
            world.reads.get() < 50_000,
            "repeated cyclic searches must be cached"
        );
        for direction in HORIZONTAL {
            assert!(
                fluids
                    .from_block(world.block(rel(origin, direction)).unwrap())
                    .is_some()
            );
        }
        rules.slope_distance = 9;
        assert!(RegisteredFlow::new(&fluids, [(id, rules)]).is_err());
    }
    #[test]
    fn glass_drop_eight_falls_but_does_not_spread_across_the_floor() {
        let registry = registry();
        let fluids = catalog(&registry);
        let id = fluids.id("test:one").unwrap();
        let mut rules = policy();
        rules.drop_off = 8;
        rules.slope_distance = 8;
        let flow = RegisteredFlow::new(&fluids, [(id, rules)]).unwrap();
        let mut world = World::new(&fluids);
        let origin = BlockPos::new(0, 3, 0);
        flow.place_source(&mut world, origin, id).unwrap();
        world.run(&flow);
        for y in 1..=2 {
            let at = BlockPos::new(0, y, 0);
            assert!(
                fluids
                    .from_block(world.block(at).unwrap())
                    .unwrap()
                    .falling()
            );
            for direction in HORIZONTAL {
                assert_eq!(world.block(rel(at, direction)), Some(BlockStateId(0)));
            }
        }
        assert_eq!(flow.pickup_source(&mut world, origin).unwrap(), Some(id));
        world.run(&flow);
        for y in 1..=3 {
            assert_eq!(world.block(BlockPos::new(0, y, 0)), Some(BlockStateId(0)));
        }
    }
    #[test]
    fn protected_unloaded_and_solid_cells_never_consume_a_source() {
        let registry = registry();
        let fluids = catalog(&registry);
        let id = fluids.id("test:one").unwrap();
        let flow = RegisteredFlow::new(&fluids, [(id, policy())]).unwrap();
        let mut world = World::new(&fluids);
        let pos = BlockPos::new(0, 1, 0);
        world.deny = Some(pos);
        assert!(!flow.place_source(&mut world, pos, id).unwrap());
        assert_eq!(world.ticks.count(), 0);
        assert!(
            !flow
                .place_source(&mut world, BlockPos::new(0, 0, 0), id)
                .unwrap()
        );
        assert!(
            !flow
                .place_source(&mut world, BlockPos::new(13, 1, 0), id)
                .unwrap()
        );
        world.deny = None;
        assert!(flow.place_source(&mut world, pos, id).unwrap());
        world.deny = Some(pos);
        assert_eq!(flow.pickup_source(&mut world, pos).unwrap(), None);
        assert!(
            fluids
                .from_block(world.block(pos).unwrap())
                .unwrap()
                .is_source()
        );
    }
    #[test]
    fn distinct_liquids_and_stale_ticks_do_not_alias() {
        let registry = registry();
        let fluids = catalog(&registry);
        let a = fluids.id("test:one").unwrap();
        let b = fluids.id("test:two").unwrap();
        let flow = RegisteredFlow::new(&fluids, [(a, policy()), (b, policy())]).unwrap();
        let mut world = World::new(&fluids);
        let pos = BlockPos::new(0, 1, 0);
        assert!(flow.place_source(&mut world, pos, b).unwrap());
        assert!(!flow.place_source(&mut world, pos, a).unwrap());
        let before = world.ticks.count();
        flow.tick(&mut world, pos, a).unwrap();
        assert_eq!(world.ticks.count(), before);
        assert_eq!(flow.pickup_source(&mut world, pos).unwrap(), Some(b));
    }
    #[test]
    fn conversion_requires_explicit_policy_and_supported_sources() {
        let registry = registry();
        let fluids = catalog(&registry);
        let id = fluids.id("test:one").unwrap();
        let mut world = World::new(&fluids);
        let mut enabled = policy();
        enabled.source_conversion = true;
        let flow = RegisteredFlow::new(&fluids, [(id, enabled)]).unwrap();
        flow.place_source(&mut world, BlockPos::new(-1, 1, 0), id)
            .unwrap();
        flow.place_source(&mut world, BlockPos::new(1, 1, 0), id)
            .unwrap();
        let pos = BlockPos::new(0, 1, 0);
        world
            .cells
            .insert(pos, fluids.block(flow.flowing(id, 1, false)).unwrap());
        let expected = fluids.flowing(id).unwrap();
        flow.tick(&mut world, pos, expected).unwrap();
        assert!(
            fluids
                .from_block(world.block(pos).unwrap())
                .unwrap()
                .is_source()
        );
    }
    #[test]
    fn missing_foreign_and_unbounded_policies_are_rejected() {
        let registry = registry();
        let fluids = catalog(&registry);
        let other = catalog(&registry);
        let id = fluids.id("test:one").unwrap();
        assert!(RegisteredFlow::new(&fluids, [(other.id("test:one").unwrap(), policy())]).is_err());
        assert!(
            RegisteredFlow::new(
                &fluids,
                [(id, policy()), (fluids.flowing(id).unwrap(), policy())]
            )
            .is_err()
        );
        for invalid in [
            FlowPolicy {
                drop_off: 0,
                ..policy()
            },
            FlowPolicy {
                slope_distance: 9,
                ..policy()
            },
            FlowPolicy {
                tick_delay: 0,
                ..policy()
            },
        ] {
            assert!(RegisteredFlow::new(&fluids, [(id, invalid)]).is_err());
        }
        let flow = RegisteredFlow::new(&fluids, []).unwrap();
        let mut world = World::new(&fluids);
        assert!(
            flow.place_source(&mut world, BlockPos::new(0, 1, 0), id)
                .is_err()
        );
        assert!(world.cells.is_empty());
    }
}
