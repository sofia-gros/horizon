// Fluid contact is adapted from Tinkers' Construct, Copyright (c) 2022 SlimeKnights.
// MIT License. The full notice is retained in THIRD_PARTY_NOTICES.md.
// Changed: declarative policy and Horizon player health/effect/sound dispatch.
//! Identity-preserving custom-liquid player travel and contact effects.
use super::*;
use horizon_core::random::RandomSource;
use horizon_world::{
    fluid_flow::{FlowSample, FluidFlowWorld, flow_at_sample},
    registered_fluids::{RegisteredFluidState, interaction::InteractionPolicy},
};

#[derive(Clone, Copy)]
enum Sample {
    Empty,
    Other,
    Liquid {
        state: RegisteredFluidState,
        source: horizon_world::registered_fluids::RegisteredFluidId,
        up: bool,
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
    fn flows_up(self) -> bool {
        matches!(self, Self::Liquid { up: true, .. })
    }
}
struct CurrentWorld<'a, 'r> {
    scene: &'a World,
    resources: &'a horizon_world::registered_fluids::resources::RegisteredFluidResources<'r>,
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
        Ok(
            if let Some(state) = self.resources.catalog.from_block(block) {
                Sample::Liquid {
                    state,
                    source: self.resources.catalog.source(state.id())?,
                    up: self.resources.flow.vertical_direction(state.id())?
                        == horizon_world::registered_flow::VerticalFlow::Up,
                }
            } else if self.fluids.fluid(block).kind() == horizon_world::fluid::FluidKind::Empty {
                Sample::Empty
            } else {
                Sample::Other
            },
        )
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
            resources: &resources,
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
                    .cloned()
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
        let burn = policy.burn;
        if policy.effect.is_some()
            && let Some(fluid) = resources.catalog.from_block(state)
        {
            self.player
                .events
                .push(MotionEvent::RegisteredFluidEffect(fluid.id()));
        }
        if let Some(burn) = burn {
            self.registered_fluid_burn(burn)?;
        }
        Ok(())
    }
    fn registered_fluid_burn(
        &mut self,
        burn: horizon_world::registered_fluids::interaction::BurnContact,
    ) -> Result<()> {
        self.player.ignite_for_ticks(i32::from(burn.seconds) * 20)?;
        if self.environment_hurt_result("minecraft:lava", burn.damage)? {
            // Source draws from level.random only if hurt returned true. Keep
            // f32 operations and do not substitute the generic hurt sound.
            let pitch = 2.0f32 + self.scene.random.next_float() * 0.4f32;
            self.player
                .events
                .push(MotionEvent::RegisteredFluidBurnSound {
                    position: self.player.position,
                    pitch,
                });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_world::registered_fluids::interaction::BurnContact;

    #[test]
    fn accepted_fluid_burns_alone_emit_sound_and_draw_pitch() {
        let root = std::env::var_os("HORIZON_TEST_RESOURCES")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../.local/horizon-client")
                    .join(horizon_core::VANILLA_VERSION)
            });
        if !root.join("movement-properties.nbt").exists() {
            eprintln!("skipped registered-fluid burn audio: prepared resources unavailable");
            return;
        }
        let assets = Assets::load(&root).unwrap();
        let render = assets.reload(&root).unwrap();
        let binding = crate::collisions::load(&root, assets.registry).unwrap();
        let resources = MotionResources::load(&root, &assets).unwrap();
        let burn = BurnContact {
            seconds: 10,
            damage: 5.0,
        };
        for mode in 0..5 {
            let mut scene = World::test_scene(&assets, &render, 7).unwrap();
            let mut player = PlayerMotion::new(&resources, Vec3::new(20.5, 4.0, 20.5)).unwrap();
            player.invulnerable = mode == 1;
            player.spectator = mode == 2;
            player.client_side = mode == 3;
            player.status.fire_resistance = mode == 4;
            let mut expected_random = scene.random.clone();
            let mut w = MotionWorld {
                entities: &[],
                player: &mut player,
                scene: &mut scene,
                assets: &assets,
                collision_binding: &binding,
                resources: &resources,
            };
            w.registered_fluid_burn(burn).unwrap();
            let sounds = |events: &[MotionEvent]| {
                events
                    .iter()
                    .filter_map(|event| match event {
                        MotionEvent::RegisteredFluidBurnSound { position, pitch } => {
                            Some((*position, *pitch))
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            };
            if mode == 0 {
                assert_eq!(w.player.health.health(), 15.0);
                let pitch = 2.0f32 + expected_random.next_float() * 0.4f32;
                assert_eq!(sounds(&w.player.events), vec![(w.player.position, pitch)]);
                w.player.events.clear();
                // A second equal hit inside the damage cooldown is rejected.
                w.registered_fluid_burn(burn).unwrap();
                assert!(sounds(&w.player.events).is_empty());
                // A stronger cooldown hit is accepted without a new hurt
                // animation, and still gets the source's extra burn sound.
                w.registered_fluid_burn(BurnContact {
                    damage: 6.0,
                    ..burn
                })
                .unwrap();
                assert_eq!(w.player.health.health(), 14.0);
                let pitch = 2.0f32 + expected_random.next_float() * 0.4f32;
                assert_eq!(sounds(&w.player.events), vec![(w.player.position, pitch)]);
            } else {
                assert!(sounds(&w.player.events).is_empty());
                assert_eq!(w.player.health.health(), 20.0);
            }
            assert_eq!(
                w.scene.random.next_float(),
                expected_random.next_float(),
                "rejected damage must not consume the pitch random draw"
            );
        }
    }
}
