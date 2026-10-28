# Make and test a first mod

A code mod is a Rust crate compiled to WebAssembly and packaged as an `.hmod` component. The default scaffold is a **client** mod: it registers `/hello` and shows a local chat message. A server mod uses a different trait and manifest; see [architecture](architecture.md).

You need Git, Rust with the `wasm32-unknown-unknown` target, and the fork's mod tool. The runtime crate at this baseline requires Rust 1.95 or newer. These commands use a local SDK from the same checkout, avoiding an accidental mismatch:

```sh
git clone https://github.com/sofia-gros/horizon.git
cd horizon
git checkout fc4848f4d7920390ea3aa8e6a5f804111f93a45d
rustup target add wasm32-unknown-unknown
cargo install --path crates/cargo-horizon-mod --locked
cargo horizon-mod new ../my_mod --sdk-path "$PWD/crates/horizon-mod-sdk"
cd ../my_mod
cargo horizon-mod build
cargo horizon-mod test
cargo horizon-mod check target/horizon-mod/my_mod.hmod
```

The scaffold's `tests/hello.toml` invokes `/hello` twice and checks the replies without starting a game. `check` inspects the built package and runs its sandbox checks; it does not prove gameplay correctness. For a distributable project, replace the local SDK path in `Cargo.toml` with a Git dependency pinned to the Horizon revision you test, and keep `Cargo.lock`:

```toml
horizon-mod-sdk = { git = "https://github.com/sofia-gros/horizon.git", rev = "fc4848f4d7920390ea3aa8e6a5f804111f93a45d" }
```

To try it in a prepared Horizon client, set `HORIZON_CLIENT` to the executable or pass `--game PATH`, then run `cargo horizon-mod run`. This builds and starts the game with `--dev-mod`; rebuild after an edit to trigger development reload. The game needs resources prepared from your own Minecraft installation, as explained in the [Horizon source-build instructions](../../README.md#build-from-source). Reload restarts in-memory mod state; declared mod storage persists. Test content registration changes after restarting the game.

The template's actual Rust is in [`template/src/lib.rs`](../../crates/cargo-horizon-mod/template/src/lib.rs); its manifest declares the `chat` capability. Content-only projects can instead use `cargo horizon-mod new ../my_content --template content`. See [manifest and testing details](../mods.md#the-manifest), including how to replace the template scenario when your mod changes behavior.
