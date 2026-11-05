# Registered-fluid float/swim AI activation (#379)

This opt-in host slice lets existing common float/swim behaviors request jumps
in a registered liquid whose selected interaction policy permits swimming. It
stacks on the mob fall-distance slice. No new goals, entity tags, content
coefficients, saved fields or protocol format are introduced.

Covered entry and continuation predicates:

- GoalSelector `FloatGoal` (continuation delegates to the same `can_use`).
- Animal brain `Swim`.
- Monster brain `Swim`, retaining its existing Creaking movement guard.
- Villager brain `Swim`.
- Warden brain `Swim`.

Native water depth and lava retain the original short-circuit eligibility.
Otherwise bounded, loaded-only registered body observations select the deepest
configured family, with last encounter winning ties. This is the same selected
policy used by common travel and configured jumping. That policy must have
`can_swim: true`, and its body depth must be strictly greater than the mob's
existing fluid jump threshold. Eye-only, missing, shallow or non-swimmable
contacts do not activate these behaviors. A deeper non-swimmable family does
not borrow permission from a shallower swimmable one.

The current eleven enabled declarations are beetroot soup, earth slime, ender
slime, honey, ichor, magma, meat soup, mushroom stew, rabbit stew, sky slime and
venom. The other declarations remain non-swimmable. Existing behavior priorities,
control flags, start/end timers and the 0.8 random jump-request cadence remain
unchanged. The existing navigation float configuration is not broadened, and
mobs that do not already have one of these behaviors are not given it. Requests
use the configured jump integration from the previous slice.

This is original wiring of existing Horizon declarations, contact selection and
AI predicates. No external implementation or game assets were read/copied. It
does not claim exact Tinkers'/Forge species or mixed-fluid behavior parity.
Existing underlying registered-fluid source attributions continue to apply.

Brief static review, rustfmt and whitespace inspection only. Three synthetic
regressions cover strict depth/policy eligibility, shared travel selection and
tie ordering, configured jump connection, source/flowing geometry, missing and
shallow samples, and species eye-height thresholds. They are **not run** and do
not substitute for live goal selection, timer, RNG, movement or network coverage.
No tests, builds, CI inspection, version checks or merge were performed.

This does not add swimming navigation/path classification, aquatic movement or
hydration, a native swimming pose, species-specific swim-stroll/breaching logic,
automatic ridden custom-fluid buoyancy, passenger boat shielding or custom mob
status behavior. Those and the final integration pass remain open; #379 is not
complete.

Immediate boat-passenger shielding for registered observations is covered by the
later `registered-mob-fluid-boat-shield.md` slice, using the boat's existing status.
