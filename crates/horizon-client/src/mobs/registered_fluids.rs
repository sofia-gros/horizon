// Fluid burn/effect contact adapts Tinkers' Construct, Copyright (c) 2022 SlimeKnights.
// MIT License. Full notice and source are in THIRD_PARTY_NOTICES.md.
// Changed: explicit family policy, bounded loaded-only sampling, host mob damage.
//! Registered liquid block contact for authoritative mobs, without native aliases.
use super::{Damage, DamageKind, Mob, MobEvent, MobWorld};
use horizon_core::{
    geometry::{Aabb, Vec3},
    position::BlockPos,
    random::RandomSource,
};
use horizon_world::{
    registered_fluids::{
        contact::Contact, interaction::TravelPolicy, resources::RegisteredFluidResources,
    },
    registry::BlockStateId,
};

pub(super) fn contacts(m: &Mob, w: &MobWorld<'_>) -> Vec<Contact> {
    let Some(resources) = w.registered_fluids else {
        return Vec::new();
    };
    sample(m, resources, |p| {
        (w.world.loaded(p.x, p.z) && (w.world.bottom_y()..w.world.top_y()).contains(&p.y))
            .then(|| w.world.block(p.x, p.y, p.z))
    })
}

fn sample(
    m: &Mob,
    resources: &RegisteredFluidResources<'_>,
    block: impl FnMut(BlockPos) -> Option<BlockStateId>,
) -> Vec<Contact> {
    // This snapshot belongs only to this block-contact pass. Oversized or
    // invalid bounds fail closed rather than scanning unbounded world space.
    resources
        .catalog
        .contacts(
            m.bounds().deflate(Vec3::new(0.001, 0.001, 0.001)),
            Vec3::new(m.position.x, m.eye_y(), m.position.z),
            8192,
            block,
        )
        .unwrap_or_default()
}

/// Native water/lava and species-specific travel take precedence at the caller.
/// Like the player adapter, the deepest configured family wins; equal heights
/// retain the last encounter in the bounded sampler's deterministic order.
pub(super) fn travel(m: &Mob, w: &MobWorld<'_>) -> Option<(f64, TravelPolicy)> {
    select_travel(w.registered_fluids?, &contacts(m, w))
}
fn select_travel(
    resources: &RegisteredFluidResources<'_>,
    contacts: &[Contact],
) -> Option<(f64, TravelPolicy)> {
    contacts
        .iter()
        .filter(|contact| contact.height > 0.0)
        .filter_map(|contact| {
            resources
                .interaction(contact.source)
                .map(|p| (contact.height, p.travel))
        })
        .max_by(|a, b| a.0.total_cmp(&b.0))
}

/// A fluid exit must not put the body inside any registered family. Unlike
/// ordinary contact observations, absent probe cells are not evidence of dry air.
pub(super) fn dry_exit(w: &MobWorld<'_>, bounds: Aabb) -> bool {
    let Some(resources) = w.registered_fluids else {
        return true;
    };
    dry_sample(resources, bounds, |p| {
        (w.world.loaded(p.x, p.z) && (w.world.bottom_y()..w.world.top_y()).contains(&p.y))
            .then(|| w.world.block(p.x, p.y, p.z))
    })
}
fn dry_sample(
    resources: &RegisteredFluidResources<'_>,
    bounds: Aabb,
    mut block: impl FnMut(BlockPos) -> Option<BlockStateId>,
) -> bool {
    let mut missing = false;
    let result = resources
        .catalog
        .contacts(bounds, bounds.min_position(), 8192, |p| {
            let state = block(p);
            missing |= state.is_none();
            state
        });
    !missing && result.is_ok_and(|contacts| contacts.iter().all(|c| c.height <= 0.0))
}

/// Only the family's explicit eye-immersion drowning policy enters common
/// mob air logic; body contact, burn state and native water flags are unrelated.
pub(super) fn drowning_eyes(m: &Mob, w: &MobWorld<'_>) -> bool {
    w.registered_fluids
        .is_some_and(|resources| can_drown(resources, &contacts(m, w)))
}
fn can_drown(resources: &RegisteredFluidResources<'_>, contacts: &[Contact]) -> bool {
    contacts.iter().any(|contact| {
        contact.eyes_inside
            && resources
                .interaction(contact.source)
                .is_some_and(|policy| policy.can_drown)
    })
}

/// Policy-only opt-in to the mob's existing water/rain fire-clearing phase.
/// A burning mob needs body contact, not eye immersion or native water tags.
pub(super) fn extinguishes(m: &Mob, w: &MobWorld<'_>) -> bool {
    if m.fire_ticks <= 0 {
        return false;
    }
    w.registered_fluids
        .is_some_and(|resources| can_extinguish(resources, &contacts(m, w)))
}
fn can_extinguish(resources: &RegisteredFluidResources<'_>, contacts: &[Contact]) -> bool {
    contacts.iter().any(|contact| {
        contact.height > 0.0
            && resources
                .interaction(contact.source)
                .is_some_and(|policy| policy.extinguishes)
    })
}

pub(super) fn inside(
    m: &mut Mob,
    resources: Option<&RegisteredFluidResources<'_>>,
    state: BlockStateId,
    contacts: &[Contact],
    level_random: &mut impl RandomSource,
) {
    let Some(policy) = resources.and_then(|r| r.interaction_at_contact(state, contacts)) else {
        return;
    };
    // Source effect contact applies to LivingEntity, including fire-immune mobs.
    // Resolve vanilla immunities and effect registry IDs in the game adapter.
    if let Some(effect) = &policy.effect
        && matches!(
            effect.id.id(),
            "minecraft:poison" | "minecraft:slowness" | "minecraft:blindness"
        )
        && let Some(fluid) = resources.and_then(|r| r.catalog.from_block(state))
    {
        m.outbox.push(MobEvent::RegisteredFluidEffect {
            mob: m.id,
            fluid: fluid.id(),
        });
    }
    // BurningLiquidBlock checks immunity before ignition. Another family's
    // height cannot trigger this block's effect or burn policy.
    if m.kind.flags.fire_immune {
        return;
    }
    let Some(burn) = policy.burn else {
        return;
    };
    m.ignite_for_ticks(i32::from(burn.seconds) * 20);
    if m.damage(burn.damage, Damage::of(DamageKind::Lava)) {
        let pitch = 2.0f32 + level_random.next_float() * 0.4f32;
        // The source draws the pitch before Entity.playSound checks silence.
        if !m.silent {
            m.outbox.push(MobEvent::RegisteredFluidBurnSound {
                at: [m.position.x, m.position.y, m.position.z],
                pitch,
                hostile: m.hostile(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_core::random::LegacyRandom;
    use horizon_world::registry::BlockRegistry;
    use serde_json::json;
    use std::sync::Arc;

    fn registry() -> BlockRegistry {
        let mut rows: Vec<_> = ["test:hot", "test:other"]
            .into_iter()
            .flat_map(|name| {
                (0..16).map(move |level| {
                json!([name, false, true, "", {"id":name,"properties":{"level":level.to_string()}}])
            })
            })
            .collect();
        rows.push(json!(["minecraft:air", false, false, "", {"id":"minecraft:air"}]));
        BlockRegistry::from_facts_json(
            json!({"states":rows,"defaults":{"test:hot":0,"test:other":16,"minecraft:air":32}})
                .to_string()
                .as_bytes(),
        )
        .unwrap()
    }
    fn resources(registry: &BlockRegistry) -> RegisteredFluidResources<'_> {
        resources_with_effect(registry, None)
    }
    fn resources_with_effect<'r>(
        registry: &'r BlockRegistry,
        effect: Option<&str>,
    ) -> RegisteredFluidResources<'r> {
        resources_with_motion(registry, effect, [0.0, 0.0])
    }
    fn resources_with_motion<'r>(
        registry: &'r BlockRegistry,
        effect: Option<&str>,
        scales: [f64; 2],
    ) -> RegisteredFluidResources<'r> {
        let effect = effect
            .map(|id| json!({"id":id,"duration_ticks":100,"amplifier":0,"no_curatives":true}));
        let burn = effect.is_none().then(|| json!({"seconds":10,"damage":3.0}));
        let fluids: Vec<_> = ["hot", "other"].into_iter().map(|name| json!({
            "source":format!("test:{name}"),"flowing":format!("test:flowing_{name}"),
            "block":format!("test:{name}"),"bucket":format!("test:{name}_bucket"),
            "policy":{"drop_off":1,"slope_distance":4,"tick_delay":5,"source_conversion":false},
            "visual":{"still":"test:still","flowing":"test:flow","tint":16777215,"emission":0},
            "interaction":{"travel":{"acceleration":if name == "hot" {0.02} else {0.04},"horizontal_drag":0.8,"vertical_drag":0.8,"gravity_scale":0.0625,"jump":0.04},
                "motion_scale":scales[usize::from(name == "other")],"can_swim":false,"can_drown":name == "hot","extinguishes":name == "hot" && effect.is_some(),
                "fall_distance_scale":1.0,"burn":burn,"effect":effect}
        })).collect();
        RegisteredFluidResources::from_reader(
            registry,
            json!({"format":1,"fluids":fluids}).to_string().as_bytes(),
        )
        .unwrap()
    }
    fn mob() -> Mob {
        Mob::new(
            1,
            super::super::sensor_scan_tests::kind(),
            Vec3::new(0.5, 0.0, 0.5),
            0.0,
        )
    }
    fn burns(m: &Mob) -> Vec<([f64; 3], f32, f32)> {
        m.outbox
            .iter()
            .filter_map(|event| match event {
                MobEvent::RegisteredFluidBurnSound { at, pitch, hostile } => {
                    assert!(!hostile);
                    Some((*at, 0.4, *pitch))
                }
                _ => None,
            })
            .collect()
    }
    fn current_contact(
        resources: &RegisteredFluidResources<'_>,
        state: u32,
        vector: Vec3,
    ) -> Contact {
        let mut current = horizon_world::entity_fluid::CurrentAccumulator::default();
        current.height = 0.8;
        current.accumulate(vector);
        Contact {
            source: resources
                .catalog
                .source(
                    resources
                        .catalog
                        .from_block(BlockStateId(state))
                        .unwrap()
                        .id(),
                )
                .unwrap(),
            height: 0.8,
            eyes_inside: false,
            current,
        }
    }
    #[test]
    fn travel_chooses_deepest_body_family_and_keeps_eye_only_contact_inert() {
        let registry = registry();
        let resources = resources(&registry);
        let mut a = current_contact(&resources, 1, Vec3::ZERO);
        let mut b = current_contact(&resources, 16, Vec3::ZERO);
        a.height = 0.7;
        b.height = 0.2;
        assert_eq!(
            select_travel(&resources, &[a, b]).unwrap().1.acceleration,
            0.02
        );
        b.height = 0.9;
        assert_eq!(
            select_travel(&resources, &[a, b]).unwrap().1.acceleration,
            0.04
        );
        // Equal depth follows the existing player's last-encounter tie rule.
        b.height = a.height;
        assert_eq!(
            select_travel(&resources, &[a, b]).unwrap().1.acceleration,
            0.04
        );
        assert_eq!(
            select_travel(&resources, &[b, a]).unwrap().1.acceleration,
            0.02
        );
        a.height = 0.0;
        a.eyes_inside = true;
        assert!(select_travel(&resources, &[a]).is_none());
        assert!(select_travel(&resources, &[]).is_none());
        // Geometry also rejects a level-seven surface below the mob's feet.
        let mut m = mob();
        m.position.y = 0.2;
        let contacts = sample(&m, &resources, |p| (p.y == 0).then_some(BlockStateId(7)));
        assert!(select_travel(&resources, &contacts).is_none());
    }
    fn travel_policy() -> TravelPolicy {
        TravelPolicy {
            acceleration: 0.02,
            horizontal_drag: 0.8,
            vertical_drag: 0.8,
            gravity_scale: 0.0625,
            jump: 0.04,
        }
    }
    #[test]
    fn configured_jump_strength_obeys_body_depth_and_ground_threshold() {
        let mut policy = travel_policy();
        policy.jump = 0.12;
        let mut m = mob();
        m.velocity = Vec3::new(0.25, -0.1, -0.5);
        assert!(!m.jump_in_registered_fluid(0.0, policy));
        m.on_ground = true;
        assert!(!m.jump_in_registered_fluid(0.4, policy));
        assert_eq!(m.velocity, Vec3::new(0.25, -0.1, -0.5));
        m.ai.no_jump_delay = 7;
        assert!(m.jump_in_registered_fluid(0.8, policy));
        assert!((m.velocity.y - 0.02).abs() < 1e-12);
        assert_eq!(m.velocity.x, 0.25);
        assert_eq!(m.velocity.z, -0.5);
        assert_eq!(m.ai.no_jump_delay, 7);
        assert!(!m.fluids.in_water && !m.fluids.in_lava);
        // An airborne shallow body still uses the fluid jump.
        m.on_ground = false;
        assert!(m.jump_in_registered_fluid(0.1, policy));
        assert!((m.velocity.y - 0.14).abs() < 1e-12);
    }
    #[test]
    fn zero_strength_consumes_deep_jump_and_species_threshold_is_retained() {
        let mut m = mob();
        let mut policy = travel_policy();
        policy.jump = 0.0;
        m.on_ground = true;
        assert!(m.jump_in_registered_fluid(0.8, policy));
        assert_eq!(m.velocity, Vec3::ZERO);
        Arc::make_mut(&mut m.kind).species = super::super::Species::Breeze;
        // Breeze uses eye height rather than the ordinary 0.4 threshold.
        assert!(!m.jump_in_registered_fluid(1.0, policy));
        assert!(m.jump_in_registered_fluid(1.5, policy));
    }
    #[test]
    fn exit_requires_dry_body_space_for_every_registered_family() {
        let registry = registry();
        let resources = resources(&registry);
        let bounds = Aabb::new(Vec3::new(0.1, 0.2, 0.1), Vec3::new(0.9, 1.6, 0.9));
        let air = BlockStateId(32);
        assert!(dry_sample(&resources, bounds, |_| Some(air)));
        for state in [0, 1, 16, 17] {
            assert!(!dry_sample(&resources, bounds, |p| {
                Some(if p.y == 0 { BlockStateId(state) } else { air })
            }));
        }
        // A thin layer below the prospective feet does not block escape.
        assert!(dry_sample(&resources, bounds, |p| {
            Some(if p.y == 0 { BlockStateId(7) } else { air })
        }));
    }
    #[test]
    fn exit_does_not_treat_missing_or_overbudget_samples_as_dry() {
        let registry = registry();
        let resources = resources(&registry);
        let bounds = Aabb::new(Vec3::new(0.1, 0.2, 0.1), Vec3::new(0.9, 1.6, 0.9));
        assert!(!dry_sample(&resources, bounds, |_| None));
        assert!(!dry_sample(&resources, bounds, |p| (p.y == 0)
            .then_some(BlockStateId(32))));
        let oversized = Aabb::new(Vec3::ZERO, Vec3::new(100.0, 100.0, 100.0));
        assert!(!dry_sample(&resources, oversized, |_| panic!(
            "budget must reject before lookup"
        )));
    }
    #[test]
    fn travel_damps_collision_resolved_velocity_before_scaled_gravity() {
        let mut m = mob();
        // X already stopped by collision; do not restore its pre-move value.
        m.velocity = Vec3::new(0.0, -0.1, 0.6);
        m.finish_registered_travel(travel_policy(), 0.08, true);
        assert_eq!(m.velocity.x, 0.0);
        assert!((m.velocity.y + 0.085).abs() < 1e-12);
        assert!((m.velocity.z - 0.48).abs() < 1e-12);
        assert!(!m.fluids.in_water && !m.fluids.in_lava);
    }
    #[test]
    fn travel_keeps_no_gravity_slow_falling_and_sprinting_adjustments() {
        for mode in 0..3 {
            let mut m = mob();
            m.velocity = Vec3::new(0.2, -0.1, 0.0);
            if mode == 0 {
                m.no_gravity = true;
            } else if mode == 1 {
                m.add_effect("minecraft:slow_falling", 100, 0);
            } else {
                m.extra.sprinting = true;
            }
            let gravity = m.effective_gravity();
            m.finish_registered_travel(travel_policy(), gravity, true);
            let expected = if mode == 1 { -0.080625 } else { -0.08 };
            assert!((m.velocity.y - expected).abs() < 1e-12);
            assert!((m.velocity.x - 0.16).abs() < 1e-12);
        }
    }
    #[test]
    fn travel_uses_premove_falling_flag_for_the_existing_fluid_snap() {
        let mut policy = travel_policy();
        policy.gravity_scale = 0.125;
        let mut m = mob();
        // Block effects/movement may have raised Y since falling was captured.
        m.velocity.y = 0.0125;
        m.finish_registered_travel(policy, 0.08, true);
        assert_eq!(m.velocity.y, -0.003);
        m.velocity.y = 0.0125;
        m.finish_registered_travel(policy, 0.08, false);
        assert!(m.velocity.y.abs() < 1e-12);
    }
    #[test]
    fn registered_currents_use_nonplayer_direction_and_each_familys_scale() {
        let registry = registry();
        let resources = resources_with_motion(&registry, None, [0.01, 0.02]);
        let mut m = mob();
        let mut east = current_contact(&resources, 1, Vec3::new(3.0, 0.0, 4.0));
        east.current.accumulate(Vec3::new(3.0, 0.0, 4.0));
        let north = current_contact(&resources, 16, Vec3::new(0.0, 0.0, -8.0));
        m.push_registered_currents(&resources, &[east, north]);
        assert!((m.velocity.x - 0.006).abs() < 1e-12);
        assert!((m.velocity.z + 0.012).abs() < 1e-12);
        assert_eq!(m.velocity.y, 0.0);
        assert!(!m.fluids.in_water && !m.fluids.in_lava);
        assert!(m.outbox.is_empty());
    }
    #[test]
    fn registered_currents_keep_existing_species_push_immunity() {
        use super::super::Species as S;
        let registry = registry();
        let resources = resources_with_motion(&registry, None, [0.01, 0.0]);
        let contact = current_contact(&resources, 0, Vec3::new(1.0, 0.0, 0.0));
        for species in [
            S::Tadpole,
            S::Cod,
            S::Salmon,
            S::TropicalFish,
            S::Pufferfish,
            S::Squid,
            S::GlowSquid,
            S::Dolphin,
            S::Turtle,
            S::Axolotl,
            S::Frog,
            S::Nautilus,
            S::ZombieNautilus,
        ] {
            let mut m = mob();
            Arc::make_mut(&mut m.kind).species = species;
            m.push_registered_currents(&resources, &[contact]);
            assert_eq!(m.velocity, Vec3::ZERO, "{species:?}");
        }
        let mut m = mob();
        Arc::make_mut(&mut m.kind).flags.fire_immune = true;
        m.push_registered_currents(&resources, &[contact]);
        assert_eq!(m.velocity.x, 0.01, "fire immunity is not current immunity");
    }
    #[test]
    fn zero_policy_eye_only_and_cancelled_currents_do_not_push() {
        let registry = registry();
        let resources = resources_with_motion(&registry, None, [0.01, 0.0]);
        let mut eye = current_contact(&resources, 0, Vec3::new(1.0, 0.0, 0.0));
        eye.height = 0.0;
        eye.eyes_inside = true;
        let zero_policy = current_contact(&resources, 16, Vec3::new(1.0, 0.0, 0.0));
        let mut cancelled = current_contact(&resources, 0, Vec3::new(1.0, 0.0, 0.0));
        cancelled.current.accumulate(Vec3::new(-1.0, 0.0, 0.0));
        let mut m = mob();
        m.push_registered_currents(&resources, &[eye, zero_policy, cancelled]);
        assert_eq!(m.velocity, Vec3::ZERO);
    }
    #[test]
    fn registered_currents_keep_minimum_drift_and_vertical_direction() {
        let registry = registry();
        let scale = 0.0023333333333333335;
        let resources = resources_with_motion(&registry, None, [scale, 0.0]);
        for direction in [-1.0, 1.0] {
            let contact = current_contact(&resources, 8, Vec3::new(0.0, direction * 6.0, 0.0));
            let mut m = mob();
            m.push_registered_currents(&resources, &[contact]);
            assert_eq!(m.velocity.y, direction * 0.0045000000000000005);
            m.velocity = Vec3::new(0.01, 0.0, 0.0);
            m.push_registered_currents(&resources, &[contact]);
            assert_eq!(m.velocity.y, direction * scale);
            assert_eq!(m.velocity.x, 0.01);
        }
    }
    #[test]
    fn burn_requires_matching_positive_body_contact_and_obeys_damage_result() {
        let registry = registry();
        let resources = resources(&registry);
        for mode in 0..7 {
            let mut m = mob();
            if mode == 1 {
                m.invulnerable_flag = true;
            }
            if mode == 2 {
                Arc::make_mut(&mut m.kind).flags.fire_immune = true;
            }
            if mode == 5 {
                m.add_effect("minecraft:fire_resistance", 100, 0);
            }
            if mode == 6 {
                m.silent = true;
            }
            let contacts = sample(&m, &resources, |p| (p.y == 0).then_some(BlockStateId(0)));
            let mut random = LegacyRandom::new(7);
            let mut expected = random.clone();
            let state = if mode == 3 {
                BlockStateId(16)
            } else {
                BlockStateId(0)
            };
            let live = if mode == 4 { None } else { Some(&resources) };
            inside(&mut m, live, state, &contacts, &mut random);
            if mode == 6 {
                assert_eq!(m.health, 7.0);
                assert_eq!(m.fire_ticks, 200);
                assert!(burns(&m).is_empty());
                let _ = expected.next_float();
            } else if mode == 0 {
                assert_eq!(m.health, 7.0);
                assert_eq!(m.fire_ticks, 200);
                assert_eq!(
                    burns(&m),
                    vec![([0.5, 0.0, 0.5], 0.4, 2.0 + expected.next_float() * 0.4)]
                );
                // Repeated blocks of the same fluid refresh ignition, but an
                // invulnerability-rejected hit has no extra sound/random draw.
                inside(&mut m, live, BlockStateId(1), &contacts, &mut random);
                assert_eq!(m.health, 7.0);
                assert_eq!(burns(&m).len(), 1);
            } else {
                assert_eq!(m.health, 10.0);
                assert!(burns(&m).is_empty());
                assert_eq!(m.fire_ticks, if matches!(mode, 1 | 5) { 200 } else { 0 });
            }
            assert_eq!(random.next_float(), expected.next_float());
        }
    }
    #[test]
    fn standard_effect_contacts_include_fire_immune_mobs_but_not_other_families() {
        let registry = registry();
        for id in [
            "minecraft:poison",
            "minecraft:slowness",
            "minecraft:blindness",
            "test:custom",
        ] {
            let resources = resources_with_effect(&registry, Some(id));
            let mut m = mob();
            Arc::make_mut(&mut m.kind).flags.fire_immune = true;
            let contacts = sample(&m, &resources, |p| (p.y == 0).then_some(BlockStateId(0)));
            let mut random = LegacyRandom::new(8);
            inside(
                &mut m,
                Some(&resources),
                BlockStateId(16),
                &contacts,
                &mut random,
            );
            assert!(m.outbox.is_empty());
            inside(
                &mut m,
                Some(&resources),
                BlockStateId(1),
                &contacts,
                &mut random,
            );
            if id == "test:custom" {
                assert!(m.outbox.is_empty());
            } else {
                assert!(
                    matches!(m.outbox.as_slice(), [MobEvent::RegisteredFluidEffect { mob: 1, fluid }]
                    if *fluid == resources.catalog.from_block(BlockStateId(1)).unwrap().id())
                );
            }
            assert_eq!(m.health, 10.0);
            assert_eq!(m.fire_ticks, 0);
        }
    }
    #[test]
    fn drowning_requires_eyes_in_the_matching_enabled_family() {
        let registry = registry();
        let resources = resources(&registry);
        let m = mob();
        let body_only = sample(&m, &resources, |p| (p.y == 0).then_some(BlockStateId(0)));
        assert!(body_only[0].height > 0.0);
        assert!(!can_drown(&resources, &body_only));
        for state in [0, 1] {
            let full = sample(&m, &resources, |p| {
                (p.y <= 1).then_some(BlockStateId(state))
            });
            assert!(can_drown(&resources, &full));
        }
        let other = sample(&m, &resources, |p| (p.y <= 1).then_some(BlockStateId(16)));
        assert!(!can_drown(&resources, &other));
        assert!(!can_drown(&resources, &sample(&m, &resources, |_| None)));
        // An independently recorded eye sample does not need body depth.
        let mut eye = current_contact(&resources, 1, Vec3::ZERO);
        eye.height = 0.0;
        eye.eyes_inside = true;
        assert!(can_drown(&resources, &[eye]));
    }
    #[test]
    fn registered_eye_contact_runs_the_existing_air_damage_and_refill_path() {
        let registry = registry();
        let resources = resources(&registry);
        let mut m = mob();
        let contacts = sample(&m, &resources, |p| (p.y <= 1).then_some(BlockStateId(1)));
        let submerged = can_drown(&resources, &contacts);
        let mut events = Vec::new();
        m.air = 300;
        super::super::ai::tick_common_air(&mut m, submerged, &mut events);
        assert_eq!(m.air, 299);
        assert_eq!(m.health, 10.0);
        assert!(events.is_empty());
        m.air = -19;
        super::super::ai::tick_common_air(&mut m, submerged, &mut events);
        assert_eq!(m.air, 0);
        assert_eq!(m.health, 8.0);
        assert!(matches!(
            events.as_slice(),
            [MobEvent::Drown { mob: 1, .. }]
        ));
        super::super::ai::tick_common_air(&mut m, false, &mut events);
        assert_eq!(m.air, 4);
        assert_eq!(events.len(), 1);
        assert!(!m.fluids.in_water && !m.fluids.eye_in_water);
    }
    #[test]
    fn common_air_respects_water_breathing_nautilus_and_inherent_breathing() {
        for mode in 0..4 {
            let mut m = mob();
            let mut events = Vec::new();
            m.air = 100;
            match mode {
                0 => m.add_effect("minecraft:water_breathing", 100, 0),
                1 => m.add_effect("minecraft:breath_of_the_nautilus", 100, 0),
                2 => Arc::make_mut(&mut m.kind).flags.breathes_water = true,
                _ => {}
            }
            super::super::ai::tick_common_air(&mut m, true, &mut events);
            assert_eq!(
                m.air,
                match mode {
                    0 | 2 => 104,
                    1 => 100,
                    _ => 99,
                }
            );
            assert_eq!(m.health, 10.0);
            assert!(events.is_empty());
            // Leaving the fluid refills even while the nautilus effect remains.
            super::super::ai::tick_common_air(&mut m, false, &mut events);
            assert_eq!(
                m.air,
                match mode {
                    0 | 2 => 108,
                    1 => 104,
                    _ => 103,
                }
            );
        }
    }
    #[test]
    fn drowning_damage_keeps_native_invulnerability_and_air_cap() {
        let mut m = mob();
        let mut events = Vec::new();
        m.invulnerable_flag = true;
        m.air = -19;
        super::super::ai::tick_common_air(&mut m, true, &mut events);
        assert_eq!(m.air, 0);
        assert_eq!(m.health, 10.0);
        assert!(matches!(events.as_slice(), [MobEvent::Drown { .. }]));
        m.air = 299;
        super::super::ai::tick_common_air(&mut m, false, &mut events);
        assert_eq!(m.air, 300);
    }
    #[test]
    fn only_positive_body_contact_with_an_extinguishing_family_clears_fire() {
        let registry = registry();
        let resources = resources_with_effect(&registry, Some("minecraft:poison"));
        let mut m = mob();
        let mut events = Vec::new();
        m.fire_ticks = 200;
        let no = sample(&m, &resources, |p| (p.y == 0).then_some(BlockStateId(16)));
        assert!(
            !can_extinguish(&resources, &no),
            "different nonextinguishing family"
        );
        assert!(!can_extinguish(&resources, &[]));
        let yes = sample(&m, &resources, |p| (p.y == 0).then_some(BlockStateId(1)));
        assert!(!yes[0].eyes_inside, "only the body needs to be immersed");
        assert!(can_extinguish(&resources, &yes));
        let mut expected = m.random.clone();
        let pitch = 1.6 + (expected.next_float() - expected.next_float()) * 0.4;
        super::super::ai::extinguish_environment_fire(
            &mut m,
            can_extinguish(&resources, &yes),
            &mut events,
        );
        assert_eq!(m.fire_ticks, 0);
        assert!(
            matches!(events.as_slice(), [MobEvent::Sound("entity.generic.extinguish_fire", _, volume, got)]
            if *volume == 0.7 && *got == pitch)
        );
        assert_eq!(m.random.next_float(), expected.next_float());
        // Remaining in the liquid causes neither another sound nor another draw.
        let mut expected = m.random.clone();
        super::super::ai::extinguish_environment_fire(&mut m, true, &mut events);
        assert_eq!(events.len(), 1);
        assert_eq!(m.random.next_float(), expected.next_float());
        // Being above a shallow source of that same family is not body contact.
        m.position.y = 0.2;
        let shallow = sample(&m, &resources, |p| (p.y == 0).then_some(BlockStateId(7)));
        assert!(!can_extinguish(&resources, &shallow));
        assert!(!can_extinguish(
            &resources,
            &sample(&m, &resources, |_| None)
        ));
        // Negative immunity countdowns are never promoted to zero by a wet tick.
        m.fire_ticks = -3;
        super::super::ai::extinguish_environment_fire(&mut m, true, &mut events);
        assert_eq!(m.fire_ticks, -3);
    }
    #[test]
    fn active_contact_curative_policy_survives_weaker_updates_and_generic_curing() {
        let mut m = mob();
        m.add_effect_with_curatives("poison", 100, 0, true);
        m.add_effect("speed", 200, 0);
        m.add_effect("poison", 50, 0);
        m.cure_effects();
        assert_eq!(m.extra.effects, [("minecraft:poison", 100, 0)]);
        assert_eq!(m.extra.no_curative_effects, ["minecraft:poison"]);
        // A longer ordinary active instance takes back ordinary curability.
        m.add_effect("poison", 200, 0);
        assert!(m.extra.no_curative_effects.is_empty());
        m.cure_effects();
        assert!(m.extra.effects.is_empty());
        // Stronger instances replace the active policy even when shorter.
        m.add_effect_with_curatives("slowness", 100, 0, true);
        m.add_effect("slowness", 20, 1);
        assert!(m.extra.no_curative_effects.is_empty());
        // Infinite protected instances are not weakened by finite refreshes.
        m.add_effect_with_curatives("blindness", -1, 0, true);
        m.add_effect("blindness", 100, 0);
        m.cure_effects();
        assert_eq!(m.extra.effects, [("minecraft:blindness", -1, 0)]);
    }
    #[test]
    fn fire_resistance_rejects_lava_and_afterburn_but_not_unrelated_damage() {
        for kind in [DamageKind::Lava, DamageKind::OnFire] {
            let mut m = mob();
            m.add_effect("minecraft:fire_resistance", 100, 0);
            assert!(!m.damage(3.0, Damage::of(kind)));
            assert_eq!(m.health, 10.0);
            assert_eq!(m.invulnerable, 0);
            assert!(m.outbox.is_empty());
            assert!(m.damage(2.0, Damage::of(DamageKind::Generic)));
            assert_eq!(m.health, 8.0);
        }
    }
    #[test]
    fn shallow_surface_and_unloaded_samples_do_not_ignite() {
        let registry = registry();
        let resources = resources(&registry);
        let mut m = mob();
        // The level-seven surface is only 1/9 high, below these feet.
        m.position.y = 0.2;
        let contacts = sample(&m, &resources, |p| (p.y == 0).then_some(BlockStateId(7)));
        assert!(contacts.is_empty());
        let mut random = LegacyRandom::new(8);
        inside(
            &mut m,
            Some(&resources),
            BlockStateId(7),
            &contacts,
            &mut random,
        );
        assert_eq!(m.fire_ticks, 0);
        assert_eq!(m.health, 10.0);
        assert!(sample(&m, &resources, |_| None).is_empty());
    }
}
