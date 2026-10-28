# Items, mining and combat hooks

The server APIs are experimental. They operate on a player's actual stack, not on a global material or tool name. Declare every interface you use in both `capabilities` and, for experimental ones, `experimental`.

1. `items_read::main_hand(player_id, &path)` reads a bounded snapshot, including an optional string at the selected custom-data path. It can return no stack or an error; handle both.
2. `items_edit::replace_main_hand_text` queues a comparison against the expected old string. `items_tools::replace_main_hand_profile_durability` can also queue the tool profile and durability as one checked edit; the combat variant includes a melee profile. The host validates the data, owner and complete stack before applying it. An `Ok` from a queue call is not a promise that a later recheck will succeed.
3. A profile has one owner. Another mod cannot silently take over the same stack's profile. Other components of a surviving stack are preserved; use a namespace in your own custom-data path and do not target the reserved `horizon:tool_control` root.
4. Call `items_tools::enable_mining_events()` or `enable_melee_events()` during `ServerMod::init` and implement `mined` or `melee_hit`. The host calls the owning mod for eligible successful actions. A failed callback or trap discards the associated queued edit. Keep your own calculation deterministic and handle stale state.

[`welcome`](../../crates/horizon-mod-sdk/examples/welcome/src/lib.rs) contains real registration, reads, checked edits and mining callbacks. Its manifest declares `players`, `storage`, `items-read`, `items-edit` and `items-tools`, with the item interfaces marked experimental. Its fixture branches exercise errors and traps; they are tests, not production recipe logic. The [generated WIT reference](../api/mods-reference.md) gives exact signatures and bounds.

The current combat callback covers successful direct non-Creative hits, not every possible entity or sweeping secondary hit. Profiles and generic hooks are not a complete representation of every modifier from another game's mod ecosystem. A mod must implement its own material and gameplay rules.

## Minimal mining callback

This example opts in at initialization, reads the captured hand and queues a checked text change. It only demonstrates the hook: it does **not** assign an owned mining profile to an item. An eligible tool must already have a profile owned by this mod before the callback can be invoked. Production code should calculate real replacement state instead of the constant `"mined"`.

```rust
use horizon_mod_sdk::server::{items_edit, items_read, items_tools, Registration, ServerMod};

struct MiningExample;

impl ServerMod for MiningExample {
    fn init() -> Result<(Self, Registration), String> {
        items_tools::enable_mining_events();
        Ok((Self, Registration::new()))
    }

    fn mined(&mut self, player: &str, _dimension: &str, _pos: [i32; 3], _state: &str, _hardness: f32) -> Result<(), String> {
        let path = vec!["example".into(), "state".into()];
        let Some(stack) = items_read::main_hand(player, &path)? else { return Ok(()) };
        let Some(old) = stack.custom_text.as_deref() else { return Ok(()) };
        items_edit::replace_main_hand_text(player, &path, old, "mined")?;
        Ok(())
    }
}

horizon_mod_sdk::export_server_mod!(MiningExample);
```

```toml
# horizon-mod.toml
id = "horizon_docs_mining"
name = "Mining Example"
version = "0.1.0"
description = "Minimal checked mining hook example"
authors = []
api = "0.3"
side = "server"
capabilities = ["players", "items-read", "items-edit", "items-tools"]
experimental = ["items-read", "items-edit", "items-tools"]
```

This SDK export also imports `players` through its server interface, so the manifest declares it even though the callback does not call a players function. A scenario containing `[[step]]` and `expect.commands = []` checks that this server mod starts. It does not exercise a real mining event. Use an owned tool and a game or a suitable fixture to test callbacks and verify stale-stack, error and trap behavior.
