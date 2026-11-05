// Burning-liquid contact follows TConstruct, Copyright (c) 2022 SlimeKnights.
// MIT License; source and license are in THIRD_PARTY_NOTICES.md.
// Changed: registered family policies and the host dropped-item damage model.
//! Authoritative item burn contact, without native fluid aliases.
use super::ItemEntity;
use horizon_core::{geometry::Vec3, position::BlockPos};
use horizon_world::{
    registered_fluids::{interaction::BurnContact, resources::RegisteredFluidResources},
    registry::BlockStateId,
};

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
    let bounds = item.bounds().deflate(Vec3::new(0.001, 0.001, 0.001));
    let eyes = Vec3::new(
        item.position.x,
        item.position.y + f64::from(0.2125f32),
        item.position.z,
    );
    let Ok(contacts) = resources.catalog.contacts(bounds, eyes, 8192, &mut block) else {
        return 0;
    };
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
        let fluids:Vec<_> = ["hot", "cool"].map(|name| json!({
            "source":format!("test:{name}"),"flowing":format!("test:flowing_{name}"),
            "block":format!("test:{name}"),"bucket":format!("test:{name}_bucket"),
            "policy":{"drop_off":1,"slope_distance":4,"tick_delay":5,"source_conversion":false},
            "visual":{"still":"test:still","flowing":"test:flow","tint":16777215,"emission":0},
            "interaction":{"travel":{"acceleration":0.02,"horizontal_drag":0.8,"vertical_drag":0.8,"gravity_scale":0.0625,"jump":0.04},
              "motion_scale":0.0,"can_swim":false,"can_drown":false,"extinguishes":false,"fall_distance_scale":0.5,
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
