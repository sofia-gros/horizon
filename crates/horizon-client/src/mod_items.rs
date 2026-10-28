//! Frozen main-hand views and checked string edits for the server Mod APIs.
use crate::items::ItemStack;
use horizon_mods::{ITEM_TEXT_MAX, ItemReader, ServerHeldStack};
use horizon_world::nbt::Tag;

#[derive(Debug)]
pub struct ServerItems(pub Vec<(String, usize, ItemStack)>);

impl ItemReader for ServerItems {
    fn main_hand(&self, player: &str, path: &[String]) -> Result<Option<ServerHeldStack>, String> {
        let Some((_, _, stack)) = self.0.iter().find(|(id, _, _)| id == player) else {
            return Ok(None);
        };
        let mut tag = if path.is_empty() {
            None
        } else {
            stack
                .data()
                .and_then(|data| data.other("minecraft:custom_data"))
        };
        for key in path {
            tag = tag.and_then(|tag| tag.get(key));
        }
        let custom_text = match tag {
            Some(Tag::String(text)) => {
                // At most 3 UTF-8 bytes per UTF-16 unit; bound allocation even
                // before converting, then enforce the exact UTF-8 limit.
                if text.0.len() > ITEM_TEXT_MAX {
                    return Err("item custom string exceeds 4096 bytes".into());
                }
                let text = String::from_utf16(&text.0)
                    .map_err(|_| "item custom string is not valid Unicode")?;
                if text.len() > ITEM_TEXT_MAX {
                    return Err("item custom string exceeds 4096 bytes".into());
                }
                Some(text)
            }
            _ => None,
        };
        Ok(Some(ServerHeldStack {
            item: stack.item.to_string(),
            count: stack.count,
            damage: stack.damage,
            custom_text,
        }))
    }
}

impl ServerItems {
    /// Stage a checked main-hand edit and consumption from another inventory
    /// slot, then commit both together. The caller must serialize all access to
    /// this inventory from snapshot capture through commit, and capture
    /// `ingredient` at the same time as `self`. This function does not track
    /// revisions and must not be used with a preview that can outlive a write.
    /// This does not authorize a recipe, select a player, or validate a menu.
    pub fn apply_consuming(
        &self,
        selected: usize,
        inventory: &mut [Option<ItemStack>],
        ingredient_slot: usize,
        ingredient: &ItemStack,
        count: u32,
        edit: &horizon_mods::ItemEdit,
    ) -> Result<(), String> {
        if selected == ingredient_slot || count == 0 || count > ingredient.count {
            return Err("invalid ingredient consumption".into());
        }
        let held = inventory.get(selected).ok_or("invalid selected slot")?;
        let input = inventory
            .get(ingredient_slot)
            .ok_or("invalid ingredient slot")?;
        if input.as_ref() != Some(ingredient) {
            return Err("ingredient changed; edit rejected".into());
        }
        let mut changed = held.clone();
        self.apply(selected, &mut changed, edit)?;
        let mut remainder = ingredient.clone();
        remainder.count -= count;
        // All checks and fallible transformations are complete before writes.
        inventory[selected] = changed;
        inventory[ingredient_slot] = (remainder.count != 0).then_some(remainder);
        Ok(())
    }

    /// Apply associated text and mining settings as one checked stack replacement.
    pub fn apply(
        &self,
        selected: usize,
        cell: &mut Option<ItemStack>,
        edit: &horizon_mods::ItemEdit,
    ) -> Result<(), String> {
        let Some(profile) = &edit.profile else {
            return self.replace(&edit.player, selected, cell, &edit.path, &edit.text);
        };
        horizon_mods::validate_tool_profile(profile)?;
        let mut changed = cell.clone();
        self.replace(&edit.player, selected, &mut changed, &edit.path, &edit.text)?;
        let stack = changed.as_mut().ok_or("empty main hand")?;
        if let Some((damage, max_damage)) = edit.durability {
            if stack.count != 1
                || max_damage == 0
                || max_damage > i32::MAX as u32
                || damage > max_damage
            {
                return Err("invalid item durability".into());
            }
            stack.damage = damage;
            stack
                .data_mut()
                .set_other("minecraft:max_damage", Some(Tag::Int(max_damage as i32)));
        }
        let mut custom = stack
            .data()
            .and_then(|d| d.other("minecraft:custom_data"))
            .cloned()
            .ok_or("missing custom data")?;
        let Tag::Compound(data) = &mut custom else {
            return Err("custom data is not a compound".into());
        };
        if edit
            .melee
            .as_ref()
            .is_some_and(|m| !horizon_mods::valid_melee(m))
        {
            return Err("invalid melee profile".into());
        }
        let prior = match data.get(&"horizon:tool_control".into()) {
            None => None,
            Some(old) => {
                let Some(Tag::String(owner)) = old.get("owner") else {
                    return Err("invalid existing tool profile owner".into());
                };
                if String::from_utf16(&owner.0).ok().as_deref() != Some(edit.owner.as_str()) {
                    return Err("tool profile belongs to another mod".into());
                }
                old.get("melee").cloned()
            }
        };
        let melee = edit
            .melee
            .as_ref()
            .map(|m| {
                Tag::compound([
                    ("damage_bonus", Tag::Float(m.damage_bonus.to_bits())),
                    (
                        "damage_multiplier",
                        Tag::Float(m.damage_multiplier.to_bits()),
                    ),
                    ("attack_speed", Tag::Float(m.attack_speed.to_bits())),
                    ("enabled", Tag::Byte(i8::from(m.enabled))),
                ])
            })
            .or(prior);
        let mut control = vec![
            ("schema", Tag::Int(1)),
            ("owner", Tag::String(edit.owner.as_str().into())),
            (
                "mining_enabled",
                Tag::Byte(i8::from(profile.mining_enabled)),
            ),
        ];
        if let Some(melee) = melee {
            control.push(("melee", melee));
        }
        data.insert("horizon:tool_control".into(), Tag::compound(control));
        let rules = profile
            .rules
            .iter()
            .map(|r| {
                let blocks = if r.blocks.len() == 1 && r.blocks[0].starts_with('#') {
                    Tag::String(r.blocks[0].as_str().into())
                } else {
                    Tag::List(
                        r.blocks
                            .iter()
                            .map(|b| Tag::String(b.as_str().into()))
                            .collect(),
                    )
                };
                let mut fields = vec![("blocks", blocks)];
                if let Some(speed) = r.speed {
                    fields.push(("speed", Tag::Float(speed.to_bits())));
                }
                if let Some(correct) = r.correct_for_drops {
                    fields.push(("correct_for_drops", Tag::Byte(i8::from(correct))));
                }
                Tag::compound(fields)
            })
            .collect();
        let tool = Tag::compound([
            ("rules", Tag::List(rules)),
            (
                "default_mining_speed",
                Tag::Float(profile.default_speed.to_bits()),
            ),
            (
                "damage_per_block",
                Tag::Int(profile.damage_per_block as i32),
            ),
            (
                "can_destroy_blocks_in_creative",
                Tag::Byte(i8::from(profile.can_destroy_in_creative)),
            ),
        ]);
        let data = stack.data_mut();
        data.set_other("minecraft:custom_data", Some(custom));
        data.set_other("minecraft:tool", Some(tool));
        data.set_other(
            "minecraft:unbreakable",
            profile
                .unbreakable
                .then(|| Tag::compound([] as [(&str, Tag); 0])),
        );
        *cell = changed;
        Ok(())
    }

    /// Full stack comparison includes count, vanilla damage and every component.
    /// Only an existing string leaf is replaced; all other values stay intact.
    pub fn replace(
        &self,
        player: &str,
        selected: usize,
        cell: &mut Option<ItemStack>,
        path: &[String],
        text: &str,
    ) -> Result<(), String> {
        let (_, slot, expected) = self
            .0
            .iter()
            .find(|(id, _, _)| id == player)
            .ok_or("no captured main hand")?;
        if *slot != selected || cell.as_ref() != Some(expected) {
            return Err("main hand changed; edit rejected".into());
        }
        if path.is_empty()
            || path
                .first()
                .is_some_and(|key| key == "horizon:tool_control")
            || path.len() > 8
            || path.iter().any(|key| key.is_empty() || key.len() > 128)
            || text.len() > ITEM_TEXT_MAX
        {
            return Err("invalid item edit".into());
        }
        let mut data = expected
            .data()
            .and_then(|data| data.other("minecraft:custom_data"))
            .cloned()
            .ok_or("missing custom data")?;
        let mut leaf = &mut data;
        for key in path {
            let Tag::Compound(compound) = leaf else {
                return Err("custom path is not a compound".into());
            };
            leaf = compound
                .get_mut(&key.as_str().into())
                .ok_or("custom path is missing")?;
        }
        if !matches!(leaf, Tag::String(_)) {
            return Err("custom path is not a string".into());
        }
        *leaf = Tag::String(text.into());
        cell.as_mut()
            .ok_or("empty main hand")?
            .data_mut()
            .set_other("minecraft:custom_data", Some(data));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consuming_edit_checks_both_stacks_before_committing() {
        let mut tool = ItemStack::new("example:tool", 1);
        tool.data_mut().set_other(
            "minecraft:custom_data",
            Some(Tag::compound([
                ("state", Tag::String("old".into())),
                ("foreign", Tag::Int(91)),
            ])),
        );
        let mut material = ItemStack::new("example:material", 3);
        material
            .data_mut()
            .set_other("example:foreign", Some(Tag::Int(42)));
        let snapshot = std::sync::Arc::new(ServerItems(vec![("alice".into(), 0, tool.clone())]));
        let edit = horizon_mods::ItemEdit {
            owner: "example_mod".into(),
            player: "alice".into(),
            path: vec!["state".into()],
            text: "new".into(),
            snapshot: snapshot.clone(),
            durability: None,
            melee: None,
            profile: None,
        };
        let original = vec![Some(tool), Some(material.clone())];
        for (slot, count) in [(0, 1), (1, 0), (1, 4), (2, 1)] {
            let mut inventory = original.clone();
            assert!(
                snapshot
                    .apply_consuming(0, &mut inventory, slot, &material, count, &edit)
                    .is_err()
            );
            assert_eq!(inventory, original);
        }
        let mut inventory = original.clone();
        inventory[1].as_mut().unwrap().damage = 1;
        let before = inventory.clone();
        assert!(
            snapshot
                .apply_consuming(0, &mut inventory, 1, &material, 2, &edit)
                .is_err()
        );
        assert_eq!(inventory, before);
        inventory = original.clone();
        inventory[0].as_mut().unwrap().damage = 1;
        let before = inventory.clone();
        assert!(
            snapshot
                .apply_consuming(0, &mut inventory, 1, &material, 2, &edit)
                .is_err()
        );
        assert_eq!(inventory, before);
        let mut exhausted = original.clone();
        snapshot
            .apply_consuming(0, &mut exhausted, 1, &material, 3, &edit)
            .unwrap();
        assert_eq!(exhausted[1], None);
        inventory = original;
        snapshot
            .apply_consuming(0, &mut inventory, 1, &material, 2, &edit)
            .unwrap();
        assert_eq!(inventory[1].as_ref().unwrap().count, 1);
        assert_eq!(
            inventory[1]
                .as_ref()
                .unwrap()
                .data()
                .unwrap()
                .other("example:foreign"),
            Some(&Tag::Int(42))
        );
        assert_eq!(
            inventory[0]
                .as_ref()
                .unwrap()
                .data()
                .unwrap()
                .other("minecraft:custom_data")
                .unwrap()
                .get("foreign"),
            Some(&Tag::Int(91))
        );
        assert_eq!(
            inventory[0]
                .as_ref()
                .unwrap()
                .data()
                .unwrap()
                .other("minecraft:custom_data")
                .unwrap()
                .get("state"),
            Some(&Tag::String("new".into()))
        );
        let after = inventory.clone();
        assert!(
            snapshot
                .apply_consuming(0, &mut inventory, 1, &material, 2, &edit)
                .is_err()
        );
        assert_eq!(inventory, after);
    }

    #[test]
    fn another_mod_cannot_take_over_an_existing_profile() {
        let mut tool = ItemStack::new("example:tool", 1);
        tool.data_mut().set_other(
            "minecraft:custom_data",
            Some(Tag::compound([
                ("state", Tag::String("old".into())),
                ("foreign", Tag::Int(7)),
            ])),
        );
        let first = std::sync::Arc::new(ServerItems(vec![("alice".into(), 0, tool.clone())]));
        let mut edit = horizon_mods::ItemEdit {
            owner: "mod_a".into(),
            player: "alice".into(),
            path: vec!["state".into()],
            text: "first".into(),
            snapshot: first.clone(),
            durability: None,
            melee: None,
            profile: Some(horizon_mods::ServerToolProfile {
                rules: vec![],
                default_speed: 1.0,
                damage_per_block: 0,
                can_destroy_in_creative: true,
                mining_enabled: true,
                unbreakable: false,
            }),
        };
        let mut cell = Some(tool);
        first.apply(0, &mut cell, &edit).unwrap();
        let captured = std::sync::Arc::new(ServerItems(vec![(
            "alice".into(),
            0,
            cell.clone().unwrap(),
        )]));
        edit.owner = "mod_b".into();
        edit.snapshot = captured.clone();
        edit.text = "second".into();
        let before = cell.clone();
        assert!(captured.apply(0, &mut cell, &edit).is_err());
        assert_eq!(cell, before);
        let path = vec!["horizon:tool_control".into(), "owner".into()];
        assert_eq!(
            captured
                .main_hand("alice", &path)
                .unwrap()
                .unwrap()
                .custom_text
                .as_deref(),
            Some("mod_a")
        );
        edit.owner = "mod_a".into();
        captured.apply(0, &mut cell, &edit).unwrap();
        let updated = ServerItems(vec![("alice".into(), 0, cell.unwrap())]);
        assert_eq!(
            updated
                .main_hand("alice", &path)
                .unwrap()
                .unwrap()
                .custom_text
                .as_deref(),
            Some("mod_a")
        );
    }

    #[test]
    fn malformed_profile_owner_rejects_edit_without_changes() {
        for control in [Tag::Int(1), Tag::compound([("owner", Tag::Int(5))])] {
            let mut tool = ItemStack::new("example:tool", 1);
            tool.data_mut().set_other(
                "minecraft:custom_data",
                Some(Tag::compound([
                    ("state", Tag::String("old".into())),
                    ("horizon:tool_control", control),
                ])),
            );
            let snapshot =
                std::sync::Arc::new(ServerItems(vec![("alice".into(), 0, tool.clone())]));
            let edit = horizon_mods::ItemEdit {
                owner: "mod_a".into(),
                player: "alice".into(),
                path: vec!["state".into()],
                text: "new".into(),
                snapshot: snapshot.clone(),
                durability: None,
                melee: None,
                profile: Some(horizon_mods::ServerToolProfile {
                    rules: vec![],
                    default_speed: 1.0,
                    damage_per_block: 0,
                    can_destroy_in_creative: true,
                    mining_enabled: true,
                    unbreakable: false,
                }),
            };
            let mut cell = Some(tool.clone());
            assert!(snapshot.apply(0, &mut cell, &edit).is_err());
            assert_eq!(cell, Some(tool));
        }
    }

    #[test]
    fn profiles_update_text_and_controls_atomically_and_keep_foreign_data() {
        let mut original = ItemStack::new("example:tool", 1);
        original.damage = 58;
        original.data_mut().set_other(
            "minecraft:custom_data",
            Some(Tag::compound([
                ("state", Tag::String("old".into())),
                ("foreign", Tag::Int(9)),
            ])),
        );
        let snapshot =
            std::sync::Arc::new(ServerItems(vec![("alice".into(), 0, original.clone())]));
        let mut edit = horizon_mods::ItemEdit {
            owner: "example_mod".into(),
            player: "alice".into(),
            path: vec!["state".into()],
            text: "new".into(),
            snapshot: snapshot.clone(),
            durability: None,
            melee: None,
            profile: Some(horizon_mods::ServerToolProfile {
                rules: vec![horizon_mods::ServerToolRule {
                    blocks: vec!["#minecraft:mineable/pickaxe".into()],
                    speed: Some(3.5),
                    correct_for_drops: Some(true),
                }],
                default_speed: 1.0,
                damage_per_block: 0,
                can_destroy_in_creative: true,
                mining_enabled: false,
                unbreakable: true,
            }),
        };
        edit.durability = Some((61, 60));
        let mut cell = Some(original.clone());
        assert!(snapshot.apply(0, &mut cell, &edit).is_err());
        assert_eq!(cell.as_ref(), Some(&original));
        edit.durability = Some((60, 85));
        edit.melee = Some(horizon_mods::ServerMeleeProfile {
            damage_bonus: 2.25,
            damage_multiplier: 1.1,
            attack_speed: 1.2,
            enabled: true,
        });
        edit.melee.as_mut().unwrap().attack_speed = f32::NAN;
        assert!(snapshot.apply(0, &mut cell, &edit).is_err());
        assert_eq!(cell.as_ref(), Some(&original));
        edit.melee.as_mut().unwrap().attack_speed = 1.2;
        edit.profile.as_mut().unwrap().default_speed = f32::NAN;
        assert!(snapshot.apply(0, &mut cell, &edit).is_err());
        assert_eq!(cell.as_ref(), Some(&original));
        edit.profile.as_mut().unwrap().default_speed = 1.0;
        assert!(snapshot.apply(1, &mut cell, &edit).is_err());
        assert_eq!(cell.as_ref(), Some(&original));
        snapshot.apply(0, &mut cell, &edit).unwrap();
        let changed = cell.as_ref().unwrap();
        assert_eq!((changed.count, changed.damage), (1, 60));
        assert_eq!(crate::item_tool::max_damage(changed, 59), 85);
        let melee = crate::item_tool::melee_profile(changed).unwrap();
        assert_eq!(
            (
                melee.damage_bonus,
                melee.damage_multiplier,
                melee.attack_speed,
                melee.enabled
            ),
            (2.25, 1.1, 1.2, true)
        );
        assert!(crate::item_tool::melee_profile(&original).is_none());
        assert!(crate::item_tool::unbreakable(changed));
        assert!(!crate::item_tool::mining_enabled(changed));
        assert_eq!(
            changed
                .data()
                .unwrap()
                .other("minecraft:custom_data")
                .unwrap()
                .get("foreign"),
            Some(&Tag::Int(9))
        );
        let readback = ServerItems(vec![("alice".into(), 0, changed.clone())]);
        assert_eq!(
            readback
                .main_hand("alice", &edit.path)
                .unwrap()
                .unwrap()
                .custom_text
                .as_deref(),
            Some("new")
        );
        assert!(snapshot.apply(0, &mut cell, &edit).is_err());
    }

    #[test]
    fn edits_compare_the_whole_stack_and_slot_and_preserve_unrelated_data() {
        let mut original = ItemStack::new("example:tool", 1);
        original.damage = 3;
        original.data_mut().set_other(
            "minecraft:custom_data",
            Some(Tag::compound([
                ("state", Tag::String("old".into())),
                ("foreign", Tag::Int(7)),
            ])),
        );
        let snapshot = ServerItems(vec![("alice".into(), 2, original.clone())]);
        let path = ["state".into()];
        let mut cell = Some(original.clone());
        assert!(
            snapshot
                .replace("alice", 1, &mut cell, &path, "new")
                .is_err()
        );
        cell.as_mut().unwrap().damage = 4;
        assert!(
            snapshot
                .replace("alice", 2, &mut cell, &path, "new")
                .is_err()
        );
        cell = Some(original.clone());
        cell.as_mut().unwrap().data_mut().custom_name = Some("different".into());
        assert!(
            snapshot
                .replace("alice", 2, &mut cell, &path, "new")
                .is_err()
        );
        cell = Some(original.clone());
        assert!(
            snapshot
                .replace("alice", 2, &mut cell, &["missing".into()], "new")
                .is_err()
        );
        assert_eq!(cell, Some(original.clone()));
        snapshot
            .replace("alice", 2, &mut cell, &path, "new")
            .unwrap();
        let item = cell.as_ref().unwrap();
        assert_eq!(
            (&item.item, item.count, item.damage),
            (&original.item, 1, 3)
        );
        let data = item.data().unwrap().other("minecraft:custom_data").unwrap();
        assert_eq!(data.get("foreign"), Some(&Tag::Int(7)));
        assert_eq!(data.get("state"), Some(&Tag::String("new".into())));
        assert_eq!(
            snapshot
                .main_hand("alice", &path)
                .unwrap()
                .unwrap()
                .custom_text
                .as_deref(),
            Some("old")
        );
        assert!(
            snapshot
                .replace("alice", 2, &mut cell, &path, "again")
                .is_err()
        );
    }

    #[test]
    fn main_hand_reads_a_frozen_stack_without_changing_or_truncating_it() {
        let mut stack = ItemStack::new("example:tool", 1);
        stack.damage = 7;
        stack.data_mut().set_other(
            "minecraft:custom_data",
            Some(Tag::compound([(
                "example",
                Tag::compound([("state", Tag::String("original".into()))]),
            )])),
        );
        let snapshot = ServerItems(vec![("alice".into(), 0, stack.clone())]);
        stack.data_mut().set_other(
            "minecraft:custom_data",
            Some(Tag::Compound(Default::default())),
        );
        let path = ["example".into(), "state".into()];
        let item = snapshot.main_hand("alice", &path).unwrap().unwrap();
        assert_eq!(
            (
                item.item.as_str(),
                item.count,
                item.damage,
                item.custom_text.as_deref()
            ),
            ("example:tool", 1, 7, Some("original"))
        );
        assert!(snapshot.main_hand("bob", &path).unwrap().is_none());
        assert!(
            snapshot
                .main_hand("alice", &[])
                .unwrap()
                .unwrap()
                .custom_text
                .is_none()
        );
        assert!(
            snapshot
                .main_hand("alice", &["missing".into()])
                .unwrap()
                .unwrap()
                .custom_text
                .is_none()
        );
        for text in ["x".repeat(4097), "木".repeat(1366)] {
            stack.data_mut().set_other(
                "minecraft:custom_data",
                Some(Tag::compound([(
                    "state",
                    Tag::String(text.as_str().into()),
                )])),
            );
            assert!(
                ServerItems(vec![("alice".into(), 0, stack.clone())])
                    .main_hand("alice", &["state".into()])
                    .is_err()
            );
        }
        stack.data_mut().set_other(
            "minecraft:custom_data",
            Some(Tag::compound([(
                "state",
                Tag::String(horizon_world::nbt::JavaString(vec![0xd800])),
            )])),
        );
        assert!(
            ServerItems(vec![("alice".into(), 0, stack)])
                .main_hand("alice", &["state".into()])
                .unwrap_err()
                .contains("Unicode")
        );
    }
}
