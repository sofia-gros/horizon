//! Authoritative LevelAccess adapter. The initial safe surface accepts air or
//! the same registered liquid only; washing away blocks, containers and
//! fluid-specific reactions require explicit implementations, not lava aliases.
use super::RegisteredFlowWorld;
use crate::{
    compression::invalid,
    level::{Behaviours, LevelAccess, Spread, Ticks},
    registered_fluids::{
        RegisteredFluidId, RegisteredFluidState, RegisteredFluids,
        scheduling::RegisteredFluidScheduler,
    },
    registry::BlockStateId,
};
use horizon_core::{
    geometry::Direction,
    position::BlockPos,
    ticks::{ScheduleResult, TickPriority},
};
use std::io;

pub struct RegisteredLevel<'a, 's, 'r, L: LevelAccess + ?Sized> {
    behaviours: &'a Behaviours,
    level: &'a mut L,
    native_ticks: &'a mut Ticks,
    catalog: &'a RegisteredFluids<'r>,
    ticks: &'a mut RegisteredFluidScheduler<'s, 'r>,
    error: Option<io::Error>,
}
impl<'a, 's, 'r, L: LevelAccess + ?Sized> RegisteredLevel<'a, 's, 'r, L> {
    pub fn new(
        behaviours: &'a Behaviours,
        level: &'a mut L,
        native_ticks: &'a mut Ticks,
        catalog: &'a RegisteredFluids<'r>,
        ticks: &'a mut RegisteredFluidScheduler<'s, 'r>,
    ) -> io::Result<Self> {
        if !std::ptr::eq(catalog, ticks.catalog()) {
            return Err(invalid(
                "flow adapter and scheduler use different fluid catalogs",
            ));
        }
        Ok(Self {
            behaviours,
            level,
            native_ticks,
            catalog,
            ticks,
            error: None,
        })
    }
    /// Surface invariant violations instead of silently leaving unscheduled cells.
    pub fn finish(self) -> io::Result<()> {
        self.error.map_or(Ok(()), Err)
    }
}
impl<L: LevelAccess + ?Sized> RegisteredFlowWorld for RegisteredLevel<'_, '_, '_, L> {
    fn block(&self, pos: BlockPos) -> Option<BlockStateId> {
        (self.level.is_loaded(pos)
            && pos.y >= self.level.min_y()
            && pos.y < self.level.max_y()
            && self.ticks.has_chunk(pos.chunk()))
        .then(|| self.level.block_state(pos))
    }
    fn air(&self) -> BlockStateId {
        self.behaviours.air()
    }
    fn solid(&self, state: BlockStateId) -> bool {
        self.behaviours.is_solid(state)
    }
    fn can_hold_any(&self, _pos: BlockPos, state: BlockStateId) -> bool {
        self.behaviours.is_air(state) || self.catalog.from_block(state).is_some()
    }
    fn can_hold(&self, _pos: BlockPos, state: BlockStateId, id: RegisteredFluidId) -> bool {
        self.catalog.name(id).is_ok()
            && (self.behaviours.is_air(state)
                || self
                    .catalog
                    .from_block(state)
                    .is_some_and(|fluid| self.catalog.same_family(id, fluid.id()).unwrap_or(false)))
    }
    fn pass_wall(
        &self,
        _pos: BlockPos,
        state: BlockStateId,
        direction: Direction,
        target: BlockStateId,
    ) -> bool {
        self.behaviours.fluid_wall_open(direction, state, target)
    }
    fn can_replace(
        &self,
        pos: BlockPos,
        state: BlockStateId,
        id: RegisteredFluidId,
        _direction: Direction,
    ) -> bool {
        self.can_hold(pos, state, id)
    }
    fn allow_spread(&self, from: BlockPos, to: BlockPos) -> bool {
        self.level.allow_spread(Spread::Fluid, from, to)
    }
    fn allow_level_change(
        &self,
        pos: BlockPos,
        old: RegisteredFluidState,
        next: Option<RegisteredFluidState>,
    ) -> bool {
        self.catalog.name(old.id()).is_ok_and(|name| {
            self.level.allow_fluid_level_change(
                pos,
                name,
                i32::from(old.amount()),
                next.map_or(0, |state| i32::from(state.amount())),
            )
        })
    }
    fn replace(&mut self, pos: BlockPos, expected: BlockStateId, next: BlockStateId) -> bool {
        if self.block(pos) != Some(expected) {
            return false;
        }
        self.behaviours
            .set_block(self.level, self.native_ticks, pos, next, 3)
    }
    fn schedule(&mut self, pos: BlockPos, id: RegisteredFluidId, delay: i32) {
        match self
            .ticks
            .schedule(self.level.game_time(), pos, id, delay, TickPriority::Normal)
        {
            Ok(ScheduleResult::Added | ScheduleResult::Duplicate) => {}
            Ok(ScheduleResult::MissingChunk) => {
                self.error = Some(invalid(
                    "registered liquid changed in an unregistered chunk",
                ))
            }
            Err(error) => self.error = Some(error),
        }
    }
}
