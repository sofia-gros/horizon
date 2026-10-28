# Workstations and inventories

A custom workstation is a contract between a content declaration, a server mod and a Horizon client. The content block must name its server handler with `workstation_handler`; registering a block does not let an unrelated mod take it over. The server mod declares experimental `workstations` and calls `workstations::register_gui` during `init`. The registered JSON defines bounded dimensions, input/storage/output slots, acceptance selectors, buttons and visual elements. See the working [TConstruct server registration](https://github.com/sofia-gros/horizon-mods/blob/ce3a5ab5e8a51ea01d1da502ce5f11d0584ef0ff/Tinkers%27%20onstruct/server-tools/src/lib.rs) and [content block declaration](https://github.com/sofia-gros/horizon-mods/blob/ce3a5ab5e8a51ea01d1da502ce5f11d0584ef0ff/Tinkers%27%20onstruct/data/tconstruct/horizon/block/tinker_station.json).

`ServerMod::gui_workstation_preview` receives ordered snapshots and an optional selection. Return a `GuiWorkstationPlan` with per-slot consumption and output descriptions, or `Ok(None)` when no recipe matches. A consumption count of zero retains that input. The callback is a preview, not a commit: queued host outputs are discarded and persistent storage writes are blocked, but the guest's own memory is not rolled back. Keep the preview deterministic and free of side effects.

When the player takes a result, the host rechecks block ownership, reach, complete input stacks, output validity and capacity before applying the transaction. A stale input or insufficient capacity rejects the take. Multiple output items are committed together; unsupported click modes are refused. A button callback is not itself an inventory transaction: its immediate storage and progress writes are not generally rolled back on failure. Handle errors rather than assuming a button press committed a recipe.

`storage` slots belong to the block and survive closing and world saves; transient `input` slots return on close. Other viewers receive updates. Changing saved slot IDs, order or count requires migration. A `shared_selection` is synchronized while the world is open but is not saved across a world reload. The current main supports bounded images, button states, icons, scroll grids and contextual labels. The name-entry and variable-slot-layout work under review is not part of this baseline.

For tick- or command-driven machines, `storage_snapshot` returns a call-scoped token; `storage_commit` queues an atomic update. The host rechecks the block, ownership, snapshot, filters and capacity before applying it. A successful queue is not proof of a committed change, so do not advance irreversible private state just because it was queued. See [the full workstation contract](../mods.md#declarative-server-workstations-experimental-fork-api-17-and-later) and [automatic transactions](../mods.md#automatic-storage-transactions).

## Minimal registration

This example registers the menu and deliberately produces no recipes yet. It needs a content package that defines `example:press` and delegates `workstation_handler` to `horizon_docs_gui`; registration alone does not create the block. Replace the scaffold's `src/lib.rs` with:

```rust
use horizon_mod_sdk::server::{workstations, GuiWorkstationPlan, Registration, ServerMod, WorkstationInput};

struct Press;

impl ServerMod for Press {
    fn init() -> Result<(Self, Registration), String> {
        workstations::register_gui(
            "example:press", "Press", &["example".into(), "state".into()], &[],
            r#"{"width":176,"height":166,"slots":[{"id":"input","x":8,"y":18,"role":"input"},{"id":"result","x":134,"y":18,"role":"output"}],"elements":[]}"#,
        )?;
        Ok((Self, Registration::new()))
    }

    fn gui_workstation_preview(&mut self, _block: &str, _inputs: &[Option<WorkstationInput>], _selection: Option<&str>) -> Result<Option<GuiWorkstationPlan>, String> {
        Ok(None) // No recipes yet.
    }
}

horizon_mod_sdk::export_server_mod!(Press);
```

```toml
# horizon-mod.toml
id = "horizon_docs_gui"
name = "Press"
version = "0.1.0"
description = "Minimal workstation registration"
authors = []
api = "0.3"
side = "server"
capabilities = ["players", "workstations"]
experimental = ["workstations"]
```

A startup-only `tests/hello.toml` can contain `[[step]]` and `expect.commands = []`. `cargo horizon-mod test` and `check` validate the component and registration; they do not test a recipe, ownership delegation, or a real menu. Implement a preview and test the actual content/server pair before shipping. The `players` capability is imported by this SDK's server exports and must be declared even when this example does not call a players function.
