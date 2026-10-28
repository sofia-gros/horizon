//! Server mods in the game (`docs/mods.md`, "Server mods"): the game that
//! simulates the world (in-frame single player, the server thread, a LAN
//! host, the dedicated server) runs them. At the end of each tick it tells
//! them who joined and left, then what players said, broke and placed since
//! the last one, then ticks them; a player's command reaches them at once.
//! Their lines go to the players they name. With no server mod on, each hook
//! here is one check.
use super::{Game, lan::LanHost};
pub(super) fn workstation_player_id(u: u128) -> String {
    let h = format!("{u:032x}");
    format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    )
}
#[cfg(not(feature = "mods"))]
use horizon_world::registry::BlockStateId;

/// Who did something a server mod hears about, resolved at the tick's end.
#[cfg_attr(not(feature = "mods"), allow(dead_code))]
pub(super) enum Who {
    /// The window's own player, in-frame.
    Local,
    /// A connection's player (the server thread's owner is one).
    Conn(u32),
    Uuid(u128),
}

/// What players did during a tick, for the server mods at its end.
#[cfg_attr(not(feature = "mods"), allow(dead_code))]
pub(super) enum ModEvent {
    Chat(Who, String),
    Block {
        who: Who,
        dimension: String,
        pos: [i32; 3],
        state: String,
        placed: bool,
    },
}

#[cfg(not(feature = "mods"))]
impl Game<'_> {
    pub(super) fn mod_workstation(&self, _block: &str) -> Option<super::stations::ModRegistration> {
        None
    }
    pub(super) fn mod_workstation_plan(
        &mut self,
        _registration: &super::stations::ModRegistration,
        _player: &str,
        _tool: &horizon_client::items::ItemStack,
        _ingredient: &horizon_client::items::ItemStack,
    ) -> Option<(horizon_client::items::ItemStack, u32)> {
        None
    }
    pub(super) fn tick_server_mods(&mut self) {}
    pub(super) fn mod_block_event(&mut self, _pos: [i32; 3], _state: BlockStateId, _placed: bool) {}
    pub(super) fn mod_mined(
        &mut self,
        _pos: [i32; 3],
        _state: BlockStateId,
        _hardness: f32,
        _slot: usize,
        _held: Option<&horizon_client::items::ItemStack>,
    ) {
    }
    pub(super) fn mod_melee_hit(
        &mut self,
        _target_id: u64,
        _target_kind: &str,
        _dealt: f32,
        _slot: usize,
        _held: Option<&horizon_client::items::ItemStack>,
    ) {
    }
    pub(super) fn mod_player_chat(&mut self, _who: Who, _text: &str) {}
    pub(super) fn local_server_mod_command(&mut self, _line: &str) -> bool {
        false
    }
    pub(super) fn guest_server_mod_command(
        &mut self,
        _host: &mut LanHost,
        _conn: u32,
        _line: &str,
    ) -> bool {
        false
    }
    pub(super) fn apply_mod_block_writes(&mut self) {}
}

#[cfg(feature = "mods")]
mod run {
    use super::{Game, LanHost, ModEvent, Who};
    use horizon_client::chunk_map::ChunkMap;
    use horizon_client::{
        command::args::GameMode as Mode,
        lan::data::component_nbt,
        text::{self, Text},
    };
    use horizon_mods::{
        ArgumentKind, BlockPos, GameMode, Output, ServerCommand, ServerPlayer, WorldView,
    };
    use horizon_protocol::packets::play::{self, Clientbound as C};
    use horizon_world::registry::{BlockRegistry, BlockStateId};
    use serde_json::{Value, json};

    /// Blocks queued for levels that don't tick (another dimension, in-frame)
    /// are kept up to this many; the oldest go first.
    const MAX_QUEUED_WRITES: usize = 1 << 16;

    /// Capability `world`'s view during a call: the levels' chunks, moved out
    /// of the game for the call and back after it (no copy).
    struct LentWorld {
        registry: &'static BlockRegistry,
        biomes: std::sync::Arc<horizon_client::biome::Biomes>,
        dims: Vec<(&'static str, ChunkMap)>,
    }
    impl WorldView for LentWorld {
        fn block(&self, dimension: &str, x: i32, y: i32, z: i32) -> Option<String> {
            let (_, chunks) = self.dims.iter().find(|(d, _)| *d == dimension)?;
            let chunk = chunks.get(&(x >> 4, z >> 4))?;
            Some(state_string(self.registry, chunk.block(x & 15, y, z & 15)))
        }
        fn biome(&self, dimension: &str, x: i32, y: i32, z: i32) -> Option<String> {
            let (_, chunks) = self.dims.iter().find(|(d, _)| *d == dimension)?;
            let chunk = chunks.get(&(x >> 4, z >> 4))?;
            Some(self.biomes.get(chunk.biome(x, y, z)).name.clone())
        }
        fn has_dimension(&self, dimension: &str) -> bool {
            matches!(
                dimension,
                "minecraft:overworld" | "minecraft:the_nether" | "minecraft:the_end"
            )
        }
        fn check_state(&self, state: &str) -> Result<(), String> {
            crate::game::render_control::parse_state(self.registry, state)
                .map(drop)
                .map_err(|e| format!("{state}: {e}"))
        }
    }

    /// Server mods' commands as a root of the commands report
    /// (`lan::commands`): a literal per command, then its arguments in a
    /// chain, each node runnable where only optional arguments follow.
    fn command_tree<'a>(commands: impl Iterator<Item = &'a ServerCommand>) -> Value {
        let mut root = serde_json::Map::new();
        for c in commands {
            let mut next: Option<(String, Value)> = None;
            for (i, a) in c.arguments.iter().enumerate().rev() {
                let executable = c.arguments.get(i + 1).is_none_or(|b| b.optional);
                let (parser, properties) = match a.kind {
                    ArgumentKind::Word => ("brigadier:string", json!({"type": "word"})),
                    ArgumentKind::Text => ("brigadier:string", json!({"type": "greedy"})),
                    ArgumentKind::Integer((min, max)) => {
                        let mut p = serde_json::Map::new();
                        if let Some(min) = min {
                            p.insert("min".into(), min.into());
                        }
                        if let Some(max) = max {
                            p.insert("max".into(), max.into());
                        }
                        ("brigadier:integer", Value::Object(p))
                    }
                    ArgumentKind::Number => ("brigadier:double", Value::Null),
                    ArgumentKind::Boolean => ("brigadier:bool", Value::Null),
                    ArgumentKind::Player => (
                        "minecraft:entity",
                        json!({"amount": "single", "type": "players"}),
                    ),
                    ArgumentKind::BlockPos => ("minecraft:block_pos", Value::Null),
                };
                let mut node = json!({
                    "type": "argument",
                    "parser": parser,
                    "properties": properties,
                    "executable": executable,
                });
                if let Some((name, child)) = next.take() {
                    node["children"] = json!({ name: child });
                }
                next = Some((a.name.clone(), node));
            }
            let executable = c.arguments.first().is_none_or(|a| a.optional);
            let mut literal = json!({"type": "literal", "executable": executable});
            if let Some((name, child)) = next {
                literal["children"] = json!({ name: child });
            }
            root.insert(c.name.clone(), literal);
        }
        if root.is_empty() {
            return Value::Null;
        }
        json!({"type": "root", "children": root})
    }

    /// `minecraft:oak_log[axis=y]`.
    fn state_string(registry: &BlockRegistry, state: BlockStateId) -> String {
        let Some(s) = registry.state(state) else {
            return "minecraft:air".into();
        };
        if s.properties.is_empty() {
            return s.name.clone();
        }
        let props: Vec<String> = s
            .properties
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        format!("{}[{}]", s.name, props.join(","))
    }

    /// `UUID.toString()`.
    fn uuid_string(u: u128) -> String {
        let h = format!("{u:032x}");
        format!(
            "{}-{}-{}-{}-{}",
            &h[..8],
            &h[8..12],
            &h[12..16],
            &h[16..20],
            &h[20..]
        )
    }

    fn mode(id: i32) -> GameMode {
        match id {
            1 => GameMode::Creative,
            2 => GameMode::Adventure,
            3 => GameMode::Spectator,
            _ => GameMode::Survival,
        }
    }

    impl Game<'_> {
        pub(in crate::game) fn mod_workstation(
            &self,
            block: &str,
        ) -> Option<super::super::stations::ModRegistration> {
            let owner = self.mods.workstation_handlers.get(block)?;
            let r = self.mods.mods.workstation(block, owner)?;
            Some(super::super::stations::ModRegistration {
                owner: r.owner,
                block: r.block,
                title: r.title,
                path: r.path,
                ingredient_tags: r.ingredient_tags,
            })
        }

        /// Rebuild a checked result from the complete input snapshots. The mod
        /// can change only its selected text and the existing generic profile.
        pub(in crate::game) fn mod_workstation_plan(
            &mut self,
            registration: &super::super::stations::ModRegistration,
            player: &str,
            tool: &horizon_client::items::ItemStack,
            ingredient: &horizon_client::items::ItemStack,
        ) -> Option<(horizon_client::items::ItemStack, u32)> {
            use horizon_mods::ItemReader;
            if self.mods.workstation_handlers.get(&registration.block) != Some(&registration.owner)
            {
                return None;
            }
            let reg = self
                .mods
                .mods
                .workstation(&registration.block, &registration.owner)?;
            if reg.owner != registration.owner
                || reg.path != registration.path
                || reg.ingredient_tags != registration.ingredient_tags
                || tool.count != 1
            {
                return None;
            }
            let view = std::sync::Arc::new(horizon_client::mod_items::ServerItems(vec![(
                player.into(),
                0,
                tool.clone(),
            )]));
            let tool_view = view.main_hand(player, &reg.path).ok()??;
            tool_view.custom_text.as_ref()?;
            let ing_view = horizon_client::mod_items::ServerItems(vec![(
                player.into(),
                0,
                ingredient.clone(),
            )])
            .main_hand(player, &reg.path)
            .ok()??;
            let tags = reg
                .ingredient_tags
                .iter()
                .filter(|tag| {
                    self.res
                        .items()
                        .item_tags
                        .get(tag.as_str())
                        .is_some_and(|items| items.contains(&*ingredient.item))
                })
                .cloned()
                .collect();
            let input = |v: horizon_mods::ServerHeldStack, tags| horizon_mods::WorkstationInput {
                item: v.item,
                count: v.count,
                damage: v.damage,
                custom_text: v.custom_text,
                tags,
            };
            let plan = self
                .mods
                .mods
                .workstation_preview(&reg, &input(tool_view, vec![]), &input(ing_view, tags))
                .ok()??;
            if plan.consume == 0
                || plan.consume > ingredient.count
                || plan.replacement.len() > horizon_mods::ITEM_TEXT_MAX
            {
                return None;
            }
            let profile = horizon_mods::ServerToolProfile {
                rules: plan
                    .tool
                    .rules
                    .into_iter()
                    .map(|r| horizon_mods::ServerToolRule {
                        blocks: r.blocks,
                        speed: r.speed,
                        correct_for_drops: r.correct_for_drops,
                    })
                    .collect(),
                default_speed: plan.tool.default_speed,
                damage_per_block: plan.tool.damage_per_block,
                can_destroy_in_creative: plan.tool.can_destroy_in_creative,
                mining_enabled: plan.tool.mining_enabled,
                unbreakable: plan.tool.unbreakable,
            };
            horizon_mods::validate_tool_profile(&profile).ok()?;
            if profile
                .rules
                .iter()
                .flat_map(|r| &r.blocks)
                .any(|selector| {
                    if let Some(tag) = selector.strip_prefix('#') {
                        !self.res.items().has_block_tag(tag)
                    } else {
                        self.pack.assets.registry.default_state(selector).is_none()
                    }
                })
            {
                return None;
            }
            let melee = horizon_mods::ServerMeleeProfile {
                damage_bonus: plan.melee.damage_bonus,
                damage_multiplier: plan.melee.damage_multiplier,
                attack_speed: plan.melee.attack_speed,
                enabled: plan.melee.enabled,
            };
            let edit = horizon_mods::ItemEdit {
                owner: reg.owner,
                player: player.into(),
                path: reg.path,
                text: plan.replacement,
                profile: Some(profile),
                durability: Some((plan.damage, plan.max_damage)),
                melee: Some(melee),
                consumption: None,
                snapshot: view.clone(),
            };
            let mut result = Some(tool.clone());
            view.apply(0, &mut result, &edit).ok()?;
            Some((result?, plan.consume))
        }

        /// Called while the acting player's inventory is swapped into Game.
        /// Apply only the profile owner's edit to that same slot before the
        /// guest state is swapped back or another block is broken.
        pub(in crate::game) fn mod_mined(
            &mut self,
            pos: [i32; 3],
            state: BlockStateId,
            hardness: f32,
            slot: usize,
            held: Option<&horizon_client::items::ItemStack>,
        ) {
            use horizon_mods::ItemReader;
            if !self.hears_players() {
                return;
            }
            if self.selected != slot || self.held() != held {
                return;
            }
            let Some(stack) = self.held().cloned() else {
                return;
            };
            let player = self
                .lan_acting_uuid
                .map(uuid_string)
                .unwrap_or_else(|| self.player_uuid_string());
            let view = std::sync::Arc::new(horizon_client::mod_items::ServerItems(vec![(
                player.clone(),
                self.selected,
                stack,
            )]));
            let path = vec!["horizon:tool_control".into(), "owner".into()];
            let Some(owner) = view
                .main_hand(&player, &path)
                .ok()
                .flatten()
                .and_then(|v| v.custom_text)
            else {
                return;
            };
            let mut snapshot = self.mod_snapshot_with(Vec::new());
            snapshot.items = Some(view.clone());
            self.mods.mods.set_snapshot(snapshot);
            let state = state_string(self.pack.assets.registry, state);
            let edits =
                self.mods
                    .mods
                    .server_mined(&owner, &player, self.dim.id(), pos, &state, hardness);
            for edit in edits {
                if let Some(profile) = &edit.profile
                    && (horizon_mods::validate_tool_profile(profile).is_err()
                        || profile
                            .rules
                            .iter()
                            .flat_map(|r| &r.blocks)
                            .any(|selector| {
                                if let Some(tag) = selector.strip_prefix('#') {
                                    !self.res.items().has_block_tag(tag)
                                } else {
                                    self.pack.assets.registry.default_state(selector).is_none()
                                }
                            }))
                {
                    continue;
                }
                if let Some(cell) = self.inventory.get_mut(self.selected)
                    && let Err(error) = view.apply(self.selected, cell, &edit)
                {
                    log::warn!("Horizon: mining item edit rejected: {error}");
                }
            }
        }
        /// Run against the attacker stack captured before a landed hit.
        pub(in crate::game) fn mod_melee_hit(
            &mut self,
            target_id: u64,
            target_kind: &str,
            dealt: f32,
            slot: usize,
            held: Option<&horizon_client::items::ItemStack>,
        ) {
            use horizon_mods::ItemReader;
            if !self.hears_players() {
                return;
            }
            if self.selected != slot || self.held() != held {
                return;
            }
            let Some(stack) = self.held().cloned() else {
                return;
            };
            let player = self
                .lan_acting_uuid
                .map(uuid_string)
                .unwrap_or_else(|| self.player_uuid_string());
            let view = std::sync::Arc::new(horizon_client::mod_items::ServerItems(vec![(
                player.clone(),
                self.selected,
                stack,
            )]));
            let path = vec!["horizon:tool_control".into(), "owner".into()];
            let Some(owner) = view
                .main_hand(&player, &path)
                .ok()
                .flatten()
                .and_then(|v| v.custom_text)
            else {
                return;
            };
            let mut snapshot = self.mod_snapshot_with(Vec::new());
            snapshot.items = Some(view.clone());
            self.mods.mods.set_snapshot(snapshot);
            let edits = self.mods.mods.server_melee_hit(
                &owner,
                &player,
                self.dim.id(),
                target_id,
                target_kind,
                dealt,
            );
            for edit in edits {
                if let Some(profile) = &edit.profile
                    && (horizon_mods::validate_tool_profile(profile).is_err()
                        || profile
                            .rules
                            .iter()
                            .flat_map(|r| &r.blocks)
                            .any(|selector| {
                                if let Some(tag) = selector.strip_prefix('#') {
                                    !self.res.items().has_block_tag(tag)
                                } else {
                                    self.pack.assets.registry.default_state(selector).is_none()
                                }
                            }))
                {
                    continue;
                }
                if let Some(cell) = self.inventory.get_mut(self.selected)
                    && let Err(error) = view.apply(self.selected, cell, &edit)
                {
                    log::warn!("Horizon: melee item edit rejected: {error}");
                }
            }
        }
        fn apply_item_edit(
            &mut self,
            host: Option<&mut LanHost>,
            edit: &horizon_mods::ItemEdit,
        ) -> Result<(), String> {
            let any: &dyn std::any::Any = edit.snapshot.as_ref();
            let snapshot = any
                .downcast_ref::<horizon_client::mod_items::ServerInventories>()
                .ok_or("invalid inventory view")?;
            if self.remote.is_some() {
                return Err("not the authoritative server".into());
            }
            if let Some(profile) = &edit.profile {
                horizon_mods::validate_tool_profile(profile)?;
                for selector in profile.rules.iter().flat_map(|r| &r.blocks) {
                    let known = if let Some(tag) = selector.strip_prefix('#') {
                        self.res.items().has_block_tag(tag)
                    } else {
                        self.pack.assets.registry.default_state(selector).is_some()
                    };
                    if !known {
                        return Err(format!("unknown tool block selector: {selector}"));
                    }
                }
            }
            if self.options.execution_role.is_client()
                && self.player.is_some()
                && self.player_uuid_string() == edit.player
            {
                if self.screen != super::super::Screen::Playing || self.carried.is_some() {
                    return Err("close the container before editing".into());
                }
                return snapshot.apply(self.selected, &mut self.inventory, edit);
            }
            let host = host.or(self.lan.as_mut()).ok_or("player is offline")?;
            let index = host
                .hub
                .guests
                .iter()
                .position(|guest| guest.placed && uuid_string(guest.uuid) == edit.player)
                .ok_or("player is offline")?;
            let guest = &mut host.hub.guests[index];
            if guest.menu.is_some() || guest.carried.is_some() {
                return Err("close the container before editing".into());
            }
            snapshot.apply(guest.selected, &mut guest.inventory, edit)?;
            host.hub.send_inventory(index);
            Ok(())
        }

        fn server_mod_snapshot(
            &mut self,
            players: Vec<ServerPlayer>,
            host: Option<&LanHost>,
            command_actor: Option<&str>,
        ) -> horizon_mods::Snapshot {
            let wants = if command_actor.is_some() {
                self.mods.mods.wants_item_reads()
            } else {
                self.mods.mods.wants_item_event_reads()
            };
            let items = wants.then(|| {
                let mut held = Vec::new();
                let mut slots = Vec::new();
                if self.options.execution_role.is_client()
                    && self.remote.is_none()
                    && self.player.is_some()
                    && let Some(stack) = self.held()
                {
                    held.push((self.player_uuid_string(), self.selected, stack.clone()));
                }
                if command_actor == Some(self.player_uuid_string().as_str())
                    && self.options.execution_role.is_client()
                    && self.remote.is_none()
                    && self.player.is_some()
                {
                    slots.push((self.player_uuid_string(), self.inventory.clone()));
                }
                for guest in host
                    .or(self.lan.as_ref())
                    .into_iter()
                    .flat_map(|host| &host.hub.guests)
                    .filter(|guest| guest.placed)
                {
                    if let Some(Some(stack)) = guest.inventory.get(guest.selected) {
                        held.push((uuid_string(guest.uuid), guest.selected, stack.clone()));
                    }
                    if command_actor == Some(uuid_string(guest.uuid).as_str()) {
                        slots.push((uuid_string(guest.uuid), guest.inventory.clone()));
                    }
                }
                std::sync::Arc::new(horizon_client::mod_items::ServerInventories {
                    held: horizon_client::mod_items::ServerItems(held),
                    slots,
                }) as std::sync::Arc<dyn horizon_mods::ItemReader>
            });
            let mut snapshot = self.mod_snapshot_with(players);
            snapshot.items = items;
            snapshot
        }

        /// The players on this game's server: the window's own player
        /// in-frame, then every connection's. `host` is the hub when it is
        /// out of `self.lan`.
        pub(in crate::game) fn server_players(&self, host: Option<&LanHost>) -> Vec<ServerPlayer> {
            let mut out = Vec::new();
            if self.options.execution_role.is_client()
                && self.remote.is_none()
                && let Some(p) = &self.player
            {
                out.push(ServerPlayer {
                    id: self.player_uuid_string(),
                    name: crate::game::chat::PLAYER_NAME.to_owned(),
                    x: p.position.x,
                    y: p.position.y,
                    z: p.position.z,
                    dimension: self.dim.id().to_owned(),
                    game_mode: match self.mode {
                        Mode::Survival => GameMode::Survival,
                        Mode::Creative => GameMode::Creative,
                        Mode::Adventure => GameMode::Adventure,
                        Mode::Spectator => GameMode::Spectator,
                    },
                    health: p.health.health(),
                });
            }
            let guests = host
                .or(self.lan.as_ref())
                .into_iter()
                .flat_map(|h| &h.hub.guests);
            out.extend(guests.filter(|g| g.placed).map(|g| ServerPlayer {
                id: uuid_string(g.uuid),
                name: g.name.clone(),
                x: g.position[0],
                y: g.position[1],
                z: g.position[2],
                dimension: g.dimension.clone(),
                game_mode: mode(g.game_mode),
                health: g.health,
            }));
            out
        }

        /// Whether what players do now reaches server mods: one is running
        /// and this game simulates the world.
        fn hears_players(&self) -> bool {
            self.mods.server && self.remote.is_none()
        }

        /// The tick's end: joins and leaves, the tick's events, the tick hook.
        pub(in crate::game) fn tick_server_mods(&mut self) {
            if !self.options.execution_role.is_client() {
                // A server has no frames to check its dev mods in.
                self.reload_dev_mods();
            }
            self.send_server_mod_commands();
            if !self.mods.server {
                return;
            }
            let players = self.server_players(None);
            let snapshot = self.server_mod_snapshot(players.clone(), None, None);
            self.mods.mods.set_snapshot(snapshot);
            self.lend_world();
            let before = std::mem::take(&mut self.mods.players);
            for p in before
                .iter()
                .filter(|p| !players.iter().any(|q| q.id == p.id))
            {
                self.mods.mods.player_left(p);
            }
            for p in players
                .iter()
                .filter(|p| !before.iter().any(|q| q.id == p.id))
            {
                self.mods.mods.player_joined(p);
            }
            for event in std::mem::take(&mut self.mods.events) {
                let who = |w: &Who| {
                    let id = match w {
                        Who::Local => self.player_uuid_string(),
                        Who::Uuid(u) => uuid_string(*u),
                        Who::Conn(c) => {
                            let guests = self.lan.iter().flat_map(|h| &h.hub.guests);
                            let g = guests.into_iter().find(|g| g.conn == *c);
                            g.map_or_else(String::new, |g| uuid_string(g.uuid))
                        }
                    };
                    players.iter().chain(&before).find(|p| p.id == id).cloned()
                };
                match event {
                    ModEvent::Chat(w, text) => {
                        if let Some(p) = who(&w) {
                            self.mods.mods.player_chat(&p, &text);
                        }
                    }
                    ModEvent::Block {
                        who: w,
                        dimension,
                        pos,
                        state,
                        placed,
                    } => {
                        let Some(mut p) = who(&w) else { continue };
                        p.dimension = dimension;
                        let pos = BlockPos {
                            x: pos[0],
                            y: pos[1],
                            z: pos[2],
                        };
                        if placed {
                            self.mods.mods.block_placed(&p, pos, &state);
                        } else {
                            self.mods.mods.block_broken(&p, pos, &state);
                        }
                    }
                }
            }
            self.mods.mods.server_tick(self.ticks);
            self.return_world();
            self.mods.players = players;
            self.show_mod_output();
            self.apply_mod_block_writes();
        }

        /// Players' command trees list the running server mods' commands
        /// after vanilla's, whenever they change (or a hub opens).
        fn send_server_mod_commands(&mut self) {
            let key = self.mods.mods.generation();
            let Some(host) = self.lan.as_mut() else {
                return;
            };
            if host.hub.mod_commands_key() == Some(key) {
                return;
            }
            let tree = command_tree(self.mods.mods.server_commands());
            host.hub.set_mod_commands(key, tree);
        }

        /// Lends the levels' chunks to capability `world` for the calls up to
        /// [`Game::return_world`], when a running server mod has it.
        fn lend_world(&mut self) {
            if !self.mods.world {
                return;
            }
            let mut dims = vec![(self.dim.id(), std::mem::take(&mut self.world.chunks))];
            for (dim, chunks) in self.levels.parked_chunks() {
                dims.push((dim.id(), std::mem::take(chunks)));
            }
            let registry = self.pack.assets.registry;
            let biomes = self.res.biomes().clone();
            self.mods.mods.lend_world(Box::new(LentWorld {
                registry,
                biomes,
                dims,
            }));
        }

        /// Puts back the chunks [`Game::lend_world`] lent.
        fn return_world(&mut self) {
            let Some(view) = self.mods.mods.take_world() else {
                return;
            };
            let view: Box<dyn std::any::Any> = view;
            let Ok(lent) = view.downcast::<LentWorld>() else {
                return;
            };
            let mut dims = lent.dims;
            if let Some(i) = dims.iter().position(|(d, _)| *d == self.dim.id()) {
                self.world.chunks = dims.swap_remove(i).1;
            }
            for (dim, chunks) in self.levels.parked_chunks() {
                if let Some(i) = dims.iter().position(|(d, _)| *d == dim.id()) {
                    *chunks = dims.swap_remove(i).1;
                }
            }
        }

        /// Makes the blocks server mods set in the level in place, in order,
        /// as `/setblock` does. Those of other dimensions wait for their level.
        pub(in crate::game) fn apply_mod_block_writes(&mut self) {
            if self.mods.block_writes.is_empty() {
                return;
            }
            let dim = self.dim.id();
            let (now, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.mods.block_writes)
                .into_iter()
                .partition(|(d, _)| d == dim);
            self.mods.block_writes = later;
            let registry = self.pack.assets.registry;
            for (_, b) in now {
                let state = match crate::game::render_control::parse_state(registry, &b.state) {
                    Ok(state) => state,
                    Err(e) => {
                        log::warn!("Horizon: a server mod set {}: {e}", b.state);
                        continue;
                    }
                };
                if let Err(e) = self.place_state([b.x, b.y, b.z], state, false) {
                    log::debug!(
                        "Horizon: a server mod's block at {} {} {}: {e:?}",
                        b.x,
                        b.y,
                        b.z
                    );
                }
            }
        }

        /// A player broke (or placed) `state` at `pos`: the player acting now.
        pub(in crate::game) fn mod_block_event(
            &mut self,
            pos: [i32; 3],
            state: BlockStateId,
            placed: bool,
        ) {
            if !self.hears_players() {
                return;
            }
            let who = self.lan_acting.map_or(Who::Local, Who::Conn);
            let state = self.state_string(state);
            self.mods.events.push(ModEvent::Block {
                who,
                dimension: self.dim.id().to_owned(),
                pos,
                state,
                placed,
            });
        }

        /// A chat message a player sent.
        pub(in crate::game) fn mod_player_chat(&mut self, who: Who, text: &str) {
            if self.hears_players() {
                self.mods.events.push(ModEvent::Chat(who, text.to_owned()));
            }
        }

        /// `/line` typed by the window's own player in-frame: true when a
        /// server mod owns it.
        pub(in crate::game) fn local_server_mod_command(&mut self, line: &str) -> bool {
            if !self.hears_players() {
                return false;
            }
            // Flush older outputs before capturing an inventory. The command's
            // checked edit is then applied before another game action runs.
            self.show_mod_output();
            let players = self.server_players(None);
            let Some(me) = players.first().cloned() else {
                return false;
            };
            let snapshot = self.server_mod_snapshot(players, None, Some(&me.id));
            self.mods.mods.set_snapshot(snapshot);
            self.lend_world();
            let owned = self.mods.mods.server_command(&me, line);
            self.return_world();
            self.show_mod_output();
            self.apply_mod_block_writes();
            owned
        }

        /// `/line` typed by a connection's player: true when a server mod
        /// owns it, whatever the player's permission level.
        pub(in crate::game) fn guest_server_mod_command(
            &mut self,
            host: &mut LanHost,
            conn: u32,
            line: &str,
        ) -> bool {
            if !self.hears_players() {
                return false;
            }
            self.route_mod_output(Some(host));
            let Some(g) = host.hub.guest(conn) else {
                return false;
            };
            let id = uuid_string(g.uuid);
            let players = self.server_players(Some(host));
            let Some(me) = players.iter().find(|p| p.id == id).cloned() else {
                return false;
            };
            let snapshot = self.server_mod_snapshot(players, Some(host), Some(&me.id));
            self.mods.mods.set_snapshot(snapshot);
            self.lend_world();
            let owned = self.mods.mods.server_command(&me, line);
            self.return_world();
            if !owned {
                return false;
            }
            self.route_mod_output(Some(host));
            self.apply_mod_block_writes();
            true
        }

        /// Sends a server mod's line to the player with this id: the
        /// window's own chat for the local player, a system message for a
        /// connection's. `host` is the hub when it is out of `self.lan`.
        pub(in crate::game) fn tell_server_player(
            &mut self,
            host: Option<&mut LanHost>,
            id: &str,
            line: Text,
        ) {
            let local = self.options.execution_role.is_client()
                && self.remote.is_none()
                && self.player.is_some();
            if local && self.player_uuid_string() == id {
                self.add_mod_chat_line(line);
                return;
            }
            let Some(host) = host.or(self.lan.as_mut()) else {
                return;
            };
            let to = host
                .hub
                .guests
                .iter()
                .find(|g| g.placed && uuid_string(g.uuid) == id);
            if let Some(conn) = to.map(|g| g.conn) {
                host.hub.send_to(
                    conn,
                    &C::SystemChat(play::SystemChat {
                        content: component_nbt(&line.component()),
                        overlay: false,
                    }),
                );
            }
        }

        /// Routes a server mod's output: `Tell` and `Reply` to their player,
        /// `Broadcast` to everyone and the log.
        pub(in crate::game) fn route_server_output(
            &mut self,
            mut host: Option<&mut LanHost>,
            output: Output,
        ) {
            let named = |name: &str, text: String| {
                let mut line = Text::literal(format!("[{name}] ")).with_color(text::GRAY);
                line.push(Text::literal(text));
                line
            };
            match output {
                Output::ItemEdit(edit) => {
                    let result = self.apply_item_edit(host.as_deref_mut(), &edit);
                    let line = match result {
                        Ok(()) => Text::literal(format!("[{}] item state updated", edit.owner)),
                        Err(error) => {
                            Text::literal(format!("[{}] {error}", edit.owner)).with_color(text::RED)
                        }
                    };
                    self.tell_server_player(host, &edit.player, line);
                }
                Output::Tell { name, player, text } => {
                    self.tell_server_player(host, &player, named(&name, text));
                }
                Output::Reply {
                    player,
                    text,
                    error,
                } => {
                    let line = Text::literal(text);
                    let line = if error {
                        line.with_color(text::RED)
                    } else {
                        line
                    };
                    self.tell_server_player(host, &player, line);
                }
                Output::Broadcast { name, text } => {
                    let plain = format!("[{name}] {text}");
                    if self.options.dedicated.is_some() {
                        super::super::dedicated::log("INFO", &plain);
                    } else {
                        log::info!("{plain}");
                    }
                    for p in self.server_players(host.as_deref()) {
                        let line = named(&name, text.clone());
                        self.tell_server_player(host.as_deref_mut(), &p.id, line);
                    }
                }
                Output::SetBlocks {
                    name,
                    dimension,
                    blocks,
                } => {
                    let queued = &mut self.mods.block_writes;
                    queued.extend(blocks.into_iter().map(|b| (dimension.clone(), b)));
                    if queued.len() > MAX_QUEUED_WRITES {
                        let over = queued.len() - MAX_QUEUED_WRITES;
                        queued.drain(..over);
                        log::warn!(
                            "Horizon: mod {name} set more blocks than wait for their level: {over} dropped"
                        );
                    }
                }
                _ => {}
            }
        }
    }
}
