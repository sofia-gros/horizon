use super::{ServerHeldStack, State, server_bindings::horizon::mods::items_read};

pub const ITEM_TEXT_MAX: usize = 4096;
pub(super) const READS_PER_CALL: usize = 16;

/// The game lends immutable, owned stack snapshots. Implementations must bound
/// string conversion before allocating; the runtime checks the result as well.
pub trait ItemReader: std::any::Any + std::fmt::Debug + Send + Sync {
    fn main_hand(&self, player: &str, path: &[String]) -> Result<Option<ServerHeldStack>, String>;
}

#[derive(Clone, Debug)]
pub struct ItemEdit {
    pub owner: String,
    pub player: String,
    pub path: Vec<String>,
    pub text: String,
    pub profile: Option<super::ServerToolProfile>,
    /// Optional checked vanilla damage and max-damage values.
    pub durability: Option<(u32, u32)>,
    pub snapshot: std::sync::Arc<dyn ItemReader>,
}

impl PartialEq for ItemEdit {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner
            && self.player == other.player
            && self.path == other.path
            && self.text == other.text
            && self.profile == other.profile
            && self.durability == other.durability
            && std::sync::Arc::ptr_eq(&self.snapshot, &other.snapshot)
    }
}

impl super::server_bindings::horizon::mods::items_edit::Host for State {
    fn replace_main_hand_text(
        &mut self,
        player: String,
        custom_path: Vec<String>,
        expected: String,
        replacement: String,
    ) -> Result<(), String> {
        self.queue_item_edit(player, custom_path, expected, replacement, None, None)
    }
}

impl super::server_bindings::horizon::mods::items_tools::Host for State {
    fn enable_mining_events(&mut self) {
        self.mining_enabled = true;
    }
    fn replace_main_hand_profile(
        &mut self,
        player: String,
        custom_path: Vec<String>,
        expected: String,
        replacement: String,
        tool: super::ServerToolProfile,
    ) -> Result<(), String> {
        self.queue_item_edit(player, custom_path, expected, replacement, Some(tool), None)
    }
    fn replace_main_hand_profile_durability(
        &mut self,
        player: String,
        custom_path: Vec<String>,
        expected: String,
        replacement: String,
        tool: super::ServerToolProfile,
        damage: u32,
        max_damage: u32,
    ) -> Result<(), String> {
        if max_damage == 0 || max_damage > i32::MAX as u32 || damage > max_damage {
            return Err("invalid item durability".into());
        }
        self.queue_item_edit(
            player,
            custom_path,
            expected,
            replacement,
            Some(tool),
            Some((damage, max_damage)),
        )
    }
}

impl State {
    fn queue_item_edit(
        &mut self,
        player: String,
        custom_path: Vec<String>,
        expected: String,
        replacement: String,
        profile: Option<super::ServerToolProfile>,
        durability: Option<(u32, u32)>,
    ) -> Result<(), String> {
        if self.item_edits_left == 0 {
            return Err("items-edit budget exhausted".into());
        }
        self.item_edits_left -= 1;
        if let Some(profile) = &profile {
            validate_tool_profile(profile)?;
        }
        if custom_path.is_empty()
            || custom_path
                .first()
                .is_some_and(|key| key == "horizon:tool_control")
            || expected.len() > ITEM_TEXT_MAX
            || replacement.len() > ITEM_TEXT_MAX
        {
            return Err("invalid items-edit path or text size".into());
        }
        if self.item_edits.contains(&player) {
            return Err("one item edit per player per hook".into());
        }
        let view = read(
            self.snapshot.items.as_deref(),
            &mut self.item_reads_left,
            &player,
            &custom_path,
        )?
        .ok_or("empty main hand")?;
        if view.custom_text.as_deref() != Some(expected.as_str()) {
            return Err("main-hand text does not match expected value".into());
        }
        let snapshot = self.snapshot.items.clone().ok_or("no server item view")?;
        self.item_edits.insert(player.clone());
        self.out.push(super::Output::ItemEdit(ItemEdit {
            owner: self.id.clone(),
            player,
            path: custom_path,
            text: replacement,
            profile,
            durability,
            snapshot,
        }));
        Ok(())
    }
}

/// Bounds checked again by native application adapters; no material-specific rules.
pub fn validate_tool_profile(profile: &super::ServerToolProfile) -> Result<(), String> {
    let speed = |v: f32| v.is_finite() && v > 0.0;
    if !speed(profile.default_speed) || profile.rules.len() > 16 || profile.damage_per_block > 1024
    {
        return Err("invalid tool profile limits".into());
    }
    for rule in &profile.rules {
        if rule.blocks.len() > 64
            || rule.speed.is_some_and(|v| !speed(v))
            || (rule.blocks.len() != 1 && rule.blocks.iter().any(|b| b.starts_with('#')))
        {
            return Err("invalid tool rule limits".into());
        }
        for block in &rule.blocks {
            let id = block.strip_prefix('#').unwrap_or(block);
            let valid = id.split_once(':').is_some_and(|(namespace, path)| {
                !namespace.is_empty()
                    && !path.is_empty()
                    && namespace.bytes().all(|b| {
                        b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b)
                    })
                    && path.bytes().all(|b| {
                        b.is_ascii_lowercase() || b.is_ascii_digit() || b"_/.-".contains(&b)
                    })
            });
            if block.len() > 128 || !valid {
                return Err("invalid tool block selector".into());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod profile_tests {
    use super::*;
    use crate::{ServerToolProfile, ServerToolRule};

    #[test]
    fn tool_profiles_bound_rules_selectors_and_numbers() {
        let profile = ServerToolProfile {
            rules: vec![ServerToolRule {
                blocks: vec!["#minecraft:mineable/pickaxe".into()],
                speed: Some(3.5),
                correct_for_drops: Some(true),
            }],
            default_speed: 1.0,
            damage_per_block: 0,
            can_destroy_in_creative: true,
            mining_enabled: true,
            unbreakable: false,
        };
        assert!(validate_tool_profile(&profile).is_ok());
        for speed in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            let mut p = profile.clone();
            p.default_speed = speed;
            assert!(validate_tool_profile(&p).is_err());
            p.default_speed = 1.0;
            p.rules[0].speed = Some(speed);
            assert!(validate_tool_profile(&p).is_err());
        }
        for blocks in [
            vec!["minecraft:stone".into(); 65],
            vec![
                "#minecraft:mineable/pickaxe".into(),
                "minecraft:stone".into(),
            ],
            vec!["invalid".into()],
            vec!["minecraft:UPPERCASE".into()],
        ] {
            let mut p = profile.clone();
            p.rules[0].blocks = blocks;
            assert!(validate_tool_profile(&p).is_err());
        }
        let mut p = profile.clone();
        p.rules = vec![p.rules[0].clone(); 17];
        assert!(validate_tool_profile(&p).is_err());
        p = profile;
        p.damage_per_block = 1025;
        assert!(validate_tool_profile(&p).is_err());
    }
}

fn read(
    reader: Option<&dyn ItemReader>,
    remaining: &mut usize,
    player: &str,
    path: &[String],
) -> Result<Option<ServerHeldStack>, String> {
    if *remaining == 0 {
        return Err("items-read budget exhausted".into());
    }
    *remaining -= 1;
    if player.len() > 64
        || path.len() > 8
        || path.iter().any(|key| key.is_empty() || key.len() > 128)
    {
        return Err("invalid items-read player or custom path".into());
    }
    let item = reader.map_or(Ok(None), |reader| reader.main_hand(player, path))?;
    if item.as_ref().is_some_and(|item| {
        item.item.len() > 256
            || item
                .custom_text
                .as_ref()
                .is_some_and(|text| text.len() > ITEM_TEXT_MAX)
    }) {
        return Err("items-read result exceeds its size limit".into());
    }
    Ok(item)
}

impl items_read::Host for State {
    fn main_hand(
        &mut self,
        player: String,
        custom_path: Vec<String>,
    ) -> Result<Option<ServerHeldStack>, String> {
        read(
            self.snapshot.items.as_deref(),
            &mut self.item_reads_left,
            &player,
            &custom_path,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Oversized;
    impl ItemReader for Oversized {
        fn main_hand(&self, _: &str, _: &[String]) -> Result<Option<ServerHeldStack>, String> {
            Ok(Some(ServerHeldStack {
                item: "example:tool".into(),
                count: 1,
                damage: 0,
                custom_text: Some("x".repeat(ITEM_TEXT_MAX + 1)),
            }))
        }
    }

    #[test]
    fn reads_are_bounded_and_never_truncate_a_state() {
        let mut budget = READS_PER_CALL;
        assert!(
            read(Some(&Oversized), &mut budget, "player", &[])
                .unwrap_err()
                .contains("size limit")
        );
        assert!(read(None, &mut budget, "player", &[String::new()]).is_err());
        assert!(read(None, &mut budget, "player", &vec!["x".into(); 9]).is_err());
        assert!(read(None, &mut budget, "player", &["x".repeat(129)]).is_err());
        while budget > 0 {
            assert!(read(None, &mut budget, "offline", &[]).unwrap().is_none());
        }
        assert!(
            read(None, &mut budget, "offline", &[])
                .unwrap_err()
                .contains("budget")
        );
    }
}
