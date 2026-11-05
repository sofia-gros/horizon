# Registered-fluid primed-TNT currents

Part of sofia-gros/horizon-mods#379, stacked on host draft #61.

The authoritative primed-TNT loop supplies optional registered-fluid resources
to its tick. After movement, collision/bubble-column handling, drag, ground
bounce and fuse decrement, a surviving TNT first receives its existing native
water/lava push, then registered-family currents. An expired TNT samples neither
current path before its existing explosion dispatch. NoGravity does not imply
current immunity; the existing EntityRules push predicate is respected.

The shared non-player adapter is now exported for gameplay-owned entities. It
uses the TNT's current 0.98-by-0.98 body, existing float-rounded eye height,
0.001 body deflation and loaded-only bounded registered-flow sampler. Each
positive body contact uses its explicit motion scale, non-player direction
normalization and existing minimum-drift/finite-delta handling. Missing or
invalid contacts and flow-query errors add no registered force; a zero motion
scale opts out. Custom fluid identity never populates native immersion flags.

Remote TNT prediction explicitly passes no registered resources. Existing native
TNT tests pass the same opt-out. Fuse length, explosion power/attribution,
explosion water classification, random draws, border collision, slime bounce,
saves and network fields are unchanged. The shared adapter's other entity
callers are only renamed, with no behavioral change.

## Source and validation

Original Rust integration with the host TNT tick and registered-current adapter.
It consumes the explicit motion-scale policy already attributed to MIT Tinkers'
Construct TinkerFluids.cool()/hot() at
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, Copyright (c) 2022 SlimeKnights.
No additional external implementation, Forge source or game assets were read
or copied. No exact source tick-order parity is claimed.

Brief static review, rustfmt and git diff --check only. Two unrun unit tests
cover actual TNT body/eye geometry, moved-position sampling, non-player current
normalization, NoGravity eligibility, fuse expiry and blast-metadata preservation.
They exercise geometry and the current kernel, not live world/TNT-tick wiring.
Existing native test call sites were updated without changing their assertions.
Tests, builds, injection planning, CI/version checks, integration and merges are
left to the separate batch workflow.

This slice does not add registered-fluid TNT burn/extinguish, custom buoyancy,
fall/effect policies or any boat, minecart, falling-block or projectile currents.
