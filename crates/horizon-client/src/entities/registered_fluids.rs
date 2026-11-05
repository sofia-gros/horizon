// Fluid fire policy follows TConstruct, Copyright (c) 2022 SlimeKnights.
// MIT License; source and license are in THIRD_PARTY_NOTICES.md.
// Changed: registered family policies and the host dropped-item damage model.
//! Authoritative item fluid forces and fire contact, without native aliases.
use super::ItemEntity;
use horizon_core::{geometry::Vec3, position::BlockPos};
use horizon_world::{
    entity_fluid::CurrentTarget,
    registered_fluids::{
        contact::Contact, interaction::BurnContact, resources::RegisteredFluidResources,
    },
    registry::BlockStateId,
};

/// The existing entity target retains non-player normalization, minimum drift
/// and finite-delta checks. Fire resistance does not imply current immunity.
pub(super) fn apply_currents<W: CurrentTarget>(
    target: &mut W,
    resources: &RegisteredFluidResources<'_>,
    contacts: &[Contact],
) -> Result<(), W::Error> {
    for contact in contacts {
        if contact.height > 0.0
            && let Some(policy) = resources.interaction(contact.source)
            && policy.motion_scale > 0.0
        {
            contact.current.apply_to(target, policy.motion_scale)?;
        }
    }
    Ok(())
}

/// Clear burning before the ordinary on-fire tick, without changing health.
/// Native water/fire immunity and native-mod cancellation remain at the caller.
pub(super) fn extinguish(
    item: &mut ItemEntity,
    resources: &RegisteredFluidResources<'_>,
    block: impl FnMut(BlockPos) -> Option<BlockStateId>,
) {
    if item.fire_ticks <= 0 {
        return;
    }
    if sample(item, resources, block).is_some_and(|contacts| {
        contacts.iter().any(|contact| {
            contact.height > 0.0
                && resources
                    .interaction(contact.source)
                    .is_some_and(|p| p.extinguishes)
        })
    }) {
        item.fire_ticks = 0;
    }
}

fn sample(
    item: &ItemEntity,
    resources: &RegisteredFluidResources<'_>,
    block: impl FnMut(BlockPos) -> Option<BlockStateId>,
) -> Option<Vec<Contact>> {
    let bounds = item.bounds().deflate(Vec3::new(0.001, 0.001, 0.001));
    let eyes = Vec3::new(
        item.position.x,
        item.position.y + f64::from(0.2125f32),
        item.position.z,
    );
    resources.catalog.contacts(bounds, eyes, 8192, block).ok()
}

pub(super) fn burn_contacts(
    item: &mut ItemEntity,
    resources: &RegisteredFluidResources<'_>,
    fire_immune: bool,
    lava_resistant: bool,
    mut block: impl FnMut(BlockPos) -> Option<BlockStateId>,
) -> u32 {
    if item.health <= 0 || fire_immune {
        return 0;
    }
    let Some(contacts) = sample(item, resources, &mut block) else {
        return 0;
    };
    let bounds = item.bounds().deflate(Vec3::new(0.001, 0.001, 0.001));
    // The sampler has validated finite coordinates and the query budget. An
    // item's fixed 0.25-wide box visits at most eight body cells, never eye-only
    // or above-surface probe cells. Source entityInside can hit once per block.
    let lo = bounds.min_position();
    let hi = bounds.max_position();
    let mut accepted = 0;
    for x in lo.x.floor() as i32..hi.x.ceil() as i32 {
        for y in lo.y.floor() as i32..hi.y.ceil() as i32 {
            for z in lo.z.floor() as i32..hi.z.ceil() as i32 {
                let Some(state) = block(BlockPos::new(x, y, z)) else {
                    continue;
                };
                let Some(burn) = resources
                    .interaction_at_contact(state, &contacts)
                    .and_then(|p| p.burn)
                else {
                    continue;
                };
                if burn_item(item, burn, lava_resistant) {
                    accepted += 1;
                }
                if item.health <= 0 {
                    return accepted;
                }
            }
        }
    }
    accepted
}

fn burn_item(item: &mut ItemEntity, burn: BurnContact, lava_resistant: bool) -> bool {
    // Ignition precedes the damage-source resistance check, as in the source
    // block callback. Categorical fire immunity was checked before sampling.
    item.fire_ticks = item.fire_ticks.max(i32::from(burn.seconds) * 20);
    if lava_resistant {
        return false;
    }
    item.health = (item.health as f32 - burn.damage) as i32;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::ItemStack;
    use horizon_world::registry::BlockRegistry;
    use serde_json::json;

    fn registry() -> BlockRegistry {
        let mut rows = Vec::new();
        for name in ["test:hot", "test:cool"] {
            for level in 0..16 {
                rows.push(json!([name,false,true,"",{"id":name,"properties":{"level":level.to_string()}}]));
            }
        }
        BlockRegistry::from_facts_json(
            json!({"states":rows,"defaults":{"test:hot":0,"test:cool":16}})
                .to_string()
                .as_bytes(),
        )
        .unwrap()
    }
    fn resources(registry: &BlockRegistry) -> RegisteredFluidResources<'_> {
        resources_with_hot_extinguishing(registry, false)
    }
    fn resources_with_hot_extinguishing(
        registry: &BlockRegistry,
        hot_extinguishes: bool,
    ) -> RegisteredFluidResources<'_> {
        resources_with_motion(registry, hot_extinguishes, [0.0, 0.0])
    }
    fn resources_with_motion(
        registry: &BlockRegistry,
        hot_extinguishes: bool,
        scales: [f64; 2],
    ) -> RegisteredFluidResources<'_> {
        let fluids:Vec<_> = ["hot", "cool"].map(|name| json!({
            "source":format!("test:{name}"),"flowing":format!("test:flowing_{name}"),
            "block":format!("test:{name}"),"bucket":format!("test:{name}_bucket"),
            "policy":{"drop_off":1,"slope_distance":4,"tick_delay":5,"source_conversion":false},
            "visual":{"still":"test:still","flowing":"test:flow","tint":16777215,"emission":0},
            "interaction":{"travel":{"acceleration":0.02,"horizontal_drag":0.8,"vertical_drag":0.8,"gravity_scale":0.0625,"jump":0.04},
              "motion_scale":scales[usize::from(name == "cool")],"can_swim":false,"can_drown":false,"extinguishes":name == "cool" || hot_extinguishes,"fall_distance_scale":0.5,
              "burn":if name == "hot" { json!({"seconds":10,"damage":2.0}) } else { json!(null) }}
        })).into_iter().collect();
        RegisteredFluidResources::from_reader(
            registry,
            json!({"format":1,"fluids":fluids}).to_string().as_bytes(),
        )
        .unwrap()
    }
    fn item(y: f64) -> ItemEntity {
        let position = Vec3::new(0.5, y, 0.5);
        ItemEntity {
            health: 5,
            fire_ticks: 0,
            id: 1,
            stack: ItemStack {
                item: "minecraft:oak_planks".into(),
                count: 1,
                damage: 0,
                extra: None,
            },
            thrower: None,
            target: None,
            position,
            previous: position,
            velocity: Vec3::ZERO,
            on_ground: false,
            age: 0,
            pickup_delay: 0,
            bob_offset: 0.0,
            ticks: 0,
            needs_sync: false,
            stuck: Vec3::ZERO,
            no_gravity: false,
            vertical_collision: false,
            horizontal_collision: false,
        }
    }
    fn source(p: BlockPos) -> Option<BlockStateId> {
        (p.y == 0).then_some(BlockStateId(0))
    }
    struct Velocity(Vec3);
    impl CurrentTarget for Velocity {
        type Error = std::convert::Infallible;
        fn is_player_class(&self) -> bool {
            false
        }
        fn velocity(&mut self) -> Result<Vec3, Self::Error> {
            Ok(self.0)
        }
        fn add_delta_movement(&mut self, delta: Vec3) -> Result<(), Self::Error> {
            self.0 = Vec3::new(self.0.x + delta.x, self.0.y + delta.y, self.0.z + delta.z);
            Ok(())
        }
    }
    fn current(r: &RegisteredFluidResources<'_>, name: &str, vector: Vec3) -> Contact {
        let mut current = horizon_world::entity_fluid::CurrentAccumulator::default();
        current.height = 0.8;
        current.accumulate(vector);
        Contact {
            source: r.catalog.id(name).unwrap(),
            height: 0.8,
            eyes_inside: false,
            current,
        }
    }
    #[test]
    fn item_currents_use_each_familys_scale_and_nonplayer_normalization() {
        let registry = registry();
        let r = resources_with_motion(&registry, false, [0.01, 0.02]);
        let contacts = [
            current(&r, "test:flowing_hot", Vec3::new(3.0, 4.0, 0.0)),
            current(&r, "test:cool", Vec3::new(0.0, 0.0, -5.0)),
        ];
        let mut target = Velocity(Vec3::new(0.1, 0.2, 0.3));
        apply_currents(&mut target, &r, &contacts).unwrap();
        assert!((target.0.x - 0.106).abs() < 1e-12);
        assert!((target.0.y - 0.208).abs() < 1e-12);
        assert!((target.0.z - 0.28).abs() < 1e-12);
    }
    #[test]
    fn item_currents_keep_stationary_minimum_drift_and_vertical_direction() {
        let registry = registry();
        let r = resources_with_motion(&registry, false, [0.0023333333333333335, 0.0]);
        for (vector, wanted) in [
            (Vec3::new(0.0, 1.0, 0.0), 0.0045000000000000005),
            (Vec3::new(0.0, -1.0, 0.0), -0.0045000000000000005),
        ] {
            let mut target = Velocity(Vec3::ZERO);
            apply_currents(&mut target, &r, &[current(&r, "test:hot", vector)]).unwrap();
            assert_eq!(target.0, Vec3::new(0.0, wanted, 0.0));
        }
    }
    #[test]
    fn eye_only_zero_scale_and_canceled_currents_leave_velocity_unchanged() {
        let registry = registry();
        let r = resources_with_motion(&registry, false, [0.01, 0.0]);
        let mut eye = current(&r, "test:hot", Vec3::new(1.0, 0.0, 0.0));
        eye.height = 0.0;
        eye.eyes_inside = true;
        let zero = current(&r, "test:cool", Vec3::new(1.0, 0.0, 0.0));
        let mut canceled = current(&r, "test:hot", Vec3::new(1.0, 0.0, 0.0));
        canceled.current.accumulate(Vec3::new(-1.0, 0.0, 0.0));
        let old = Vec3::new(0.1, 0.2, 0.3);
        let mut target = Velocity(old);
        apply_currents(&mut target, &r, &[eye, zero, canceled]).unwrap();
        assert_eq!(target.0, old);
    }
    fn cool(p: BlockPos) -> Option<BlockStateId> {
        (p.y == 0).then_some(BlockStateId(16))
    }
    #[test]
    fn cool_contact_clears_fire_without_healing_or_changing_item_metadata() {
        let registry = registry();
        let r = resources(&registry);
        let mut drop = item(0.8);
        drop.fire_ticks = 200;
        drop.health = 2;
        drop.age = 77;
        drop.pickup_delay = 14;
        drop.thrower = Some([1, 2, 3, 4]);
        drop.target = Some([5, 6, 7, 8]);
        extinguish(&mut drop, &r, cool);
        assert_eq!(
            (drop.fire_ticks, drop.health, drop.age, drop.pickup_delay),
            (0, 2, 77, 14)
        );
        assert_eq!(drop.thrower, Some([1, 2, 3, 4]));
        assert_eq!(drop.target, Some([5, 6, 7, 8]));
        assert_eq!(drop.stack.item.as_ref(), "minecraft:oak_planks");
        assert_eq!(drop.stack.count, 1);
        extinguish(&mut drop, &r, |_| panic!("unlit items need no query"));
    }
    #[test]
    fn only_the_declared_extinguishing_body_surface_puts_out_fire() {
        let registry = registry();
        let r = resources(&registry);
        for (y, state, expected) in [
            (0.0, 16, 0),
            (0.9, 16, 200),
            (0.0, 23, 0),
            (0.2, 23, 200),
            (0.0, 0, 200),
        ] {
            let mut drop = item(y);
            drop.fire_ticks = 200;
            extinguish(&mut drop, &r, |p| (p.y == 0).then_some(BlockStateId(state)));
            assert_eq!(drop.fire_ticks, expected, "y={y}, state={state}");
        }
        let mut absent = item(0.0);
        absent.fire_ticks = 200;
        extinguish(&mut absent, &r, |_| None);
        assert_eq!(absent.fire_ticks, 200);
        absent.position.y = f64::NAN;
        extinguish(&mut absent, &r, |_| panic!("invalid bounds"));
        assert_eq!(absent.fire_ticks, 200);
    }
    #[test]
    fn cool_then_hot_contact_can_reignite_without_reviving_a_drop() {
        let registry = registry();
        let r = resources(&registry);
        let mut drop = item(0.0);
        drop.position.x = 1.0;
        drop.fire_ticks = 200;
        let mixed = |p: BlockPos| (p.y == 0).then_some(BlockStateId(if p.x == 0 { 16 } else { 0 }));
        extinguish(&mut drop, &r, mixed);
        assert_eq!(drop.fire_ticks, 0);
        assert_eq!(burn_contacts(&mut drop, &r, false, false, mixed), 1);
        assert_eq!((drop.fire_ticks, drop.health), (200, 3));
        // Magma's source policy can both extinguish and burn in the same family.
        let magma_like = resources_with_hot_extinguishing(&registry, true);
        drop.position.x = 0.5;
        extinguish(&mut drop, &magma_like, source);
        assert_eq!((drop.fire_ticks, drop.health), (0, 3));
        assert_eq!(
            burn_contacts(&mut drop, &magma_like, false, false, source),
            1
        );
        assert_eq!((drop.fire_ticks, drop.health), (200, 1));
        drop.health = 0;
        extinguish(&mut drop, &r, cool);
        assert_eq!((drop.fire_ticks, drop.health), (0, 0));
    }
    #[test]
    fn contact_ignites_and_damages_each_tick_including_the_lethal_hit() {
        let registry = registry();
        let r = resources(&registry);
        let mut drop = item(0.0);
        for health in [3, 1, -1] {
            assert_eq!(burn_contacts(&mut drop, &r, false, false, source), 1);
            assert_eq!(drop.health, health);
            assert_eq!(drop.fire_ticks, 200);
        }
        assert_eq!(
            burn_contacts(&mut drop, &r, false, false, |_| panic!("dead item")),
            0
        );
        assert_eq!(drop.health, -1);
    }
    #[test]
    fn fire_immunity_and_lava_resistance_keep_distinct_ignition_rules() {
        let registry = registry();
        let r = resources(&registry);
        let mut immune = item(0.0);
        assert_eq!(
            burn_contacts(&mut immune, &r, true, false, |_| panic!("immune")),
            0
        );
        assert_eq!((immune.health, immune.fire_ticks), (5, 0));
        let mut resistant = item(0.0);
        assert_eq!(burn_contacts(&mut resistant, &r, false, true, source), 0);
        assert_eq!((resistant.health, resistant.fire_ticks), (5, 200));
        resistant.fire_ticks = 400;
        assert_eq!(burn_contacts(&mut resistant, &r, false, true, source), 0);
        assert_eq!(resistant.fire_ticks, 400);
    }
    #[test]
    fn only_positive_body_contact_of_the_burning_family_counts() {
        let registry = registry();
        let r = resources(&registry);
        let mut shallow = item(0.8); // Native point sample at y + 0.1 would miss this.
        assert_eq!(burn_contacts(&mut shallow, &r, false, false, source), 1);
        let mut above = item(0.9);
        assert_eq!(burn_contacts(&mut above, &r, false, false, source), 0);
        let mut cool = item(0.0);
        assert_eq!(
            burn_contacts(&mut cool, &r, false, false, |p| (p.y == 0)
                .then_some(BlockStateId(16))),
            0
        );
        let mut flowing = item(0.2);
        assert_eq!(
            burn_contacts(&mut flowing, &r, false, false, |p| (p.y == 0)
                .then_some(BlockStateId(7))),
            0
        );
        flowing.position.y = 0.0;
        assert_eq!(
            burn_contacts(&mut flowing, &r, false, false, |p| (p.y == 0)
                .then_some(BlockStateId(7))),
            1
        );
    }
    #[test]
    fn touching_two_cells_can_hurt_twice_but_missing_or_invalid_cells_do_not() {
        let registry = registry();
        let r = resources(&registry);
        let mut drop = item(0.0);
        drop.position.x = 1.0;
        assert_eq!(burn_contacts(&mut drop, &r, false, false, source), 2);
        assert_eq!(drop.health, 1);
        let mut absent = item(0.0);
        assert_eq!(burn_contacts(&mut absent, &r, false, false, |_| None), 0);
        assert_eq!(absent.health, 5);
        absent.position.y = f64::NAN;
        assert_eq!(
            burn_contacts(&mut absent, &r, false, false, |_| panic!("invalid")),
            0
        );
    }
}
