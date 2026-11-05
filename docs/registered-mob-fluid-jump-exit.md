# Registered-fluid mob jumping and exit (#379)

This opt-in host slice extends common registered-fluid travel with the existing
`travel.jump` parameter and collision-aware exit/step-up. It stacks on the mob
travel slice; no content, saved fields, native tags or network format changes.

When existing mob AI asks to jump and native water/lava is absent, the deepest
configured registered body contact supplies the jump strength. An airborne mob
or one deeper than its fluid jump threshold adds that strength to vertical
velocity. A shallow grounded mob retains ordinary ground jumping and its delay.
The existing species threshold remains in use, including Breeze's eye-height
threshold. Explicit zero strength consumes a deep fluid jump without falling
through to ground jumping. Horizontal velocity and native fluid flags are
unchanged. Native water/lava jumping and their species overrides are untouched.
This does not create a swim/float goal or interpret `can_swim` as a native tag.
The generic declaration, rather than a water/lava-specific bee or magma-cube
jump override, governs these new registered-fluid jumps.

After common registered-fluid movement and damping, the existing native fluid
exit code probes ahead using the old Y position and resolved velocity. A
horizontal collision can produce the existing 0.3F vertical step only if the
prospective box is clear of block collisions, native liquids, registered liquid
surfaces and collidable living shulkers. Native water/lava exit shares this
helper, so a registered liquid ahead no longer counts as a dry exit. Without
registered resources, the additional predicate is inert and native behavior is
unchanged, including the subsequent ridden-water buoyancy phase.

Registered dry-space probes use the bounded catalog geometry and only loaded,
in-height blocks. Any missing sample, invalid bounds or exhausted budget rejects
the exit rather than assuming dry air. They consider all registered families,
not only the current travel family or families with interaction policies. A
thin layer entirely below the destination's feet does not block exit.

This is original wiring of Horizon's existing jump declarations, mob movement
and contact geometry. No external implementation or game assets were read or
copied. It does not claim Tinkers'/Forge species-specific jumping or tick-order
parity. Existing source notices for the underlying registered fluids remain.

Brief static review, rustfmt and whitespace inspection only. Four added
synthetic regressions cover jump strength, depth/ground thresholds, zero strength,
Breeze's threshold, dry/source/flowing/other-family/shallow exit geometry, missing
samples and a query rejected before any overbudget read. Tests are **not run**;
they do not replace live AI/collision/step-up/network coverage. No tests, builds,
CI inspection, version checks or merge were performed.

Remaining: swimming/float AI activation, automatic ridden custom-fluid buoyancy,
fall-distance policy, drowning and passenger boat shielding. Flying and
species-consumed travel paths remain as documented by the earlier slice, while
any mob reaching the shared jump handler can consume a registered jump policy.
Custom mob status behavior and exact source/mixed-fluid order remain separate.
The final integration pass is still required; #379 remains open.
