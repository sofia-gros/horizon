# Registered fluid contact observations (draft)

The catalog now samples bounded, loaded-cell-only body and eye intersections
using the fluid's actual level and same-family cell above it. Contacts aggregate
per catalog-scoped source identity. Separate families, including native water and
lava, remain distinct. Nonfinite, inverted and oversized queries fail before any
world reads; the caller supplies an explicit read budget.

Player motion ticks now populate `PlayerFluids.registered` from those observations
on both authoritative and predicting players. These are geometric observations,
not vanilla water/lava flags. They do not yet apply travel drag, buoyancy,
suffocation, fog, boat shelter, mob behavior or source-specific contact effects.
Those policies must consume the separate identity rather than silently treating
all custom liquids as lava. No fluid declaration ships with this stage.

Focused test passed: two distinct families, source/flow heights, eye intersection,
full column surface, unloaded cells and rejected query budgets/nonfinite bounds.
Formatting and public-tree checks pass. New motion adapter compilation and runtime
fixtures remain deferred to the requested full integration batch. Keep draft.
