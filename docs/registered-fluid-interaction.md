# Registered fluid player interaction (draft)

The optional `interaction` declaration binds one explicit policy to the source
and flowing IDs of a family. It supplies travel acceleration, horizontal and
vertical drag, gravity scale, jump impulse, current scale, swim/drowning flags,
extinguishing, fall-distance scaling and optional burn seconds/damage. Invalid
numbers fail resource load. Missing policy keeps observations inert.

Player base ticks sample loaded cells, preserve independent currents per family,
apply configured push and fall scaling, and respect the boat's sheltered volume.
Native water/lava tags stay unchanged. A configured body contact uses its own
travel path, jump and collision escape; drowning is selected by the eye's family.
Block-inside burning uses that family's positive body-height guard, the declared
burn time and damage through the existing fire/health path. The same motion code
serves predicting and authoritative players.

Source: MIT TConstruct f4e77d43, `TinkerFluids` and `BurningLiquidBlock.entityInside`.
The generic configurable travel integrator uses Horizon's existing movement and
fluid-falling primitives, not Forge implementation code. Matching Forge defaults,
mixed-fluid precedence, and movement sequences still need an independent oracle.

Remaining: mobs/items/projectiles, custom potion effects and curative semantics,
source-specific burn audio, camera fog, fluid mixing/solidification, inverted
fluids, dispenser use, and live save/reload/LAN acceptance. This is not complete
#379 support and must stay draft. The paired TConstruct slice declares four
molten ore families only; other fluids are not silently enabled.

Validation: an initial focused registered-fluid unit run passed 19 tests before
final helper changes. Newly added policy/current regressions and the final batch,
full tests, CI review and merge remain deferred. Rust formatting and whitespace
were checked. `cargo check -p horizon-client-lib --lib --no-default-features --offline`
passed for the runtime changes. Do not treat focused checks as the final integration result.
