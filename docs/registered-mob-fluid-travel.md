# Registered-fluid ordinary mob travel (#379)

This opt-in host slice applies existing registered-fluid `travel` declarations
to mobs using the common living movement path. It stacks on the mob current
slice. It does not add or change content coefficients, fluid identity, saved
state, native tags or network formats.

Species-specific movement gets first refusal, as before; flying travel also
keeps its existing path. Native water/lava travel takes precedence over
registered liquids. A lava-walker standing on native lava keeps its existing
standing behavior. Otherwise a positive loaded-only registered body contact
selects the deepest configured family, with the last encountered family winning
equal-depth ties, as in the existing player adapter. Eye-only contact and a
surface below the mob's feet do not select a travel policy. Missing, invalid or
oversized contact queries leave the common air path unchanged.

The selected policy feeds the existing mob input rotation/acceleration and
collision/block-effect movement. Drag uses the collision-resolved velocity.
The gravity and falling predicate are captured before movement; the existing
fluid falling adjustment consumes scaled gravity after drag, matching the
registered player travel kernel. No-gravity, Slow Falling, the current mob
sprinting rule and the fluid vertical snap remain in that calculation. This
changes neither native water/lava membership nor swimming flags.

This is wiring for Horizon's existing opt-in `TravelPolicy`, not a new Forge
algorithm port. No external implementation, game assets or new source-derived
constants were read or copied for this slice. It makes no claim of exact
Tinkers'/Forge species-specific movement parity. The existing MIT source
attributions for declarations and contact/flow sampling remain applicable.

Brief static review, rustfmt and whitespace inspection only. Four new synthetic
regressions cover deepest/tied family selection, source/flowing contact,
eye-only/shallow rejection, post-collision velocity damping, scaled gravity,
no-gravity, Slow Falling, sprinting and the captured falling predicate. They
are **not run**, and do not substitute for a collision-world integration test.
No tests, builds, CI inspection, version checks or merge were performed.

Limits: this slice covers common acceleration, collision-resolved motion, drag
and gravity only. Configured jumping, registered-fluid exit/step-up, swimming
AI, buoyancy, fall-distance policy, drowning and passenger boat shielding remain
separate. Species overrides that consume travel, flying mobs and separately
simulated dragons remain on their prior paths; overrides that decline travel
can fall through to this common path. Custom mob effects and mixed-fluid/source
phase parity also remain open. Live collision behavior and network movement
need the final integration pass. This does not finish #379.
