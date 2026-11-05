// Fluid fire policy follows TConstruct, Copyright (c) 2022 SlimeKnights.
// MIT License; source and license are in THIRD_PARTY_NOTICES.md.
// Changed: registered family policies and host dropped-item/armor-stand adapters.
//! Authoritative entity fluid forces and item/armor-stand fire contact, without native aliases.
use super::{ArmorStand, ItemEntity};
use horizon_core::{geometry::Vec3, position::BlockPos};
use horizon_world::{
    entity_fluid::CurrentTarget,
    registered_fluids::{
        contact::Contact,
        interaction::{BurnContact, TravelPolicy},
        resources::RegisteredFluidResources,
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
    if sample(item, resources, block)
        .is_some_and(|contacts| extinguishing_contact(resources, &contacts))
    {
        item.fire_ticks = 0;
    }
}

/// Stand fire clearing uses body contact, even when invisible or NoGravity.
/// Markers retain their exclusion from registered fluid immersion.
pub(super) fn extinguish_stand(
    stand: &mut ArmorStand,
    resources: &RegisteredFluidResources<'_>,
    block: impl FnMut(BlockPos) -> Option<BlockStateId>,
) {
    if stand.fire_ticks <= 0 {
        return;
    }
    if extinguishing_contact(resources, &stand_contacts(stand, resources, block)) {
        stand.fire_ticks = 0;
    }
}

/// BurningLiquidBlock ignites before asking the target to accept lava damage.
/// The existing stand model rejects that direct damage, so there is no accepted
/// hit or burn sound here. Ordinary ongoing fire damage stays in environment_tick.
pub(super) fn ignite_stand(
    stand: &mut ArmorStand,
    resources: &RegisteredFluidResources<'_>,
    block: impl FnMut(BlockPos) -> Option<BlockStateId>,
) {
    // As in the native stand block-contact path, marker/NoGravity stands do
    // not run inside-block callbacks. Invisibility alone does not block the
    // source's unconditional ignition step, though it rejects fire damage.
    if stand.marker || stand.no_gravity || stand.health <= 0.5 {
        return;
    }
    for contact in stand_contacts(stand, resources, block) {
        if contact.height > 0.0
            && let Some(burn) = resources.interaction(contact.source).and_then(|p| p.burn)
        {
            // Repeated cells of one family have the same idempotent max;
            // rejected lava hits cause no additional per-cell side effects.
            stand.fire_ticks = stand.fire_ticks.max(i32::from(burn.seconds) * 20);
        }
    }
}

pub(super) fn stand_contacts(
    stand: &ArmorStand,
    resources: &RegisteredFluidResources<'_>,
    block: impl FnMut(BlockPos) -> Option<BlockStateId>,
) -> Vec<Contact> {
    let Some(bounds) = stand.registered_fluid_box() else {
        return Vec::new();
    };
    resources
        .catalog
        .contacts(
            bounds.deflate(Vec3::new(0.001, 0.001, 0.001)),
            stand.registered_fluid_eyes(),
            8192,
            block,
        )
        .unwrap_or_default()
}

/// Once per positive family at the fluid refresh, not once per sampled cell.
/// This is the stand's existing fall accumulator for landing enchantments.
pub(super) fn scale_stand_fall(
    stand: &mut ArmorStand,
    resources: &RegisteredFluidResources<'_>,
    contacts: &[Contact],
) {
    for contact in contacts {
        if contact.height > 0.0
            && let Some(policy) = resources.interaction(contact.source)
        {
            stand.enchantment_fall_distance *= policy.fall_distance_scale;
        }
    }
}

/// Deepest configured body family wins; equal depths keep the last encounter,
/// matching the existing registered player and common-mob travel adapters.
pub(super) fn stand_travel(
    resources: &RegisteredFluidResources<'_>,
    contacts: &[Contact],
) -> Option<TravelPolicy> {
    contacts
        .iter()
        .filter(|contact| contact.height > 0.0)
        .filter_map(|contact| {
            resources
                .interaction(contact.source)
                .map(|policy| (contact.height, policy.travel))
        })
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, policy)| policy)
}

/// Finish the collision-resolved, block-pushed move with declared drag and the
/// existing living-fluid falling adjustment. Stands do not sprint or provide
/// movement input; marker/NoGravity early exits remain in their tick method.
pub(super) fn stand_travel_velocity(policy: TravelPolicy, velocity: Vec3, falling: bool) -> Vec3 {
    let mut damped = policy.damp(velocity, 0.0);
    let gravity = 0.08 * policy.gravity_scale * 16.0;
    if gravity != 0.0 {
        damped.y = if falling
            && (damped.y - 0.005).abs() >= 0.003
            && (damped.y - gravity / 16.0).abs() < 0.003
        {
            -0.003
        } else {
            damped.y - gravity / 16.0
        };
    }
    damped
}

fn extinguishing_contact(resources: &RegisteredFluidResources<'_>, contacts: &[Contact]) -> bool {
    contacts.iter().any(|contact| {
        contact.height > 0.0
            && resources
                .interaction(contact.source)
                .is_some_and(|p| p.extinguishes)
    })
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
        resources_with_fall_scales(registry, hot_extinguishes, scales, [0.5, 0.5])
    }
    fn resources_with_fall_scales(
        registry: &BlockRegistry,
        hot_extinguishes: bool,
        scales: [f64; 2],
        fall_scales: [f64; 2],
    ) -> RegisteredFluidResources<'_> {
        resources_with_travel(registry, hot_extinguishes, scales, fall_scales, [0.8, 0.8])
    }
    fn resources_with_travel(
        registry: &BlockRegistry,
        hot_extinguishes: bool,
        scales: [f64; 2],
        fall_scales: [f64; 2],
        drag: [f64; 2],
    ) -> RegisteredFluidResources<'_> {
        let fluids:Vec<_> = ["hot", "cool"].map(|name| json!({
            "source":format!("test:{name}"),"flowing":format!("test:flowing_{name}"),
            "block":format!("test:{name}"),"bucket":format!("test:{name}_bucket"),
            "policy":{"drop_off":1,"slope_distance":4,"tick_delay":5,"source_conversion":false},
            "visual":{"still":"test:still","flowing":"test:flow","tint":16777215,"emission":0},
            "interaction":{"travel":{"acceleration":0.02,"horizontal_drag":drag[usize::from(name == "cool")],"vertical_drag":0.8,"gravity_scale":0.0625,"jump":0.04},
              "motion_scale":scales[usize::from(name == "cool")],"can_swim":false,"can_drown":false,"extinguishes":name == "cool" || hot_extinguishes,"fall_distance_scale":fall_scales[usize::from(name == "cool")],
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
    #[test]
    fn stand_currents_respect_marker_and_small_body_geometry_not_no_gravity() {
        let registry = registry();
        let r = resources_with_motion(&registry, false, [0.01, 0.0]);
        for (small, marker, no_gravity, expected) in [
            (false, false, false, 0.01),
            (true, false, false, 0.0),
            (false, true, false, 0.0),
            (false, false, true, 0.01),
        ] {
            let mut stand = crate::entities::ArmorStand::new(7, Vec3::new(0.5, 0.0, 0.5), 0.0);
            stand.small = small;
            stand.marker = marker;
            stand.no_gravity = no_gravity;
            let mut target = Velocity(stand.velocity);
            if let Some(bounds) = stand.registered_fluid_box() {
                // A fluid in the second vertical cell touches a normal stand
                // but remains above a small one's body. Markers have no query.
                let contacts = r
                    .catalog
                    .contacts_with_flow(
                        bounds.deflate(Vec3::new(0.001, 0.001, 0.001)),
                        stand.position,
                        8192,
                        |p| (p.y == 1).then_some(BlockStateId(0)),
                        |_| Ok(Vec3::new(1.0, 0.0, 0.0)),
                    )
                    .unwrap();
                apply_currents(&mut target, &r, &contacts).unwrap();
            }
            assert_eq!(
                target.0,
                Vec3::new(expected, 0.0, 0.0),
                "small={small}, marker={marker}, no_gravity={no_gravity}"
            );
            assert_eq!(stand.health, 20.0);
        }
    }
    fn orb(y: f64) -> crate::entities::ExperienceOrb {
        let position = Vec3::new(0.5, y, 0.5);
        crate::entities::ExperienceOrb {
            id: 8,
            position,
            previous: position,
            velocity: Vec3::ZERO,
            value: 7,
            age: 42,
            on_ground: false,
            count: 3,
            ticks: 9,
            health: 5,
            no_gravity: false,
        }
    }
    #[test]
    fn orb_current_sampling_uses_its_half_block_body_without_changing_rewards() {
        let registry = registry();
        let r = resources_with_motion(&registry, false, [0.01, 0.0]);
        let mut orb = orb(0.75);
        // A 0.5-high orb reaches the next cell; a 0.25-high dropped item does not.
        let sample = |bounds: horizon_core::geometry::Aabb| {
            r.catalog
                .contacts_with_flow(
                    bounds.deflate(Vec3::new(0.001, 0.001, 0.001)),
                    orb.position,
                    8192,
                    |p| (p.y == 1).then_some(BlockStateId(0)),
                    |_| Ok(Vec3::new(3.0, 0.0, 4.0)),
                )
                .unwrap()
        };
        let contacts = sample(orb.bounds());
        let item_contacts = sample(item(0.75).bounds());
        let mut target = Velocity(orb.velocity);
        apply_currents(&mut target, &r, &contacts).unwrap();
        orb.velocity = target.0;
        assert_eq!(orb.velocity, Vec3::new(0.006, 0.0, 0.008));
        let mut item_target = Velocity(Vec3::ZERO);
        apply_currents(&mut item_target, &r, &item_contacts).unwrap();
        assert_eq!(item_target.0, Vec3::ZERO);
        assert_eq!(
            (orb.value, orb.count, orb.age, orb.ticks, orb.health),
            (7, 3, 42, 9, 5)
        );
    }
    #[test]
    fn orb_currents_preserve_no_gravity_mobility_and_missing_surface_opt_out() {
        let registry = registry();
        let r = resources_with_motion(&registry, false, [0.01, 0.0]);
        let mut orb = orb(0.0);
        orb.no_gravity = true;
        assert!(crate::entity_rules::EntityRules::is_pushed_by_fluid(&orb));
        let sample = |y, state| {
            let at = Vec3::new(0.5, y, 0.5);
            r.catalog
                .contacts_with_flow(
                    crate::entities::ExperienceOrb::box_at(at)
                        .deflate(Vec3::new(0.001, 0.001, 0.001)),
                    at,
                    8192,
                    |p| if p.y == 0 { state } else { None },
                    |_| Ok(Vec3::new(0.0, 1.0, 0.0)),
                )
                .unwrap()
        };
        for (y, state, expected) in [
            (0.0, Some(BlockStateId(7)), 0.01),
            (0.2, Some(BlockStateId(7)), 0.0),
            (0.0, Some(BlockStateId(16)), 0.0),
            (0.0, None, 0.0),
        ] {
            let mut target = Velocity(Vec3::ZERO);
            apply_currents(&mut target, &r, &sample(y, state)).unwrap();
            assert_eq!(target.0, Vec3::new(0.0, expected, 0.0));
        }
        assert_eq!((orb.value, orb.count), (7, 3));
    }
    #[test]
    fn stand_burning_contact_ignites_without_direct_lava_damage_or_other_changes() {
        let registry = registry();
        let r = resources(&registry);
        for invisible in [false, true] {
            let mut stand = ArmorStand::new(7, Vec3::new(0.5, 0.0, 0.5), 0.0);
            stand.invisible = invisible;
            stand.health = 6.0;
            stand.velocity = Vec3::new(0.1, -0.2, 0.3);
            stand.equipment[0] = Some(item(0.0).stack);
            ignite_stand(&mut stand, &r, source);
            assert_eq!((stand.fire_ticks, stand.health), (200, 6.0));
            assert_eq!(stand.velocity, Vec3::new(0.1, -0.2, 0.3));
            assert_eq!(stand.equipment[0].as_ref().unwrap().count, 1);
            stand.fire_ticks = 400;
            ignite_stand(&mut stand, &r, source);
            assert_eq!((stand.fire_ticks, stand.health), (400, 6.0));
        }
        for (marker, no_gravity, health) in [
            (true, false, 20.0),
            (false, true, 20.0),
            (false, false, 0.5),
        ] {
            let mut stand = ArmorStand::new(7, Vec3::new(0.5, 0.0, 0.5), 0.0);
            stand.marker = marker;
            stand.no_gravity = no_gravity;
            stand.health = health;
            ignite_stand(&mut stand, &r, |_| panic!("excluded stand must not query"));
            assert_eq!((stand.fire_ticks, stand.health), (0, health));
        }
    }
    #[test]
    fn stand_burning_contact_requires_positive_burning_family_body_height() {
        let registry = registry();
        let r = resources(&registry);
        for (small, y, fluid_y, state, expected) in [
            (false, 0.0, 1, 0, 200),
            (true, 0.0, 1, 0, 0),
            (false, 0.9, 0, 0, 0),
            (false, 0.0, 0, 7, 200),
            (false, 0.2, 0, 7, 0),
            (false, 0.0, 0, 16, 0),
        ] {
            let mut stand = ArmorStand::new(7, Vec3::new(0.5, y, 0.5), 0.0);
            stand.small = small;
            ignite_stand(&mut stand, &r, |p| {
                (p.y == fluid_y).then_some(BlockStateId(state))
            });
            assert_eq!((stand.fire_ticks, stand.health), (expected, 20.0));
        }
        let mut stand = ArmorStand::new(7, Vec3::new(0.5, 0.0, 0.5), 0.0);
        ignite_stand(&mut stand, &r, |_| None);
        assert_eq!(stand.fire_ticks, 0);
        stand.position.y = f64::NAN;
        ignite_stand(&mut stand, &r, |_| panic!("invalid body"));
        assert_eq!(stand.fire_ticks, 0);
    }
    #[test]
    fn stand_extinguishing_then_burning_can_reignite_in_mixed_or_magma_like_fluids() {
        let registry = registry();
        let r = resources(&registry);
        let mut stand = ArmorStand::new(7, Vec3::new(1.0, 0.0, 0.5), 0.0);
        stand.fire_ticks = 80;
        let mixed = |p: BlockPos| (p.y == 0).then_some(BlockStateId(if p.x == 0 { 16 } else { 0 }));
        extinguish_stand(&mut stand, &r, mixed);
        assert_eq!(stand.fire_ticks, 0);
        ignite_stand(&mut stand, &r, mixed);
        assert_eq!((stand.fire_ticks, stand.health), (200, 20.0));
        let magma_like = resources_with_hot_extinguishing(&registry, true);
        stand.position.x = 0.5;
        extinguish_stand(&mut stand, &magma_like, source);
        assert_eq!(stand.fire_ticks, 0);
        ignite_stand(&mut stand, &magma_like, source);
        assert_eq!((stand.fire_ticks, stand.health), (200, 20.0));
    }
    #[test]
    fn stand_travel_chooses_deepest_body_family_and_last_equal_depth() {
        let registry = registry();
        let r = resources_with_travel(&registry, false, [0.0, 0.0], [0.5, 0.5], [0.8, 0.6]);
        let mut hot = current(&r, "test:hot", Vec3::ZERO);
        let mut cool = current(&r, "test:cool", Vec3::ZERO);
        hot.height = 0.7;
        cool.height = 0.2;
        assert_eq!(stand_travel(&r, &[hot, cool]).unwrap().horizontal_drag, 0.8);
        cool.height = 0.9;
        assert_eq!(stand_travel(&r, &[hot, cool]).unwrap().horizontal_drag, 0.6);
        cool.height = hot.height;
        assert_eq!(stand_travel(&r, &[hot, cool]).unwrap().horizontal_drag, 0.6);
        assert_eq!(stand_travel(&r, &[cool, hot]).unwrap().horizontal_drag, 0.8);
        hot.height = 0.0;
        hot.eyes_inside = true;
        assert!(stand_travel(&r, &[hot]).is_none());
        assert!(stand_travel(&r, &[]).is_none());
    }
    #[test]
    fn stand_travel_uses_declared_drag_and_preserves_collision_stopped_axes() {
        let policy = TravelPolicy {
            acceleration: 0.02,
            horizontal_drag: 0.5,
            vertical_drag: 0.25,
            gravity_scale: 0.125,
            jump: 0.04,
        };
        let velocity = stand_travel_velocity(policy, Vec3::new(0.0, -0.1, 0.6), true);
        assert_eq!(velocity.x, 0.0);
        assert!((velocity.y + 0.035).abs() < 1e-12);
        assert_eq!(velocity.z, 0.3);
        let no_gravity_scale = TravelPolicy {
            gravity_scale: 0.0,
            ..policy
        };
        let velocity = stand_travel_velocity(no_gravity_scale, Vec3::new(0.4, -0.1, 0.0), true);
        assert_eq!(velocity, Vec3::new(0.2, -0.025, 0.0));
    }
    #[test]
    fn stand_travel_keeps_the_premove_falling_snap_and_surface_gate() {
        let registry = registry();
        let r = resources(&registry);
        let policy = TravelPolicy {
            gravity_scale: 0.125,
            ..stand_travel(&r, &[current(&r, "test:hot", Vec3::ZERO)]).unwrap()
        };
        // A block push can turn the velocity upward after falling was sampled.
        assert_eq!(
            stand_travel_velocity(policy, Vec3::new(0.0, 0.0125, 0.0), true).y,
            -0.003
        );
        assert!(
            stand_travel_velocity(policy, Vec3::new(0.0, 0.0125, 0.0), false)
                .y
                .abs()
                < 1e-12
        );
        let mut stand = ArmorStand::new(7, Vec3::new(0.5, 0.2, 0.5), 0.0);
        let contacts = stand_contacts(&stand, &r, |p| (p.y == 0).then_some(BlockStateId(7)));
        assert!(stand_travel(&r, &contacts).is_none());
        stand.position.y = 0.0;
        let contacts = stand_contacts(&stand, &r, |p| (p.y == 0).then_some(BlockStateId(7)));
        assert!(stand_travel(&r, &contacts).is_some());
        stand.marker = true;
        let contacts = stand_contacts(&stand, &r, |_| panic!("marker has no travel contact"));
        assert!(stand_travel(&r, &contacts).is_none());
    }
    #[test]
    fn stand_fall_scales_once_per_family_even_across_source_and_flowing_cells() {
        let registry = registry();
        let r = resources(&registry); // Zero motion scales do not disable fall policy.
        let mut stand = ArmorStand::new(7, Vec3::new(1.0, 0.0, 0.5), 0.0);
        stand.enchantment_fall_distance = 16.0;
        stand.velocity = Vec3::new(0.0, -0.4, 0.0);
        let contacts = stand_contacts(&stand, &r, |p| {
            (p.y == 0).then_some(BlockStateId(if p.x == 0 { 0 } else { 7 }))
        });
        scale_stand_fall(&mut stand, &r, &contacts);
        assert_eq!(stand.enchantment_fall_distance, 8.0);
        let mixed = stand_contacts(&stand, &r, |p| {
            (p.y == 0).then_some(BlockStateId(if p.x == 0 { 0 } else { 16 }))
        });
        scale_stand_fall(&mut stand, &r, &mixed);
        assert_eq!(stand.enchantment_fall_distance, 2.0);
        scale_stand_fall(&mut stand, &r, &mixed);
        assert_eq!(
            stand.enchantment_fall_distance, 0.5,
            "each refresh applies its policies"
        );
        assert_eq!(stand.velocity, Vec3::new(0.0, -0.4, 0.0));
        assert_eq!(stand.health, 20.0);
    }
    #[test]
    fn stand_fall_policy_handles_zero_identity_and_eye_only_contacts() {
        let registry = registry();
        let r = resources_with_fall_scales(&registry, false, [0.0, 0.0], [0.0, 1.0]);
        let mut stand = ArmorStand::new(7, Vec3::new(0.5, 0.0, 0.5), 0.0);
        stand.enchantment_fall_distance = 8.0;
        let cool = current(&r, "test:cool", Vec3::ZERO);
        scale_stand_fall(&mut stand, &r, &[cool]);
        assert_eq!(stand.enchantment_fall_distance, 8.0);
        let mut eye = current(&r, "test:hot", Vec3::ZERO);
        eye.height = 0.0;
        eye.eyes_inside = true;
        scale_stand_fall(&mut stand, &r, &[eye]);
        assert_eq!(stand.enchantment_fall_distance, 8.0);
        scale_stand_fall(&mut stand, &r, &[current(&r, "test:hot", Vec3::ZERO)]);
        assert_eq!(stand.enchantment_fall_distance, 0.0);
    }
    #[test]
    fn stand_fall_sampling_respects_small_marker_and_missing_geometry() {
        let registry = registry();
        let r = resources(&registry);
        for (small, marker, no_gravity, expected) in [
            (false, false, false, 4.0),
            (true, false, false, 8.0),
            (false, true, false, 8.0),
            (false, false, true, 4.0),
        ] {
            let mut stand = ArmorStand::new(7, Vec3::new(0.5, 0.0, 0.5), 0.0);
            stand.small = small;
            stand.marker = marker;
            stand.no_gravity = no_gravity;
            stand.enchantment_fall_distance = 8.0;
            let contacts = stand_contacts(&stand, &r, |p| {
                assert!(!marker, "marker needs no query");
                (p.y == 1).then_some(BlockStateId(0))
            });
            scale_stand_fall(&mut stand, &r, &contacts);
            assert_eq!(stand.enchantment_fall_distance, expected);
            let absent = stand_contacts(&stand, &r, |_| None);
            scale_stand_fall(&mut stand, &r, &absent);
            assert_eq!(stand.enchantment_fall_distance, expected);
            stand.position.y = f64::NAN;
            let invalid = stand_contacts(&stand, &r, |_| panic!("invalid query"));
            scale_stand_fall(&mut stand, &r, &invalid);
            assert_eq!(stand.enchantment_fall_distance, expected);
        }
    }
    #[test]
    fn stand_extinguishing_preserves_health_equipment_and_movement() {
        let registry = registry();
        let r = resources(&registry);
        for (invisible, no_gravity) in [(false, false), (true, false), (false, true)] {
            let mut stand = ArmorStand::new(7, Vec3::new(0.5, 0.0, 0.5), 23.0);
            stand.fire_ticks = 200;
            stand.health = 6.0;
            stand.invisible = invisible;
            stand.no_gravity = no_gravity;
            stand.velocity = Vec3::new(0.1, -0.2, 0.3);
            stand.enchantment_fall_distance = 8.0;
            stand.equipment[0] = Some(item(0.0).stack);
            extinguish_stand(&mut stand, &r, cool);
            assert_eq!((stand.fire_ticks, stand.health), (0, 6.0));
            assert_eq!(stand.position, Vec3::new(0.5, 0.0, 0.5));
            assert_eq!(stand.velocity, Vec3::new(0.1, -0.2, 0.3));
            assert_eq!(stand.enchantment_fall_distance, 8.0);
            assert_eq!(
                stand.equipment[0].as_ref().unwrap().item.as_ref(),
                "minecraft:oak_planks"
            );
            assert_eq!(stand.equipment[0].as_ref().unwrap().count, 1);
            extinguish_stand(&mut stand, &r, |_| panic!("unlit stand needs no query"));
        }
    }
    #[test]
    fn stand_extinguishing_respects_small_marker_and_flowing_surface_geometry() {
        let registry = registry();
        let r = resources(&registry);
        for (small, y, fluid_y, state, expected) in [
            (false, 0.0, 1, 16, 0),
            (true, 0.0, 1, 16, 200),
            (true, 0.0, 0, 16, 0),
            (false, 0.9, 0, 16, 200),
            (false, 0.0, 0, 23, 0),
            (false, 0.2, 0, 23, 200),
            (false, 0.0, 0, 0, 200),
        ] {
            let mut stand = ArmorStand::new(7, Vec3::new(0.5, y, 0.5), 0.0);
            stand.small = small;
            stand.fire_ticks = 200;
            extinguish_stand(&mut stand, &r, |p| {
                (p.y == fluid_y).then_some(BlockStateId(state))
            });
            assert_eq!(
                stand.fire_ticks, expected,
                "small={small}, y={y}, fluid_y={fluid_y}, state={state}"
            );
        }
        let mut marker = ArmorStand::new(7, Vec3::new(0.5, 0.0, 0.5), 0.0);
        marker.marker = true;
        marker.fire_ticks = 200;
        extinguish_stand(&mut marker, &r, |_| {
            panic!("marker has no registered immersion")
        });
        assert_eq!(marker.fire_ticks, 200);
    }
    #[test]
    fn stand_extinguishing_rejects_missing_invalid_and_eye_only_contacts() {
        let registry = registry();
        let r = resources(&registry);
        let mut stand = ArmorStand::new(7, Vec3::new(0.5, 0.0, 0.5), 0.0);
        stand.fire_ticks = 200;
        extinguish_stand(&mut stand, &r, |_| None);
        assert_eq!(stand.fire_ticks, 200);
        stand.position.y = f64::NAN;
        extinguish_stand(&mut stand, &r, |_| panic!("invalid body has no query"));
        assert_eq!(stand.fire_ticks, 200);
        let mut eye = current(&r, "test:cool", Vec3::ZERO);
        eye.height = 0.0;
        eye.eyes_inside = true;
        assert!(!extinguishing_contact(&r, &[eye]));
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
