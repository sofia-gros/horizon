//! Vanilla `minecraft:tool` stack overrides, shared by prediction and server mining.
use crate::items::ItemStack;
use horizon_protocol::{
    item::{Tool, ToolRule},
    packets::recipes::HolderSet,
};
use horizon_world::nbt::Tag;

/// Per-stack vanilla max-damage patch, otherwise the item's prototype.
pub fn max_damage(stack: &ItemStack, default: u32) -> u32 {
    match stack.data().and_then(|d| d.other("minecraft:max_damage")) {
        Some(Tag::Int(value)) if *value > 0 => *value as u32,
        _ => default,
    }
}

/// Host-owned control accompanies a Mod's Tool profile through custom_data.
pub fn mining_enabled(stack: &ItemStack) -> bool {
    let control = stack
        .data()
        .and_then(|d| d.other("minecraft:custom_data"))
        .and_then(|d| d.get("horizon:tool_control"));
    !control.is_some_and(|c| {
        matches!(c.get("schema"), Some(Tag::Int(1)))
            && matches!(c.get("mining_enabled"), Some(Tag::Byte(0)))
    })
}
pub fn unbreakable(stack: &ItemStack) -> bool {
    stack
        .data()
        .is_some_and(|d| d.other("minecraft:unbreakable").is_some())
}

fn number(tag: &Tag) -> Option<f32> {
    match tag {
        Tag::Float(v) => Some(f32::from_bits(*v)),
        Tag::Double(v) => Some(f64::from_bits(*v) as f32),
        Tag::Int(v) => Some(*v as f32),
        Tag::Byte(v) => Some(f32::from(*v)),
        _ => None,
    }
}
fn boolean(tag: &Tag) -> Option<bool> {
    if let Tag::Byte(v) = tag {
        Some(*v != 0)
    } else {
        None
    }
}
fn matches(blocks: &Tag, contains: &impl Fn(&str) -> bool) -> bool {
    match blocks {
        Tag::String(s) => contains(&s.to_string_lossy()),
        Tag::List(list) => list.iter().any(|b| matches(b, contains)),
        _ => false,
    }
}
fn property<'a>(tool: &'a Tag, key: &str, contains: &impl Fn(&str) -> bool) -> Option<&'a Tag> {
    let Tag::List(rules) = tool.get("rules")? else {
        return None;
    };
    rules.iter().find_map(|rule| {
        let value = rule.get(key)?;
        matches(rule.get("blocks")?, contains).then_some(value)
    })
}
pub fn mining_speed(tool: &Tag, contains: impl Fn(&str) -> bool) -> f32 {
    property(tool, "speed", &contains)
        .and_then(number)
        .or_else(|| tool.get("default_mining_speed").and_then(number))
        .unwrap_or(1.0)
}
pub fn correct_for_drops(tool: &Tag, contains: impl Fn(&str) -> bool) -> bool {
    property(tool, "correct_for_drops", &contains)
        .and_then(boolean)
        .unwrap_or(false)
}

/// Encode valid persistent Tool data using this connection's block registry.
pub fn to_network(tool: &Tag, block_id: impl Fn(&str) -> Option<i32>) -> Option<Tool> {
    fn blocks(tag: &Tag, id: &impl Fn(&str) -> Option<i32>) -> Option<HolderSet> {
        match tag {
            Tag::String(s) => {
                let s = s.to_string_lossy();
                if let Some(tag) = s.strip_prefix('#') {
                    Some(HolderSet::Tag(tag.into()))
                } else {
                    Some(HolderSet::Items(vec![id(&s)?]))
                }
            }
            Tag::List(list) => Some(HolderSet::Items(
                list.iter()
                    .map(|b| {
                        if let Tag::String(s) = b {
                            id(&s.to_string_lossy())
                        } else {
                            None
                        }
                    })
                    .collect::<Option<_>>()?,
            )),
            _ => None,
        }
    }
    let rules = match tool.get("rules") {
        Some(Tag::List(list)) => list
            .iter()
            .map(|rule| {
                Some(ToolRule {
                    blocks: blocks(rule.get("blocks")?, &block_id)?,
                    speed: rule.get("speed").and_then(number),
                    correct: rule.get("correct_for_drops").and_then(boolean),
                })
            })
            .collect::<Option<Vec<_>>>()?,
        None => Vec::new(),
        _ => return None,
    };
    Some(Tool {
        rules,
        default_speed: tool
            .get("default_mining_speed")
            .and_then(number)
            .unwrap_or(1.0),
        damage_per_block: match tool.get("damage_per_block") {
            Some(Tag::Int(n)) => *n,
            _ => 1,
        },
        can_destroy_in_creative: tool
            .get("can_destroy_blocks_in_creative")
            .and_then(boolean)
            .unwrap_or(true),
    })
}

pub fn from_network(tool: &Tool, block_name: impl Fn(i32) -> Option<String>) -> Option<Tag> {
    let rules = tool
        .rules
        .iter()
        .map(|rule| {
            let blocks = match &rule.blocks {
                HolderSet::Tag(tag) => Tag::String(format!("#{tag}").as_str().into()),
                HolderSet::Items(ids) => Tag::List(
                    ids.iter()
                        .map(|id| Some(Tag::String(block_name(*id)?.as_str().into())))
                        .collect::<Option<_>>()?,
                ),
            };
            let mut fields = vec![("blocks", blocks)];
            if let Some(speed) = rule.speed {
                fields.push(("speed", Tag::Float(speed.to_bits())));
            }
            if let Some(correct) = rule.correct {
                fields.push(("correct_for_drops", Tag::Byte(i8::from(correct))));
            }
            Some(Tag::compound(fields))
        })
        .collect::<Option<_>>()?;
    Some(Tag::compound([
        ("rules", Tag::List(rules)),
        (
            "default_mining_speed",
            Tag::Float(tool.default_speed.to_bits()),
        ),
        ("damage_per_block", Tag::Int(tool.damage_per_block)),
        (
            "can_destroy_blocks_in_creative",
            Tag::Byte(i8::from(tool.can_destroy_in_creative)),
        ),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_and_harvest_follow_independent_first_matching_rules() {
        let tool = Tag::compound([
            ("default_mining_speed", Tag::Float(0.25f32.to_bits())),
            (
                "rules",
                Tag::List(vec![
                    Tag::compound([
                        ("blocks", Tag::String("#minecraft:mineable/pickaxe".into())),
                        ("correct_for_drops", Tag::Byte(0)),
                    ]),
                    Tag::compound([
                        (
                            "blocks",
                            Tag::List(vec![Tag::String("minecraft:stone".into())]),
                        ),
                        ("speed", Tag::Float(3.5f32.to_bits())),
                        ("correct_for_drops", Tag::Byte(1)),
                    ]),
                    Tag::compound([
                        ("blocks", Tag::String("minecraft:stone".into())),
                        ("speed", Tag::Float(9.0f32.to_bits())),
                    ]),
                ]),
            ),
        ]);
        let matches = |b: &str| matches!(b, "#minecraft:mineable/pickaxe" | "minecraft:stone");
        assert_eq!(mining_speed(&tool, matches), 3.5);
        assert!(!correct_for_drops(&tool, matches));
        assert_eq!(mining_speed(&tool, |_| false), 0.25);
        assert!(!correct_for_drops(&tool, |_| false));
        let empty = Tag::compound([] as [(&str, Tag); 0]);
        assert_eq!(mining_speed(&empty, |_| true), 1.0);
        assert!(!correct_for_drops(&empty, |_| true));
    }

    #[test]
    fn named_and_direct_blocks_roundtrip_with_connection_ids() {
        for blocks in [
            HolderSet::Tag("minecraft:mineable/pickaxe".into()),
            HolderSet::Items(vec![2, 7]),
            HolderSet::Items(vec![]),
        ] {
            let expected = Tool {
                rules: vec![ToolRule {
                    blocks,
                    speed: Some(3.5),
                    correct: Some(false),
                }],
                default_speed: 0.25,
                damage_per_block: 0,
                can_destroy_in_creative: false,
            };
            let tag = from_network(&expected, |i| Some(format!("example:block_{i}"))).unwrap();
            let wire =
                to_network(&tag, |b| b.strip_prefix("example:block_")?.parse().ok()).unwrap();
            assert_eq!(wire, expected);
        }
        let bad = Tag::compound([(
            "rules",
            Tag::List(vec![Tag::compound([(
                "blocks",
                Tag::String("example:unknown".into()),
            )])]),
        )]);
        assert!(to_network(&bad, |_| None).is_none());
    }
}
