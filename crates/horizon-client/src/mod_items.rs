//! Frozen server main hands for the experimental read-only Mod API.
use crate::items::ItemStack;
use horizon_mods::{ITEM_TEXT_MAX, ItemReader, ServerHeldStack};
use horizon_world::nbt::Tag;

#[derive(Debug)]
pub struct ServerItems(pub Vec<(String, ItemStack)>);

impl ItemReader for ServerItems {
    fn main_hand(&self, player: &str, path: &[String]) -> Result<Option<ServerHeldStack>, String> {
        let Some((_, stack)) = self.0.iter().find(|(id, _)| id == player) else {
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let snapshot = ServerItems(vec![("alice".into(), stack.clone())]);
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
                ServerItems(vec![("alice".into(), stack.clone())])
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
            ServerItems(vec![("alice".into(), stack)])
                .main_hand("alice", &["state".into()])
                .unwrap_err()
                .contains("Unicode")
        );
    }
}
