//! Live authoritative registered-fluid scheduling. Rendering and item-use
//! adapters are separate integration gates; no mod declarations ship here.
use super::*;
use horizon_core::{position::ChunkPos, ticks::TickPriority};
use horizon_world::{
    nbt::Tag, registered_flow::level::RegisteredLevel,
    registered_fluids::scheduling::RegisteredFluidScheduler,
};

pub(super) type Scheduler = RegisteredFluidScheduler<'static, 'static>;
pub(super) const TICK_KEY: &str = "horizon:registered_fluid_ticks";
impl Game<'_> {
    pub(super) fn new_registered_fluid_scheduler(&self) -> Option<Scheduler> {
        self.res
            .rest
            .get()?
            .registered_fluids
            .as_ref()
            .map(|resources| Scheduler::new_shared(resources.catalog.clone()))
    }
    pub(super) fn register_registered_fluid_chunk(
        &mut self,
        key: (i32, i32),
        saved: Option<&horizon_client::chunk::ChunkExtras>,
    ) {
        let Some(ticks) = &mut self.registered_fluid_ticks else {
            return;
        };
        let empty = Tag::List(Vec::new());
        let tag = saved
            .and_then(|saved| saved.root.get(&TICK_KEY.into()))
            .unwrap_or(&empty);
        match ticks.add_pending_chunk(ChunkPos::new(key.0, key.1), tag) {
            Ok(()) => self.tick_chunks.wait(key),
            // The original root key is retained, including unknown/malformed data.
            Err(error) => self.error = Some(Box::new(error)),
        }
    }
    pub(super) fn tick_registered_fluids(&mut self) -> Result<()> {
        if self.remote.is_some() {
            return Ok(());
        }
        let Some(resources) = self.res.rest().registered_fluids.clone() else {
            return Ok(());
        };
        let Some(mut ticks) = self.registered_fluid_ticks.take() else {
            return Ok(());
        };
        self.registered_fluid_active = true;
        let result = (|| -> Result<()> {
            let changes = std::mem::take(&mut *self.registered_fluid_changes.borrow_mut());
            for (pos, time) in changes {
                for (x, y, z) in [
                    (0, 0, 0),
                    (0, -1, 0),
                    (0, 1, 0),
                    (0, 0, -1),
                    (0, 0, 1),
                    (-1, 0, 0),
                    (1, 0, 0),
                ] {
                    let at = BlockPos::new(
                        pos.x.wrapping_add(x),
                        pos.y.wrapping_add(y),
                        pos.z.wrapping_add(z),
                    );
                    if !self.world.loaded(at.x, at.z) || !ticks.has_chunk(at.chunk()) {
                        continue;
                    }
                    if let Some(fluid) = resources
                        .catalog
                        .from_block(self.world.block(at.x, at.y, at.z))
                    {
                        ticks.schedule(
                            time,
                            at,
                            fluid.id(),
                            resources.flow.tick_delay(fluid.id())?,
                            TickPriority::Normal,
                        )?;
                    }
                }
            }
            let levels = self.tickets.levels();
            ticks.begin_tick(self.world.game_time, 65536, |pos| {
                levels.loaded_ticking(pos.x, pos.z) && ticking_chunk(&self.world, pos.x, pos.z)
            })?;
            while let Some((pos, id)) = ticks.next_due()? {
                self.with_level(|behaviours, level, native| -> Result<()> {
                    let mut adapter = RegisteredLevel::new(
                        behaviours,
                        level,
                        native,
                        &resources.catalog,
                        &mut ticks,
                    )?;
                    resources.flow.tick(&mut adapter, pos, id)?;
                    adapter.finish()?;
                    Ok(())
                })?;
            }
            ticks.finish_tick()?;
            Ok(())
        })();
        self.registered_fluid_ticks = Some(ticks);
        self.registered_fluid_active = false;
        result
    }
}
