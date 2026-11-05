// Fluid burn/effect contact adapts Tinkers' Construct, Copyright (c) 2022 SlimeKnights.
// MIT License. Full notice and source are in THIRD_PARTY_NOTICES.md.
// Changed: explicit family policy, bounded loaded-only sampling, host mob damage.
//! Registered liquid block contact for authoritative mobs, without native aliases.
use super::{Damage, DamageKind, Mob, MobEvent, MobWorld};
use horizon_core::{geometry::Vec3, position::BlockPos, random::RandomSource};
use horizon_world::{
    registered_fluids::{contact::Contact, resources::RegisteredFluidResources},
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
        let rows: Vec<_> = ["test:hot", "test:other"]
            .into_iter()
            .flat_map(|name| {
                (0..16).map(move |level| {
                json!([name, false, true, "", {"id":name,"properties":{"level":level.to_string()}}])
            })
            })
            .collect();
        BlockRegistry::from_facts_json(
            json!({"states":rows,"defaults":{"test:hot":0,"test:other":16}})
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
        let effect = effect
            .map(|id| json!({"id":id,"duration_ticks":100,"amplifier":0,"no_curatives":true}));
        let burn = effect.is_none().then(|| json!({"seconds":10,"damage":3.0}));
        let fluids: Vec<_> = ["hot", "other"].into_iter().map(|name| json!({
            "source":format!("test:{name}"),"flowing":format!("test:flowing_{name}"),
            "block":format!("test:{name}"),"bucket":format!("test:{name}_bucket"),
            "policy":{"drop_off":1,"slope_distance":4,"tick_delay":5,"source_conversion":false},
            "visual":{"still":"test:still","flowing":"test:flow","tint":16777215,"emission":0},
            "interaction":{"travel":{"acceleration":0.02,"horizontal_drag":0.8,"vertical_drag":0.8,"gravity_scale":0.0625,"jump":0.04},
                "motion_scale":0.0,"can_swim":false,"can_drown":false,"extinguishes":name == "hot" && effect.is_some(),
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
