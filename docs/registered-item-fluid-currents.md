# Registered-fluid currents for dropped items

Part of sofia-gros/horizon-mods#379; stacked on host PR #53.

The authoritative dropped-item tick now applies each positively contacted
registered family's explicit `motion_scale`. All 70 current content declarations
use 0.0023333333333333335. This does not infer motion from burning, temperature,
swimming, item fire resistance or the fluid's name.

Native water/lava immersion and currents run first, unchanged. Registered
currents are applied next, before the old-velocity snapshot and ordinary item
movement. NoGravity and fire-resistant or Worldbound items can still be pushed.
The existing native-mod pre-tick cancellation still skips the entire item tick;
Worldbound's environment cancellation affects damage, not normal motion.

The existing shared player/mob flow sampler receives the item box deflated by
0.001. It preserves loaded/in-height queries, finite coordinates, the 8192-read
budget, family boundaries and upward-flow policy. Failed sampling applies no
registered force. An absent declaration, zero scale, zero/canceled current or
eye-only contact does not push the item.

The adapter reuses the existing `EntityFluidWorld` current target: accumulated
flow is normalized for a non-player entity, scaled by its family policy, and
subject to the native stationary minimum drift. Finite-vector/addition checks
remain in that target. Multiple contacted families apply in sampler order.
No new randomness, client-side registered prediction or entity fields are added.

The source motion-scale declarations are MIT Tinkers' Construct
`TinkerFluids.cool()` and `hot()` at
[f4e77d43f39ba1047bf93ae816b7e1beaab514c2](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/TinkerFluids.java).
The Rust wiring is original and reuses existing host behavior; no Forge
implementation was read or copied.

This slice adds current forces only. Native immersion heights, item buoyancy,
gravity and drag branches are not aliased to registered liquids. Registered
buoyancy/movement overrides, armor-stand/projectile currents and swept contact
remain separate work. Save/network formats, pickup, merging and lifetime do
not change. `ItemEntity::tick` gains an optional registered-resource argument;
all seven native-only item tick test calls explicitly pass `None`.

Three unrun policy regressions cover distinct source/flowing family scales,
non-player normalization, stationary minimum drift and vertical direction,
and eye-only/zero-scale/canceled-current opt-outs. Existing native item tests
are retained. Brief manual review, rustfmt and whitespace checks completed.
No tests, builds, native injection planning, CI inspection, version checks or
merge were performed as requested.
