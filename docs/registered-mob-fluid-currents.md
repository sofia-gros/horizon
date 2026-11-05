# Registered-fluid mob currents (#379)

This opt-in host slice connects the existing registered liquid current sampler
and `interaction.motion_scale` to authoritative mobs. It stacks on the mob
extinguishing slice. Existing content declarations are sufficient; no content,
version, protocol or persisted entity field is added.

At each existing mob fluid-refresh opportunity, native water/lava currents run
unchanged, then each positively immersed registered family applies its own
configured current. Source and flowing states resolve to the same family; other
registered liquids and native fluids remain separate. No water/lava flags,
swimming flags, splash events, fall distance or travel coefficients are changed.

Players and mobs now share the previous player flow sampling function without
changing the player's boat clipping or flying/spectator handling. It uses the
same deflated body box, loaded-only and in-height reads, 8192-read budget,
neighbor height differences, shallow-contact weighting and inverted-flow
wall/vertical calculation. A failed query applies no registered current. It
does not load chunks. The mobs' existing velocity target normalizes a nonzero
sum rather than using the player's average, applies the configured scale and
minimum stationary drift, and rejects nonfinite velocity additions.

The existing mob push predicate remains authoritative: tadpoles, cod, salmon,
tropical fish, pufferfish, squid, glow squid, dolphins, turtles, axolotls, frogs,
nautiluses and zombie nautiluses are not pushed. Fire immunity alone does not
prevent a current. Zero motion scale, eye-only contact and cancelled currents
produce no push. Registered families are processed in the sampler's encounter
order, after native currents.

Policy reference: MIT Tinkers' Construct
[`TinkerFluids.cool()` and `hot()` at f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/TinkerFluids.java),
which explicitly declare motion scale `0.0023333333333333335D`. The license was
read at that revision. This is host wiring using Horizon's existing
`CurrentAccumulator` and flow adapter, not a port of Forge's generic-fluid
implementation; no Forge implementation or game assets were read or copied.
Upward flow continues to use the previously attributed MIT Mantle algorithm.

Brief static review, rustfmt and whitespace inspection only. Added synthetic
regressions cover per-family scaling, source/flowing identity, non-player
normalization, all existing species exemptions, fire-immune nonexemption,
zero/eye-only/cancelled currents, stationary minimum drift and both vertical
directions. These regressions are **not run**. No tests, builds, CI inspection,
version checks or merge were performed.

Limits: this adds current forces only, not registered-fluid mob travel,
buoyancy/jumping, swimming AI, drowning, fall reset, passenger boat shielding or
custom status behavior. It follows the host's existing refresh calls, including
fall-damage refresh outside native water, rather than introducing a new once-per-
tick cache. Exact source refresh timing, mixed-fluid ordering, live movement and
network behavior remain for the final integration pass. Separately simulated
dragon parts, dropped items and projectiles are outside this slice. #379 remains
open.

The later `registered-mob-fluid-travel.md` slice covers common mob acceleration,
collision-resolved movement, drag and gravity. Its documented exclusions remain.
