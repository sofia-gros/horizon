# Registered-fluid armor-stand extinguishing

Part of sofia-gros/horizon-mods#379, stacked on host armor-stand currents (#55).

## Behavior

The authoritative armor-stand environment tick consumes the optional registered
fluid resources at the existing native-water fire-clearing phase. A positive
body height in a family with `interaction.extinguishes` clears `fire_ticks`
before ongoing on-fire damage/decrement. Native water behavior stays unchanged.
Native lava and fire handling still run afterward and can ignite the stand again.
Extinguishing does not heal the stand or change equipment, motion or fall distance.

Sampling uses the existing bounded, loaded-only family geometry, including
flowing surfaces and small stands. Marker stands have no registered immersion.
Invisible and NoGravity stands can still be extinguished. Unlit stands skip the
query. Missing resources, unloaded/out-of-height cells, invalid bounds, eye-only
contact and non-extinguishing families grant no new fire clearing. Remote-client
armor-stand ticks receive no registered resource context.

Current TConstruct declarations enable this policy for beetroot soup, earth
slime, ender slime, honey, ichor, magma, meat soup, mushroom stew, rabbit stew,
sky slime and venom. Liquid Soul does not enable it. Magma can both extinguish
and burn; this slice does not implement its armor-stand burn callback.

## Reference and scope

The opt-in policy comes from MIT Tinkers' Construct `TinkerFluids.cool()` at
[f4e77d43f39ba1047bf93ae816b7e1beaab514c2](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/TinkerFluids.java),
Copyright (c) 2022 SlimeKnights. This is original Rust host integration, reusing
the existing item extinguishing predicate and stand body/eye geometry. No Forge
implementation or game assets were read or copied. Exact source event-phase
equivalence is not claimed.

Registered armor-stand burn/effect callbacks, travel, drowning and fall policies
are outside this slice. Native stand damage, death, drops and sound behavior are
unchanged. As in its existing native-water path, fire clearing adds no sound.
The registered context is optional; no content or declaration changes are needed.

## Review and deferred validation

Brief static review and formatting only. Three new unit tests cover fire
clearing without other state changes, normal/small/marker and flowing-surface
geometry, and missing/invalid/eye-only contacts. They are not live game tests.
The existing native-only stand environment test passes `None` explicitly.
Tests, builds, injection planning, CI inspection, version checks and merging
were deliberately deferred under the requested implementation-first workflow.
