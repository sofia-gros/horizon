# Registered-fluid armor-stand fall-distance policy

Part of sofia-gros/horizon-mods#379, stacked on host stand extinguishing (#56).

## Behavior

The authoritative stand fluid refresh now multiplies its existing
`enchantment_fall_distance` by each positively immersed registered family's
`fall_distance_scale`, before travel. The current TConstruct declarations use
0.5. Contacts are aggregated per family: touching multiple source/flowing cells
of one family applies its factor once. Different touching families each apply
one factor, and later fluid refreshes apply the factors again.

Fall scaling is independent of current strength and direction. A zero motion
scale or canceled current does not disable it. The current adapter returns its
already sampled contacts; if flow sampling fails, the stand falls back to the
existing bounded loaded-only geometry sampler, without inventing any current.
Small stands use their own body size. Markers have no registered immersion;
NoGravity does not exempt a positive body contact. Eye-only, missing and invalid
contacts do not scale the accumulator.

The existing game loop then adds this tick's downward displacement and keeps its
native-water and on-ground clearing behavior. This does not add health damage
from falls: it updates the accumulator already used by stand landing/enchantment
handling. No motion, health, equipment, native water flags or landing callbacks
are changed. Remote-client and non-ticking-chunk exclusions remain at the caller.

## Source and scope

This is original Rust wiring of the existing registered-fluid interaction
contract, matching the player and common-mob adapters already in this stack.
No new external source, Forge implementation or game assets were read or copied.
The bounded stand geometry helper is shared with extinguishing; its behavior is
unchanged. Dropped items ignore the new returned contact snapshot and retain their
existing current behavior.

Registered stand travel, burn/effect callbacks and drowning remain separate.
Exact source event-order parity, live collision/landing integration and native
mod injection compatibility are not validated by this slice.

## Review and deferred validation

Brief static review, rustfmt and `git diff --check` only. Three new unit tests
cover family deduplication and repeated refreshes, zero/identity and eye-only
policies, and small/marker/NoGravity/missing/invalid geometry. They were not run
and are not live game integration tests. Builds, tests, injection planning, CI
inspection, version checks and merges remain deferred to the separate batch
integration workflow.
