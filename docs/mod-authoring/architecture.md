# Architecture and capabilities

Horizon supplies general APIs and hooks; mods supply their own behavior. Code mods are WebAssembly components loaded by the host. They do not patch Horizon's internal code: they call declared host interfaces and export callbacks. Content packages carry data such as item definitions, recipes, models and textures. A complete feature can need both a content package and a server code package.

Client mods use `api = "0.1"`, `side = "client"` and implement `horizon_mod_sdk::Mod`. Server mods use `api = "0.3"`, `side = "server"` and implement `horizon_mod_sdk::server::ServerMod`. Server callbacks run where the world is simulated: single player, the LAN host or a dedicated server. A player's client must understand the custom GUI; a vanilla client does not gain it automatically.

In `init`, return the `Registration` for the callbacks and commands you want. For example, `.on_joins()` enables join/leave events; `.command("hello")` registers a command. Other hooks retain their defaults. Mining and melee callbacks have separate opt-ins; see [items and hooks](items-and-hooks.md). The [beacons server example](../../crates/horizon-mod-sdk/examples/beacons/) includes a manifest, command, world access and tests.

Every imported host interface must be declared in `capabilities` in `horizon-mod.toml`. Experimental interfaces must also appear in `experimental`. The package checker inspects actual imports and rejects undeclared ones. For example, using `workstations` needs `capabilities = ["workstations"]` and `experimental = ["workstations"]`; add other capabilities your code imports. The [manifest guide](../mods.md#the-manifest) and [generated reference](../api/mods-reference.md) are the source for exact names.

The design is motivated in part by past reports of malware in Minecraft mod downloads; [CurseForge's incident notice](https://support.curseforge.com/en/support/solutions/articles/9000228509-june-2023-infected-mods-detection-tool/) describes one such incident. Wasm isolation, declared capabilities and host-controlled operations reduce exposure, but they are not a guarantee that every mod is harmless or compatible. Mods can still request powerful allowed operations, conflict over resources, contain bugs or use hostile content. See [limits and compatibility](compatibility.md).

Horizon Mixins are a possible future extension, not a shipped part of this Wasm interface. A trusted host extension could add a missing host API or hook while ordinary mods remain Wasm components. Such an extension runs at a different trust boundary and can change the assumptions behind the sandbox.

## A small server hook

The following is a complete `src/lib.rs` for a server code mod. Its manifest below declares the only non-log host interface this example uses. Replace the scaffold's `src/lib.rs` and `horizon-mod.toml`, and replace its client scenario with the server scenario shown afterwards.

```rust
use horizon_mod_sdk::server::{players, Player, Registration, ServerMod};

struct JoinNotice;

impl ServerMod for JoinNotice {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Self, Registration::new().on_joins()))
    }

    fn player_joined(&mut self, player: &Player) {
        players::tell(&player.id, &format!("Welcome, {}!", player.name));
    }
}

horizon_mod_sdk::export_server_mod!(JoinNotice);
```

```toml
# horizon-mod.toml
id = "horizon_docs_server"
name = "Join Notice"
version = "0.1.0"
description = "A server-side join notification"
authors = []
api = "0.3"
side = "server"
capabilities = ["players"]
```

```toml
# tests/hello.toml
[[step]]
join = "Alex"
expect.told = ["Alex: Welcome, Alex!"]
```

Build and test with `cargo horizon-mod test`, then check the package using `cargo horizon-mod check target/horizon-mod/horizon_docs_server.hmod`. This test runs a simulated join without a game; use a server or single-player world to check actual deployment and networking. The callback has no return value; for callbacks that return a `Result`, handle `Err` and do not assume queued actions were committed. The SDK's default callbacks let you implement only the ones your feature needs.
