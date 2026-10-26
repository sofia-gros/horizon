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
        let mut custom = stack
            .data()
            .and_then(|d| d.other("minecraft:custom_data"))
            .cloned()
            .ok_or("missing custom data")?;
        let Tag::Compound(data) = &mut custom else {
            return Err("custom data is not a compound".into());
        };
        data.insert(
            "horizon:tool_control".into(),
            Tag::compound([
                ("schema", Tag::Int(1)),
                ("owner", Tag::String(edit.owner.as_str().into())),
                (
                    "mining_enabled",
                    Tag::Byte(i8::from(profile.mining_enabled)),
                ),
            ]),
        );
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
        let mut cell = Some(original.clone());
        edit.profile.as_mut().unwrap().default_speed = f32::NAN;
        assert!(snapshot.apply(0, &mut cell, &edit).is_err());
        assert_eq!(cell.as_ref(), Some(&original));
        edit.profile.as_mut().unwrap().default_speed = 1.0;
        assert!(snapshot.apply(1, &mut cell, &edit).is_err());
        assert_eq!(cell.as_ref(), Some(&original));
        snapshot.apply(0, &mut cell, &edit).unwrap();
        let changed = cell.as_ref().unwrap();
        assert_eq!((changed.count, changed.damage), (1, 58));
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
