# Registered-fluid currents for armor stands

Part of sofia-gros/horizon-mods#379; stacked on host PR #54.

Authoritative armor stands now receive configured registered-fluid current
forces after native water/lava current updates and before stand travel. This
connects all 70 existing fluid motion-scale declarations without adding native
fluid identity. The existing native-water boolean still controls native travel
and fall-distance resetting; registered contact does not change that boolean.

Marker stands return before sampling. Small stands use their existing half-size
body and eye geometry; NoGravity only suppresses gravity, not current forces.
The existing entity-ticking chunk filter remains in place, and remote guests
receive no new registered simulation.

Dropped items and armor stands now share one entity current adapter. It reuses
the existing bounded loaded/in-height flow sampler, including upward currents,
and the existing non-player `EntityFluidWorld` target. Family-specific scales,
normalization, stationary minimum drift and finite-add checks are unchanged.
Failed sampling, eye-only contact, zero policy scales and canceled currents
cannot add force. The item caller's sampling and phase ordering are unchanged.

The source policy is MIT Tinkers' Construct `TinkerFluids.cool()` / `hot()` at
[f4e77d43f39ba1047bf93ae816b7e1beaab514c2](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/TinkerFluids.java).
This slice is original host wiring to existing helpers; no additional external
implementation was read or copied.

This adds current forces only. Registered armor-stand burn/effects,
extinguishing, travel/fall policies, projectile currents and full source
integration parity remain separate. No stand fields, equipment, health,
serialization, network format or item pickup behavior change.

One unrun geometry-and-policy regression covers normal versus small stands,
marker exclusion and NoGravity retaining current eligibility. Existing item
policy and native-water stand regressions remain unchanged. Brief manual review,
rustfmt and whitespace checks completed. No tests, builds, native injection
planning, CI inspection, version checks or merge were performed as requested.
