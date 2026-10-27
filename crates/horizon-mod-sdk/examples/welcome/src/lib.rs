//! Example Horizon server mod (`docs/mods.md`, "Server mods"): greets each
//! player who joins, announces a newcomer to everyone (the players it has
//! seen are kept in `storage`), `/online` lists who is on and where, and
//! `/mined [player]` counts the blocks a player broke since the server
//! started (typed arguments: the chat box completes the player's name).
use horizon_mod_sdk::{
    server::{
        Args, Argument, BlockPos, Player, Registration, ServerMod, items_edit, items_read,
        items_tools, players,
    },
    storage,
};
use std::collections::HashMap;

struct Welcome {
    /// Ids of every player seen, one per line in storage key `seen`.
    seen: Vec<String>,
    mined: HashMap<String, u32>,
}

impl ServerMod for Welcome {
    fn init() -> Result<(Self, Registration), String> {
        let seen = storage::get("seen")
            .map(|b| {
                String::from_utf8_lossy(&b)
                    .lines()
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        let hooks = Registration::new()
            .command_with("online", [])
            .command_with("mined", [Argument::player("player").optional()])
            .command("hand")
            .command("edit")
            .on_joins()
            .on_blocks();
        Ok((
            Welcome {
                seen,
                mined: HashMap::new(),
            },
            hooks,
        ))
    }

    fn player_joined(&mut self, player: &Player) {
        let others = players::online().len() - 1;
        players::tell(
            &player.id,
            &format!("Welcome, {}! {others} other player(s) online.", player.name),
        );
        if !self.seen.contains(&player.id) {
            self.seen.push(player.id.clone());
            let _ = storage::set("seen", self.seen.join("\n").as_bytes());
            players::broadcast(&format!("{} is here for the first time", player.name));
        }
    }

    fn command(
        &mut self,
        player: &Player,
        name: &str,
        args: &Args,
    ) -> Result<Option<String>, String> {
        Ok(Some(match name {
            "edit" => {
                let args = args.text();
                let path = if args == "profile-reserved" {
                    ["horizon:tool_control".into(), "owner".into()]
                } else {
                    ["example".into(), "state".into()]
                };
                let view = items_read::main_hand(&player.id, &path)?.ok_or("empty main hand")?;
                let expected = view.custom_text.ok_or("missing state")?;
                let replacement = if args == "big" {
                    "x".repeat(4097)
                } else {
                    "changed".into()
                };
                let expected = if args.ends_with("stale") {
                    "wrong"
                } else {
                    &expected
                };
                if args.starts_with("profile") {
                    let profile = items_tools::Profile {
                        rules: vec![items_tools::Rule {
                            blocks: vec![
                                if args == "profile-unknown" {
                                    "example:missing"
                                } else {
                                    "minecraft:stone"
                                }
                                .into(),
                            ],
                            speed: Some(90.0),
                            correct_for_drops: Some(true),
                        }],
                        default_speed: if args == "profile-invalid" { 0.0 } else { 1.0 },
                        damage_per_block: if args == "profile-zero" { 0 } else { 1 },
                        can_destroy_in_creative: true,
                        mining_enabled: matches!(
                            args,
                            "profile-enabled" | "profile-zero" | "profile-wear"
                        ),
                        unbreakable: !matches!(args, "profile-zero" | "profile-wear"),
                    };
                    items_tools::replace_main_hand_profile(
                        &player.id,
                        &path,
                        expected,
                        &replacement,
                        &profile,
                    )?;
                    if args == "profile-trap" {
                        panic!("profile trap");
                    }
                    if args == "profile-error" {
                        return Err("profile error".into());
                    }
                    if args == "profile-twice" {
                        items_edit::replace_main_hand_text(&player.id, &path, expected, "again")?;
                    }
                    return Ok(Some("profile queued".into()));
                }
                items_edit::replace_main_hand_text(&player.id, &path, expected, &replacement)?;
                if args == "trap" {
                    panic!("edit trap");
                }
                if args == "error" {
                    return Err("edit error".into());
                }
                if args == "twice" {
                    items_edit::replace_main_hand_text(&player.id, &path, expected, "again")?;
                }
                "queued".into()
            }
            "hand" => {
                let args = args.text();
                let path: Vec<String> = args.split_whitespace().map(str::to_owned).collect();
                let reads = if args == "budget" { 17 } else { 1 };
                let mut item = None;
                for _ in 0..reads {
                    item = items_read::main_hand(&player.id, &path)?;
                }
                match item {
                    Some(item) => format!(
                        "{} x{} damage={} text={}",
                        item.item,
                        item.count,
                        item.damage,
                        item.custom_text.as_deref().unwrap_or("none")
                    ),
                    None => "empty hand".into(),
                }
            }
            "online" => players::online()
                .iter()
                .map(|p| {
                    format!(
                        "{} ({})",
                        p.name,
                        p.dimension.trim_start_matches("minecraft:")
                    )
                })
                .collect::<Vec<_>>()
                .join(", "),
            _ => match args.player("player") {
                Some(other) if other.id != player.id => {
                    let n = self.mined.get(&other.id).copied().unwrap_or(0);
                    format!("{} broke {n} block(s)", other.name)
                }
                _ => {
                    let n = self.mined.get(&player.id).copied().unwrap_or(0);
                    format!("You broke {n} block(s)")
                }
            },
        }))
    }

    fn block_broken(&mut self, player: &Player, _pos: BlockPos, _state: &str) {
        *self.mined.entry(player.id.clone()).or_default() += 1;
    }
}

horizon_mod_sdk::export_server_mod!(Welcome);
