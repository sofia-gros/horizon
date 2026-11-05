# Registered-fluid armor-stand travel

Part of sofia-gros/horizon-mods#379, stacked on stand fall scaling (#57).

## Behavior

The authoritative stand fluid refresh returns the deepest positive configured
body family's travel policy, reusing its current/fall contact snapshot. Equal
heights keep the last family in the sampler's deterministic order, like the
existing registered player and common-mob adapters. Eye-only, missing, invalid
or unconfigured contacts select no policy. Marker stands have no registered
immersion. Current strength and swim permission do not gate travel selection.

The stand tick preserves its normal collision resolution, world-border check,
powder-snow equipment context, support update and block pushes. It then applies
the selected policy's horizontal/vertical drag and scaled gravity with the
existing living-fluid falling adjustment. Falling is captured before the move
and block pushes, so the near-stopped-fall snap keeps the same phase. Ground
friction is not used while registered-fluid travel is active.

Native water keeps precedence. A stand with no registered policy retains the
existing air/native behavior, including native-lava behavior. Marker and
NoGravity early returns are unchanged: NoGravity does not start moving merely
because a custom fluid touches it. Stands have no movement input here, so this
slice does not add acceleration, jumping or AI. No native water/lava tag is
assigned, and native-water fall clearing remains independent.

The selected policy is passed directly from the refresh into that tick; it adds
no persisted, synchronized or stale cached stand state. Remote-client and
non-ticking-chunk exclusions remain at the game-loop caller. Dropped-item travel
is unchanged.

## Reference and scope

Original Rust integration of the existing registered-fluid travel contract and
host stand movement sequence. The drag/gravity and falling adjustment follow
the existing registered player/common-mob and native stand kernels in this
repository. No new external implementation, Forge source or game assets were
read or copied. Existing fluid-policy attribution is unchanged.

Registered stand burn/effect callbacks and drowning are not included. Fluid-wall
escape, boat passenger shielding, live collision outcomes, source tick-order
parity and native-mod injection compatibility are not established by this slice.
The existing host stand tick has no separate native-lava travel policy; this PR
does not introduce one.

## Review and deferred validation

Brief static review of changed functions/call sites, rustfmt and
`git diff --check` only. Three unit tests were added for deepest-family/tie and
eye-only selection, declared drag/gravity with collision-stopped axes, and the
premove falling snap plus source/flowing/marker geometry. Existing native stand
call sites explicitly pass no registered travel policy.

Tests were not run and are not live integration coverage. Builds, tests,
injection planning, CI inspection, version checks and merges remain deferred to
the separate batch integration workflow.
