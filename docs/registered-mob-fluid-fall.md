# Registered-fluid mob fall-distance policy (#379)

This opt-in host slice connects `interaction.fall_distance_scale` to mob fluid
refresh. It stacks on the common mob drowning slice. All seventy current
registered content families declare a factor of 0.5; no declaration or
coefficient changes here.

Each positively immersed family scales accumulated fall distance once per
fluid refresh, as in the existing registered player adapter. Source and flowing
cells of one family share one contact, so multiple cells do not multiply the
same policy repeatedly. Distinct families compose in contact encounter order.
A zero factor resets distance, one leaves it unchanged, and eye-only contact or
a surface below the feet does nothing. Subsequent ordinary landing damage and
hooks consume the resulting accumulated distance.

Fall policy is independent of current susceptibility. Mobs such as frogs,
turtles and fish retain their current-force immunity but can still receive a
fall-distance policy. Ordinary mobs reuse the current sampler's observations.
Current-immune mobs use bounded geometry without flow queries. If a current
query fails, a bounded geometry-only fallback can still scale the fall; its
zero flow vectors cannot invent a current or stationary minimum drift. Invalid
or oversized geometry produces no contact, and absent blocks are never loaded.

Native water/lava current handling is unchanged. The native water fall reset
still happens afterwards and wins in mixed water/custom contact. This does not
assign native liquid/swimming flags, reset a fall merely because a registered
fluid exists, change fall immunity, or replace block-specific landing rules.
No persisted field, content, protocol format or network event is added.

The application follows the host's existing fluid-refresh calls, including the
fall-damage refresh outside native water. It can therefore apply more than once
in a game tick and does not introduce a once-per-tick cache. Exact source phase
and mixed-fluid ordering are not claimed. Fresh downward movement is still
accumulated by the existing fall-damage path after its fluid refresh.

This is original wiring of Horizon's existing registered player policy and mob
fall handling. No external implementation or game assets were read/copied.
Existing declaration/contact source notices continue to apply.

Brief static review, rustfmt and whitespace inspection only. Four added
synthetic regressions cover source/flowing deduplication, ordinary landing
health, independent family factors, zero/identity factors, current immunity,
geometry-only zero current, eye/shallow/missing exclusion and successive refresh
scaling. They are **not run** and do not replace live drop/collision/network
integration. No tests, builds, CI inspection, version checks or merge.

Remaining work includes swimming/float AI, automatic ridden custom-fluid
buoyancy, passenger boat shielding, species-specific fluid behavior and custom
mob effects. Final live integration remains pending; #379 is not finished.

Existing common FloatGoal and Swim behavior activation is covered by the later
`registered-mob-fluid-float-ai.md` slice; navigation and species-specific limits
remain documented there.
