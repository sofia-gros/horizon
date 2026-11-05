//! Resolved environmental damage for native mods, without inventing a vanilla source.
use horizon_world::damage_source::Catalog;

// Deliberately bounded to environmental sources. Reject unsupported vanilla tag
// semantics rather than silently treating a projectile, explosion, freeze, etc. as generic.
const TAGS: &[&str] = &[
    "minecraft:is_fire",
    "minecraft:witch_resistant_to",
    "minecraft:bypasses_armor",
    "minecraft:bypasses_invulnerability",
    "minecraft:bypasses_effects",
    "minecraft:bypasses_resistance",
    "minecraft:bypasses_enchantments",
    "minecraft:no_knockback",
    "minecraft:panic_causes",
    "minecraft:bypasses_wolf_armor",
];

/// A small immutable tag snapshot. Names come from native code; the catalog must
/// contain that exact name. No interning, leaked strings, global registration, or
/// cross-world mutable tag state. Retained combat records retain their original tags.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeDamage {
    name: &'static str,
    tags: u16,
    message_id: &'static str,
}
impl NativeDamage {
    pub fn from_catalog(
        catalog: &Catalog,
        name: &'static str,
        message_id: &'static str,
    ) -> Option<Self> {
        if name.starts_with("minecraft:") {
            return None;
        }
        let id = catalog.id(name)?;
        let definition = catalog.definition(id)?;
        if definition.scaling != horizon_world::damage_source::Scaling::Never
            || definition.death_message_type != "default"
            || definition.message_id != message_id
        {
            return None;
        }
        if catalog.tags().any(|tag| {
            tag.starts_with("minecraft:") && catalog.in_tag(id, tag) && !TAGS.contains(&tag)
        }) {
            return None;
        }
        let tags = TAGS.iter().enumerate().fold(0, |bits, (index, tag)| {
            bits | if catalog.in_tag(id, tag) {
                1 << index
            } else {
                0
            }
        });
        Some(Self {
            name,
            tags,
            message_id,
        })
    }
    pub fn name(self) -> &'static str {
        self.name
    }
    /// Verified default death-message key, retained with the combat entry.
    pub fn message_id(self) -> &'static str {
        self.message_id
    }
    pub(super) fn in_tag(self, tag: &str) -> bool {
        TAGS.iter()
            .position(|candidate| *candidate == tag)
            .is_some_and(|index| self.tags & (1 << index) != 0)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::mobs::{Damage, DamageKind, Mob, Species, sensor_scan_tests};
    use horizon_core::geometry::Vec3;
    use serde_json::json;
    use std::sync::Arc;

    fn catalog(tags: &[&str]) -> Catalog {
        let resources = tags
            .iter()
            .map(|tag| {
                (
                    (*tag).to_owned(),
                    json!({"values":["example:environment"]}).to_string().into(),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        Catalog::from_json(
            &json!({"data_version":horizon_world::DATA_VERSION,"types":[{
                "id":"example:environment","message_id":"example.environment",
                "exhaustion":"3dcccccd","scaling":"never","effects":"burning",
                "death_message_type":"default"
            }]}),
            &resources.into(),
        )
        .unwrap()
    }
    pub(crate) fn source(tags: &[&str]) -> NativeDamage {
        NativeDamage::from_catalog(&catalog(tags), "example:environment", "example.environment")
            .unwrap()
    }
    fn mob(species: Species, immune: bool, armor: f32) -> Mob {
        let mut kind = sensor_scan_tests::kind();
        let k = Arc::make_mut(&mut kind);
        k.species = species;
        k.flags.fire_immune = immune;
        k.armor = armor;
        Mob::new(8200, kind, Vec3::ZERO, 0.0)
    }
    #[test]
    fn unsupported_or_missing_sources_fail_closed_and_snapshots_do_not_change() {
        assert!(
            NativeDamage::from_catalog(&catalog(&[]), "example:missing", "example.missing")
                .is_none()
        );
        for unsupported in [
            "minecraft:is_projectile",
            "minecraft:bypasses_cooldown",
            "minecraft:is_freezing",
        ] {
            assert!(
                NativeDamage::from_catalog(
                    &catalog(&[unsupported]),
                    "example:environment",
                    "example.environment"
                )
                .is_none()
            );
        }
        let fire = source(&["minecraft:is_fire"]);
        let plain = source(&[]);
        assert!(Damage::of(DamageKind::Native(fire)).is_fire());
        assert!(!Damage::of(DamageKind::Native(plain)).is_fire());
        assert_eq!(fire.name(), "example:environment");
        assert_eq!(fire.message_id(), "example.environment");
        assert!(
            NativeDamage::from_catalog(&catalog(&[]), "example:environment", "wrong").is_none()
        );
    }
    #[test]
    fn native_damage_keeps_armor_cooldown_invulnerability_and_resistance() {
        let hit = Damage::of(DamageKind::Native(source(&["minecraft:no_knockback"])));
        let mut armored = mob(Species::Cow, false, 10.0);
        assert!(armored.damage(2.0, hit));
        assert!((armored.health - 8.72).abs() < 0.0001);
        assert!(!armored.damage(2.0, hit));
        assert_eq!(armored.velocity, Vec3::ZERO);
        assert_eq!(armored.last_damage.unwrap().0.kind, hit.kind);
        let mut protected = mob(Species::Cow, false, 0.0);
        protected.invulnerable_flag = true;
        assert!(!protected.damage(2.0, hit));
        protected.invulnerable_flag = false;
        protected
            .extra
            .effects
            .push(("minecraft:resistance".into(), 100, 0));
        assert!(protected.damage(2.0, hit));
        assert!((protected.health - 8.4).abs() < 0.0001);
    }
    #[test]
    fn environmental_sources_reject_attacker_and_nonfinite_amounts() {
        let mut entity = mob(Species::Cow, false, 0.0);
        let hit = Damage::of(DamageKind::Native(source(&[])));
        for amount in [f32::NAN, f32::INFINITY, -1.0] {
            assert!(!entity.damage(amount, hit));
        }
        let mut attack = hit;
        attack.attacker = Some(crate::mobs::Who::Player);
        assert!(!entity.damage(2.0, attack));
        assert_eq!(entity.health, 10.0);
        assert!(entity.last_damage.is_none());
    }
    #[test]
    fn fire_and_witch_tags_are_not_vanilla_magic_substitutes() {
        let fire = Damage::of(DamageKind::Native(source(&["minecraft:is_fire"])));
        let magic = Damage::of(DamageKind::Native(source(&[
            "minecraft:witch_resistant_to",
        ])));
        let mut blaze = mob(Species::Blaze, true, 0.0);
        assert!(!blaze.damage(2.0, fire));
        assert!(blaze.damage(2.0, magic));
        assert_eq!(blaze.health, 8.0);
        let mut resistant = mob(Species::Cow, false, 0.0);
        resistant
            .extra
            .effects
            .push(("minecraft:fire_resistance".into(), 100, 0));
        assert!(!resistant.damage(2.0, fire));
        let mut witch = mob(Species::Witch, false, 0.0);
        assert!(witch.damage(2.0, magic));
        assert!((witch.health - 9.7).abs() < 0.0001);
        let mut armored = mob(Species::Cow, false, 10.0);
        assert!(armored.damage(2.0, magic));
        assert!(
            (armored.health - 8.72).abs() < 0.0001,
            "custom magic does not imply bypasses_armor"
        );
    }
    #[test]
    fn native_damage_positive_bypass_tags_preserve_their_distinct_semantics() {
        for tag in [
            "minecraft:bypasses_effects",
            "minecraft:bypasses_resistance",
        ] {
            let mut entity = mob(Species::Cow, false, 0.0);
            entity
                .extra
                .effects
                .push(("minecraft:resistance".into(), 100, 0));
            assert!(entity.damage(2.0, Damage::of(DamageKind::Native(source(&[tag])))));
            assert_eq!(entity.health, 8.0);
        }
        let mut armored = mob(Species::Cow, false, 10.0);
        assert!(armored.damage(
            2.0,
            Damage::of(DamageKind::Native(source(&["minecraft:bypasses_armor"])))
        ));
        assert_eq!(armored.health, 8.0);
        let mut invulnerable = mob(Species::Cow, false, 0.0);
        invulnerable.invulnerable_flag = true;
        assert!(invulnerable.damage(
            2.0,
            Damage::of(DamageKind::Native(source(&[
                "minecraft:bypasses_invulnerability"
            ])))
        ));
        assert_eq!(invulnerable.health, 8.0);
        for bypass in [false, true] {
            let mut wolf = mob(Species::Wolf, false, 0.0);
            wolf.extra.head = Some("minecraft:wolf_armor".into());
            let tags = if bypass {
                &["minecraft:bypasses_wolf_armor"][..]
            } else {
                &[]
            };
            assert!(wolf.damage(2.0, Damage::of(DamageKind::Native(source(tags)))));
            assert_eq!(wolf.health, if bypass { 8.0 } else { 10.0 });
        }
        let mut gear = crate::mobs::gear::Gear::default();
        gear.upgrade(crate::mobs::gear::HEAD, "minecraft:protection", 4);
        assert_eq!(
            crate::mobs::gear::protection_points(&gear, DamageKind::Native(source(&[]))),
            4.0
        );
        for tag in [
            "minecraft:bypasses_enchantments",
            "minecraft:bypasses_effects",
        ] {
            assert_eq!(
                crate::mobs::gear::protection_points(&gear, DamageKind::Native(source(&[tag]))),
                0.0
            );
        }
    }
}
