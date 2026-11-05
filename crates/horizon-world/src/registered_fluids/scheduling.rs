//! A separate tick queue for registered fluid IDs. Vanilla enum discriminants
//! never enter this queue, and saved chunks use the existing named NBT codec.
use super::{RegisteredFluidId, RegisteredFluidTick, RegisteredFluids};
use crate::{compression::invalid, nbt::Tag};
use horizon_core::{
    position::{BlockPos, ChunkPos},
    ticks::{
        ChunkTicks, LevelTicks, SavedTick, ScheduleResult, ScheduledTick, TickPriority, TickTypeId,
    },
};
use std::io;

pub struct RegisteredFluidScheduler<'a, 'r> {
    fluids: &'a RegisteredFluids<'r>,
    queue: LevelTicks,
    sub_tick: i64,
    collecting: bool,
}
impl<'a, 'r> RegisteredFluidScheduler<'a, 'r> {
    pub fn new(fluids: &'a RegisteredFluids<'r>) -> Self {
        Self {
            fluids,
            queue: LevelTicks::default(),
            sub_tick: 0,
            collecting: false,
        }
    }
    fn idle(&self) -> io::Result<()> {
        if self.collecting {
            Err(invalid(
                "cannot change or save chunk lifecycle during a fluid tick batch",
            ))
        } else {
            Ok(())
        }
    }
    fn raw_id(&self, id: RegisteredFluidId) -> io::Result<TickTypeId> {
        self.fluids.check(id)?;
        Ok(TickTypeId(
            u32::try_from(id.index).map_err(|_| invalid("fluid tick ID overflow"))?,
        ))
    }
    fn fluid_id(&self, id: TickTypeId) -> io::Result<RegisteredFluidId> {
        let id = RegisteredFluidId {
            catalog: self.fluids.identity,
            index: id.0 as usize,
        };
        self.fluids.check(id)?;
        Ok(id)
    }
    /// Decode and validate the entire saved list before touching the queue.
    pub fn add_pending_chunk(&mut self, pos: ChunkPos, saved: &Tag) -> io::Result<()> {
        self.idle()?;
        let ticks = self
            .fluids
            .load_ticks(saved)?
            .into_iter()
            .map(|tick| {
                Ok(SavedTick {
                    kind: self.raw_id(tick.id)?,
                    pos: tick.pos,
                    delay: tick.delay,
                    priority: tick.priority,
                })
            })
            .collect::<io::Result<Vec<_>>>()?;
        self.queue
            .add_chunk(pos, ChunkTicks::from_saved(ticks))
            .map_err(io::Error::other)?;
        Ok(())
    }
    /// Start saved delays when a loaded chunk first becomes ticking, not when
    /// its bytes were loaded (a chunk may remain loaded but inactive).
    pub fn start_ticking_chunk(&mut self, pos: ChunkPos, game_time: i64) -> io::Result<()> {
        self.idle()?;
        if self.queue.chunk(pos).is_none() {
            return Err(invalid("fluid tick chunk is not loaded"));
        }
        self.queue.unpack_chunk(pos, game_time);
        Ok(())
    }
    /// Convenience for loading a chunk that is already eligible to tick.
    pub fn add_chunk(&mut self, pos: ChunkPos, saved: &Tag, game_time: i64) -> io::Result<()> {
        self.add_pending_chunk(pos, saved)?;
        self.start_ticking_chunk(pos, game_time)
    }
    pub fn save_chunk(&self, pos: ChunkPos, game_time: i64) -> io::Result<Option<Tag>> {
        self.idle()?;
        let Some(chunk) = self.queue.chunk(pos) else {
            return Ok(None);
        };
        let ticks = chunk
            .pack(game_time)
            .into_iter()
            .map(|tick| {
                Ok(RegisteredFluidTick {
                    id: self.fluid_id(tick.kind)?,
                    pos: tick.pos,
                    delay: tick.delay,
                    priority: tick.priority,
                })
            })
            .collect::<io::Result<Vec<_>>>()?;
        self.fluids.save_ticks(&ticks).map(Some)
    }
    /// Validate serialization before removal; failure leaves the queue intact.
    pub fn unload_chunk(&mut self, pos: ChunkPos, game_time: i64) -> io::Result<Option<Tag>> {
        let saved = self.save_chunk(pos, game_time)?;
        if saved.is_some() {
            self.queue.remove_chunk(pos);
        }
        Ok(saved)
    }
    pub fn schedule(
        &mut self,
        game_time: i64,
        pos: BlockPos,
        id: RegisteredFluidId,
        delay: i32,
        priority: TickPriority,
    ) -> io::Result<ScheduleResult> {
        let kind = self.raw_id(id)?;
        let sub_tick_order = self.sub_tick;
        self.sub_tick = self.sub_tick.wrapping_add(1);
        Ok(self.queue.schedule(ScheduledTick {
            kind,
            pos,
            trigger_tick: game_time.wrapping_add(i64::from(delay)),
            priority,
            sub_tick_order,
        }))
    }
    pub fn begin_tick(
        &mut self,
        game_time: i64,
        limit: i32,
        can_tick: impl FnMut(ChunkPos) -> bool,
    ) -> io::Result<()> {
        if limit < 0 {
            return Err(invalid("negative fluid tick limit"));
        }
        self.queue
            .begin_tick(game_time, limit, can_tick)
            .map_err(io::Error::other)?;
        self.collecting = true;
        Ok(())
    }
    /// Pop immediately before invoking registered flow. The caller must compare
    /// this identity against the current cell, as `RegisteredFlow::tick` does.
    pub fn next_due(&mut self) -> io::Result<Option<(BlockPos, RegisteredFluidId)>> {
        self.queue
            .next_tick()
            .map_err(io::Error::other)?
            .map(|tick| Ok((tick.pos, self.fluid_id(tick.kind)?)))
            .transpose()
    }
    pub fn finish_tick(&mut self) -> io::Result<()> {
        self.queue.finish_tick().map_err(io::Error::other)?;
        self.collecting = false;
        Ok(())
    }
    pub fn count(&self) -> usize {
        self.queue.count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{registered_fluids::LiquidFamily, registry::BlockRegistry};
    use serde_json::json;
    fn blocks() -> BlockRegistry {
        let mut rows = Vec::new();
        let mut defaults = serde_json::Map::new();
        for name in ["test:a", "test:b"] {
            defaults.insert(name.into(), json!(rows.len()));
            for level in 0..16 {
                rows.push(json!([name,false,true,"",{"id":name,"properties":{"level":level.to_string()}}]));
            }
        }
        BlockRegistry::from_facts_json(
            json!({"states":rows,"defaults":defaults})
                .to_string()
                .as_bytes(),
        )
        .unwrap()
    }
    fn catalog(blocks: &BlockRegistry, reverse: bool) -> RegisteredFluids<'_> {
        let names = if reverse { ["b", "a"] } else { ["a", "b"] };
        RegisteredFluids::new(
            blocks,
            names.map(|name| LiquidFamily {
                source: format!("test:{name}"),
                flowing: format!("test:flowing_{name}"),
                block: format!("test:{name}"),
            }),
        )
        .unwrap()
    }
    #[test]
    fn schedules_distinct_ids_and_first_schedule_wins() {
        let blocks = blocks();
        let fluids = catalog(&blocks, false);
        let mut ticks = RegisteredFluidScheduler::new(&fluids);
        let pos = BlockPos::new(0, 1, 0);
        let a = fluids.id("test:a").unwrap();
        let b = fluids.id("test:b").unwrap();
        assert_eq!(
            ticks.schedule(0, pos, a, 5, TickPriority::Normal).unwrap(),
            ScheduleResult::MissingChunk
        );
        ticks.add_chunk(pos.chunk(), &Tag::List(vec![]), 0).unwrap();
        assert_eq!(
            ticks.schedule(0, pos, a, 5, TickPriority::Normal).unwrap(),
            ScheduleResult::Added
        );
        assert_eq!(
            ticks.schedule(0, pos, a, 1, TickPriority::High).unwrap(),
            ScheduleResult::Duplicate
        );
        assert_eq!(
            ticks.schedule(0, pos, b, 5, TickPriority::High).unwrap(),
            ScheduleResult::Added
        );
        ticks.begin_tick(4, 10, |_| true).unwrap();
        assert_eq!(ticks.next_due().unwrap(), None);
        ticks.finish_tick().unwrap();
        ticks.begin_tick(5, 10, |_| true).unwrap();
        assert_eq!(ticks.next_due().unwrap(), Some((pos, b)));
        assert_eq!(ticks.next_due().unwrap(), Some((pos, a)));
        assert_eq!(ticks.next_due().unwrap(), None);
        ticks.finish_tick().unwrap();
    }
    #[test]
    fn chunk_save_reload_retains_names_timing_and_positions() {
        let blocks = blocks();
        let fluids = catalog(&blocks, false);
        let other = catalog(&blocks, true);
        let pos = BlockPos::new(-16, 64, 32);
        let id = fluids.id("test:flowing_a").unwrap();
        let mut ticks = RegisteredFluidScheduler::new(&fluids);
        ticks
            .add_chunk(pos.chunk(), &Tag::List(vec![]), 100)
            .unwrap();
        ticks
            .schedule(100, pos, id, 20, TickPriority::Normal)
            .unwrap();
        let saved = ticks.unload_chunk(pos.chunk(), 105).unwrap().unwrap();
        assert_eq!(ticks.count(), 0);
        let mut loaded = RegisteredFluidScheduler::new(&other);
        loaded.add_chunk(pos.chunk(), &saved, 1000).unwrap();
        loaded.begin_tick(1014, 10, |_| true).unwrap();
        assert_eq!(loaded.next_due().unwrap(), None);
        loaded.finish_tick().unwrap();
        loaded.begin_tick(1015, 10, |_| true).unwrap();
        assert_eq!(
            loaded.next_due().unwrap(),
            Some((pos, other.id("test:flowing_a").unwrap()))
        );
        loaded.finish_tick().unwrap();
        assert!(
            loaded
                .schedule(1000, pos, id, 5, TickPriority::Normal)
                .is_err()
        );
    }
    #[test]
    fn saved_delay_starts_when_loaded_chunk_becomes_ticking() {
        let blocks = blocks();
        let fluids = catalog(&blocks, false);
        let mut ticks = RegisteredFluidScheduler::new(&fluids);
        let pos = BlockPos::new(0, 1, 0);
        let id = fluids.id("test:a").unwrap();
        let saved = fluids
            .save_ticks(&[RegisteredFluidTick {
                id,
                pos,
                delay: 5,
                priority: TickPriority::Normal,
            }])
            .unwrap();
        ticks.add_pending_chunk(pos.chunk(), &saved).unwrap();
        ticks.begin_tick(100, 10, |_| true).unwrap();
        assert_eq!(ticks.next_due().unwrap(), None);
        ticks.finish_tick().unwrap();
        assert_eq!(ticks.save_chunk(pos.chunk(), 100).unwrap(), Some(saved));
        ticks.start_ticking_chunk(pos.chunk(), 1000).unwrap();
        ticks.begin_tick(1004, 10, |_| true).unwrap();
        assert_eq!(ticks.next_due().unwrap(), None);
        ticks.finish_tick().unwrap();
        ticks.begin_tick(1005, 10, |_| true).unwrap();
        assert_eq!(ticks.next_due().unwrap(), Some((pos, id)));
        ticks.finish_tick().unwrap();
    }
    #[test]
    fn active_batches_cannot_drop_ticks_through_save_or_unload() {
        let blocks = blocks();
        let fluids = catalog(&blocks, false);
        let mut ticks = RegisteredFluidScheduler::new(&fluids);
        let pos = BlockPos::new(0, 1, 0);
        let id = fluids.id("test:a").unwrap();
        ticks.add_chunk(pos.chunk(), &Tag::List(vec![]), 0).unwrap();
        ticks.schedule(0, pos, id, 1, TickPriority::Normal).unwrap();
        ticks.begin_tick(1, 10, |_| true).unwrap();
        assert!(ticks.save_chunk(pos.chunk(), 1).is_err());
        assert!(ticks.unload_chunk(pos.chunk(), 1).is_err());
        assert!(ticks.finish_tick().is_err());
        assert_eq!(ticks.next_due().unwrap(), Some((pos, id)));
        ticks.finish_tick().unwrap();
        assert!(ticks.save_chunk(pos.chunk(), 1).unwrap().is_some());
    }
    #[test]
    fn paused_chunks_and_wrong_chunk_data_are_safe() {
        let blocks = blocks();
        let fluids = catalog(&blocks, false);
        let mut ticks = RegisteredFluidScheduler::new(&fluids);
        let pos = BlockPos::new(0, 1, 0);
        let id = fluids.id("test:a").unwrap();
        let saved = fluids
            .save_ticks(&[RegisteredFluidTick {
                id,
                pos,
                delay: 5,
                priority: TickPriority::Normal,
            }])
            .unwrap();
        assert!(ticks.add_chunk(ChunkPos::new(1, 0), &saved, 0).is_err());
        assert_eq!(ticks.count(), 0);
        ticks.add_chunk(pos.chunk(), &saved, 0).unwrap();
        ticks.begin_tick(5, 10, |_| false).unwrap();
        assert_eq!(ticks.next_due().unwrap(), None);
        ticks.finish_tick().unwrap();
        ticks.begin_tick(6, 10, |_| true).unwrap();
        assert_eq!(ticks.next_due().unwrap(), Some((pos, id)));
        ticks.finish_tick().unwrap();
    }
}
