# Registered-fluid minecart currents

Part of sofia-gros/horizon-mods#379, stacked on host draft #62.

The shared authoritative minecart tick now applies configured registered-fluid
currents immediately after its native water/lava current refresh. This is after
rail/off-track movement, block effects and entity pushes, and before cart variant
behavior. It covers rideable, chest, hopper, furnace, TNT, command-block and
spawner carts, in both existing track-behavior modes.

The existing registered entity adapter samples the current float-sized minecart
body and its existing fluid-eye position, with body deflation and loaded-only
bounded queries. Positive body families use their explicit motion scale and
non-player current normalization/minimum drift. Missing, invalid or failed
queries add no registered force; zero scales opt out. NoGravity remains pushable.
Remote prediction never receives this extra force.

Custom currents do not set in_water, reset fall_distance, change native gravity
or drag selection, or register custom fluids as water/lava. Current velocity
naturally reaches the following variant logic: experimental TNT-cart collision
speed and fuse-expiry blast speed include it, just as they include native pushes.
The old-mode collision-speed capture and existing pending fall-blast behavior
are unchanged. Container, fuel, hopper, command, spawner, rail, sound and rider
logic are not otherwise changed. No new saved/network fields or schema.

## Source and verification

Original Rust integration with the host minecart tick and shared current adapter,
using the already attributed MIT Tinkers' Construct TinkerFluids.cool()/hot()
motion-scale policy at f4e77d43f39ba1047bf93ae816b7e1beaab514c2,
Copyright (c) 2022 SlimeKnights. No additional external implementation, Forge
source or game assets were read or copied. Exact source tick-order parity is
not claimed.

Brief static review, rustfmt and git diff --check only. Two unrun unit tests
exercise actual cart geometry for all seven kinds, source/flowing-family
coalescing, non-player normalization, NoGravity eligibility, native/variant
metadata preservation and moved-body/missing-fluid sampling. They exercise
geometry/current kernels, not the live Game tick or variant explosion outcomes.
No tests, builds, injection planning, CI/version checks, integration or merge
were performed. Consolidated verification belongs to the separate batch worker.

Remaining outside this slice: registered minecart burn/extinguish, fall scaling,
custom travel/buoyancy and other vehicle/projectile current wiring.
