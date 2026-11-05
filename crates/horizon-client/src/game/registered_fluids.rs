// Contact cure policy follows Tinkers' Construct, Copyright (c) 2022 SlimeKnights.
// MIT License; full notice in THIRD_PARTY_NOTICES.md.
// Changed: explicit declaration and Horizon player-instance dispatch.
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
    /// Contact effects use the existing authoritative effect merge/attribute
    /// path. Prediction never adds effects locally; normal sync supplies them.
    pub(super) fn registered_fluid_effect(
        &mut self,
        fluid: horizon_world::registered_fluids::RegisteredFluidId,
    ) {
        if self.remote.is_some() || self.spectator() {
            return;
        }
        let Some(effect) = self
            .res
            .rest()
            .registered_fluids
            .as_ref()
            .and_then(|resources| resources.interaction(fluid))
            .and_then(|policy| policy.effect.as_ref())
        else {
            return;
        };
        // Resolve to the existing registry-owned static ID, never leak a
        // per-contact string or treat an unknown declaration as an inert effect.
        let Some(definition) = horizon_client::brewing::effect(effect.id.player_id()) else {
            return;
        };
        let mut instance = horizon_client::effects::EffectInstance::new(
            definition.id,
            effect.duration_ticks as i32,
            i32::from(effect.amplifier),
        );
        instance.no_curatives = effect.no_curatives;
        self.add_player_effect(instance);
    }
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

impl Game<'_> {
    pub(super) fn registered_bucket(&self, item: &str) -> bool {
        self.res
            .rest()
            .registered_fluids
            .as_ref()
            .is_some_and(|r| r.bucket_fluid(item).is_some())
    }
    pub(super) fn registered_source_at(&self, pos: [i32; 3]) -> bool {
        self.res
            .rest()
            .registered_fluids
            .as_ref()
            .and_then(|r| {
                r.catalog
                    .from_block(self.world.block(pos[0], pos[1], pos[2]))
            })
            .is_some_and(|s| s.is_source())
    }
    /// Client-side result only. Authoritative mutation and inventory correction
    /// arrive from the server; custom buckets do not predict native fluid IDs.
    pub(super) fn registered_bucket_preview(&self, stack: &ItemStack) -> Option<bool> {
        let resources = self.res.rest().registered_fluids.as_ref()?;
        if resources.bucket_fluid(&stack.item).is_some() {
            return Some(self.may_build() && self.pick().is_some());
        }
        if &*stack.item == "minecraft:bucket"
            && self
                .bucket_hit()
                .is_some_and(|h| self.registered_source_at(h.pos))
        {
            return Some(self.may_build());
        }
        None
    }
    /// A shared player/dispenser transaction. Callers resolve the returned item
    /// first, then commit their inventory before propagating `finish` errors.
    pub(super) fn change_registered_source(
        &mut self,
        pos: BlockPos,
        source: Option<horizon_world::registered_fluids::RegisteredFluidId>,
    ) -> Result<(bool, std::io::Result<()>)> {
        if self.remote.is_some() {
            return Ok((false, Ok(())));
        }
        let Some(resources) = self.res.rest().registered_fluids.clone() else {
            return Ok((false, Ok(())));
        };
        let Some(mut queue) = self.registered_fluid_ticks.take() else {
            return Ok((false, Ok(())));
        };
        self.registered_fluid_active = true;
        let result = self.with_level(
            |behaviours, level, native| -> Result<(bool, std::io::Result<()>)> {
                let mut adapter = RegisteredLevel::new(
                    behaviours,
                    level,
                    native,
                    &resources.catalog,
                    &mut queue,
                )?;
                let changed = if let Some(source) = source {
                    resources.flow.place_source(&mut adapter, pos, source)?
                } else {
                    resources.flow.pickup_source(&mut adapter, pos)?.is_some()
                };
                Ok((changed, adapter.finish()))
            },
        );
        self.registered_fluid_ticks = Some(queue);
        self.registered_fluid_active = false;
        result
    }
    /// Returns None for native buckets so their existing path stays unchanged.
    pub(super) fn use_registered_bucket(&mut self, stack: &ItemStack) -> Result<Option<UseResult>> {
        let Some(resources) = self.res.rest().registered_fluids.clone() else {
            return Ok(None);
        };
        let source = resources.bucket_fluid(&stack.item);
        let filling = source.is_none() && &*stack.item == "minecraft:bucket";
        if source.is_none() && !filling {
            return Ok(None);
        }
        let hit = if filling {
            self.bucket_hit()
        } else {
            self.pick()
        };
        let Some(hit) = hit else {
            return Ok(source.map(|_| UseResult::Pass));
        };
        if filling && !self.registered_source_at(hit.pos) {
            return Ok(None);
        }
        if !self.may_build() {
            return Ok(Some(UseResult::Fail));
        }
        let target = if filling { hit.pos } else { hit.beside() };
        let mut canceled = false;
        self.native_fill_bucket(stack, target, &mut canceled);
        if canceled || (!filling && self.bucket_empty_cancelled(stack, target)) {
            return Ok(Some(UseResult::Fail));
        }
        // Resolve the return item before removing a source.
        let returned = if let Some(source) = source {
            let _ = resources.catalog.name(source)?;
            "minecraft:bucket".to_owned()
        } else {
            let Some(fluid) = resources
                .catalog
                .from_block(self.world.block(target[0], target[1], target[2]))
            else {
                return Ok(Some(UseResult::Fail));
            };
            resources.bucket(fluid.id())?.to_owned()
        };
        let pos = BlockPos::new(target[0], target[1], target[2]);
        let (changed, scheduled) = self.change_registered_source(pos, source)?;
        if !changed {
            scheduled?;
            return Ok(Some(UseResult::Fail));
        }
        // Inventory follows a committed block even if scheduling reports an
        // invariant failure: returning early here would duplicate a container.
        if filling || !self.creative {
            self.filled_result(ItemStack::new(returned, 1));
        }
        scheduled?;
        self.award_stat("used", &stack.item, 1);
        self.post_block_game_event(
            if filling {
                "minecraft:fluid_pickup"
            } else {
                "minecraft:fluid_place"
            },
            target,
            self.sculk_actor(),
            None,
        );
        self.process_level_events()?;
        Ok(Some(UseResult::Success { swing: true }))
    }
}
