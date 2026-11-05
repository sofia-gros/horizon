// Fluid contact is adapted from Tinkers' Construct, Copyright (c) 2022 SlimeKnights.
// MIT License. The full notice is retained in THIRD_PARTY_NOTICES.md.
// Changed: declarative per-family policy and Horizon player health/effect dispatch.
//! Identity-preserving custom-liquid player travel and contact effects.
use super::*;
use horizon_world::{
    fluid_flow::{FlowSample, FluidFlowWorld, flow_at_sample},
    registered_fluids::{RegisteredFluidState, RegisteredFluids, interaction::InteractionPolicy},
};

#[derive(Clone, Copy)]
enum Sample {
    Empty,
    Other,
    Liquid {
        state: RegisteredFluidState,
        source: horizon_world::registered_fluids::RegisteredFluidId,
    },
}
impl FlowSample for Sample {
    fn is_empty(self) -> bool {
        matches!(self, Self::Empty)
    }
    fn same_family(self, other: Self) -> bool {
        matches!((self, other), (Self::Liquid { source: a, .. }, Self::Liquid { source: b, .. }) if a == b)
    }
    fn own_height(self) -> f32 {
        match self {
            Self::Liquid { state, .. } => state.own_height(),
            _ => 0.0,
        }
    }
    fn falling(self) -> bool {
        matches!(self, Self::Liquid { state, .. } if state.falling())
    }
}
struct CurrentWorld<'a, 'r> {
    scene: &'a World,
    catalog: &'a RegisteredFluids<'r>,
    fluids: &'a fluids::FluidResources,
}
impl CurrentWorld<'_, '_> {
    fn loaded_block(&self, p: BlockPos) -> Option<BlockStateId> {
        let layout = self.scene.layout();
        (self.scene.loaded(p.x, p.z) && p.y >= layout.min_y() && p.y < layout.max_y())
            .then(|| self.scene.block(p.x, p.y, p.z))
    }
}
impl FluidFlowWorld<Sample> for CurrentWorld<'_, '_> {
    type Error = std::io::Error;
    type Block = Option<BlockStateId>;
    fn fluid(&mut self, p: BlockPos) -> std::io::Result<Sample> {
        let Some(block) = self.loaded_block(p) else {
            return Ok(Sample::Other);
        };
        Ok(if let Some(state) = self.catalog.from_block(block) {
            Sample::Liquid {
                state,
                source: self.catalog.source(state.id())?,
            }
        } else if self.fluids.fluid(block).kind() == horizon_world::fluid::FluidKind::Empty {
            Sample::Empty
        } else {
            Sample::Other
        })
    }
    fn block(&mut self, p: BlockPos) -> std::io::Result<Self::Block> {
        Ok(self.loaded_block(p))
    }
    fn blocks_fluid_flow(&self, state: Self::Block) -> bool {
        state.is_none_or(|s| self.fluids.blocks_flow(s))
    }
    fn is_ice_class(&self, state: Self::Block) -> bool {
        state.is_some_and(|s| self.fluids.ice(s))
    }
    fn is_face_sturdy(
        &mut self,
        state: Self::Block,
        _: BlockPos,
        d: Direction,
    ) -> std::io::Result<bool> {
        Ok(state.is_none_or(|s| self.fluids.sturdy(s, d)))
    }
}
impl MotionWorld<'_, '_, '_> {
    pub(super) fn update_registered_fluids(&mut self) -> Result<()> {
        self.player.fluids.registered.clear();
        let Some(resources) = self.assets.registered_fluids.clone() else {
            return Ok(());
        };
        if self.player.spectator {
            return Ok(());
        }
        let scene = &*self.scene;
        let mut world = CurrentWorld {
            scene,
            catalog: &resources.catalog,
            fluids: &self.resources.fluids,
        };
        let eyes = Vec3::new(
            self.player.position.x,
            self.player.position.y + f64::from(self.player.dimensions.eye_height),
            self.player.position.z,
        );
        // As with native interactions, the boat shelters the portion below its
        // top unless submerged. Currents never apply to a flying player.
        let mut bounds = self.player.bounds().deflate(Vec3::new(0.001, 0.001, 0.001));
        if let Some(boat) = self.player.fluids.boat.filter(|b| !b.under_water) {
            let lo = bounds.min_position();
            let hi = bounds.max_position();
            if boat.max_y >= hi.y {
                return Ok(());
            }
            bounds = Aabb::new(Vec3::new(lo.x, lo.y.max(boat.max_y), lo.z), hi);
        }
        let contacts = resources.catalog.contacts_with_flow(
            bounds,
            eyes,
            8192,
            |p| {
                let layout = scene.layout();
                (scene.loaded(p.x, p.z) && p.y >= layout.min_y() && p.y < layout.max_y())
                    .then(|| scene.block(p.x, p.y, p.z))
            },
            |p| {
                let sample = world.fluid(p)?;
                flow_at_sample(&mut world, sample, p)
            },
        )?;
        for contact in &contacts {
            if contact.height <= 0.0 {
                continue;
            }
            if let Some(policy) = resources.interaction(contact.source) {
                if !self.player.flying && policy.motion_scale > 0.0 {
                    contact.current.apply_to(self, policy.motion_scale)?;
                }
                self.player.fall_distance *= policy.fall_distance_scale;
            }
        }
        self.player.fluids.registered = contacts;
        Ok(())
    }
    /// Deepest configured body contact wins travel; every family retains its
    /// independent burning/extinguishing/drowning conditions.
    pub(super) fn registered_travel(&self) -> Option<(f64, InteractionPolicy)> {
        let resources = self.assets.registered_fluids.as_ref()?;
        self.player
            .fluids
            .registered
            .iter()
            .filter(|c| c.height > 0.0)
            .filter_map(|c| {
                resources
                    .interaction(c.source)
                    .copied()
                    .map(|p| (c.height, p))
            })
            .max_by(|a, b| a.0.total_cmp(&b.0))
    }
    pub(super) fn registered_condition(
        &self,
        eyes: bool,
        condition: impl Fn(&InteractionPolicy) -> bool,
    ) -> bool {
        self.assets.registered_fluids.as_ref().is_some_and(|r| {
            self.player.fluids.registered.iter().any(|c| {
                (if eyes { c.eyes_inside } else { c.height > 0.0 })
                    && r.interaction(c.source).is_some_and(&condition)
            })
        })
    }
    // BurningLiquidBlock and MobEffectLiquidBlock.entityInside, TConstruct
    // f4e77d43 (MIT). Another fluid's height cannot trigger this family's effects.
    pub(super) fn registered_fluid_inside(&mut self, state: BlockStateId) -> Result<()> {
        let Some(resources) = &self.assets.registered_fluids else {
            return Ok(());
        };
        let Some(policy) = resources.interaction_at_contact(state, &self.player.fluids.registered)
        else {
            return Ok(());
        };
        if let Some(effect) = policy.effect {
            self.player
                .events
                .push(MotionEvent::RegisteredFluidEffect(effect));
        }
        if let Some(burn) = policy.burn {
            self.player.ignite_for_ticks(i32::from(burn.seconds) * 20)?;
            self.environment_hurt("minecraft:lava", burn.damage)?;
        }
        Ok(())
    }
}
