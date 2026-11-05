# Registered burning-fluid armor-stand contact

Part of sofia-gros/horizon-mods#379, stacked on stand travel (#58).

## Behavior

At the end of the authoritative stand environment tick, positive body contact
with an explicit burning-fluid policy extends fire duration to at least the
configured seconds times 20. It never shortens an existing longer burn. Contact
uses the shared bounded, loaded-only family sampler, including flowing surfaces
and small-stand geometry. Missing or invalid samples do not ignite anything.

This is ignition, not direct health damage. TConstruct's BurningLiquidBlock
sets fire before requesting lava-source damage, and the existing Horizon stand
model rejects direct lava damage. Therefore this adapter does not subtract the
policy's damage or play the accepted-hit burn sound. Native ongoing on-fire
health damage remains in its existing earlier phase with its existing rules.
Invisibility alone does not suppress the source's ignition step, but still
retains the existing stand fire-damage exclusion.

Markers and NoGravity stands retain the existing stand inside-block callback
exclusions. Dead stands do not get new ignition. Repeated cells from one family
have the same idempotent fire-duration maximum; since direct damage is rejected,
there are no additional per-cell damage or sound side effects to reproduce.

Earlier native-water/registered-fluid extinguishing and native fire/lava logic
remain intact. A stand can be extinguished and then reignited later in the same
tick, including Magma's combined extinguishing/burning policy. No healing,
equipment, movement, drops, persistence or sound changes are introduced.
Registered resources remain absent for remote-client ticks.

## Source and license

Adapted from MIT Tinkers' Construct
[BurningLiquidBlock.entityInside at f4e77d43f39ba1047bf93ae816b7e1beaab514c2](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/block/BurningLiquidBlock.java),
Copyright (c) 2022 SlimeKnights. The source and LICENSE were read at that pinned
revision. Attribution is retained in the adapter and THIRD_PARTY_NOTICES.md.
The host stand phase/geometry wiring is original Rust code. No Forge
implementation or Mojang files/assets were read or copied.

Current content has 58 burning families. Registered stand effects and drowning
remain separate. Exact source event-order and ongoing-fire cadence parity,
live death/drop behavior, and native-mod injection compatibility are unverified.
The callback samples the current body rather than the swept movement path.

## Review and deferred validation

Brief static review, rustfmt and `git diff --check` only. Three new unit tests
cover ignition without direct health/equipment/motion changes, exclusions and
body/surface gates, and extinguish-then-reignite behavior. They were not run and
are not live integration tests. Existing native stand lava behavior is retained.
Builds, tests, injection planning, CI inspection, version checks, integration and
merges remain delegated to the separate batch workflow.
