//! Vanilla tool and generic mod profile settings on individual item stacks.
use crate::items::ItemStack;
use horizon_world::nbt::Tag;

/// Per-stack vanilla max-damage patch, otherwise the item's prototype.
pub fn max_damage(stack: &ItemStack, default: u32) -> u32 {
    match stack.data().and_then(|d| d.other("minecraft:max_damage")) {
        Some(Tag::Int(value)) if *value > 0 => *value as u32,
        _ => default,
    }
}

/// Generic checked melee settings attached to a stack by its owning mod.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeleeProfile {
    pub damage_bonus: f32,
    pub damage_multiplier: f32,
    pub attack_speed: f32,
    pub enabled: bool,
}
pub fn melee_profile(stack: &ItemStack) -> Option<MeleeProfile> {
    let c = stack
        .data()?
        .other("minecraft:custom_data")?
        .get("horizon:tool_control")?;
    if !matches!(c.get("schema"), Some(Tag::Int(1))) {
        return None;
    }
    let m = c.get("melee")?;
    let profile = MeleeProfile {
        damage_bonus: number(m.get("damage_bonus")?)?,
        damage_multiplier: number(m.get("damage_multiplier")?)?,
        attack_speed: number(m.get("attack_speed")?)?,
        enabled: boolean(m.get("enabled")?)?,
    };
    (profile.damage_bonus.is_finite()
        && (0.0..=2048.0).contains(&profile.damage_bonus)
        && profile.damage_multiplier.is_finite()
        && (0.0..=64.0).contains(&profile.damage_multiplier)
        && profile.attack_speed.is_finite()
        && (0.01..=1024.0).contains(&profile.attack_speed))
    .then_some(profile)
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
