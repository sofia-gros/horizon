# Registered fluid flow and scheduling (draft)

This describes the simulation layer built on the registered fluid catalog.
Both directions use the draft resource, game-world, renderer, contact and bucket
adapters. Upward declarations now load, select inverted flowing artwork and
sample upward currents. No Ichor/Cinderslime definitions ship in this host patch.

`RegisteredFlow` executes the existing host FlowingFluid transition and spread
ordering with catalog-scoped identities. Every family needs an explicit policy:
drop-off, slope-search distance (bounded to eight), tick delay and source
conversion. There is no implicit lava policy, burning or solidification.
The extended bound admits TConstruct's glass policy (drop eight, slope eight,
tick delay thirty) without substituting a different liquid's parameters.
Slope results are cached by cell, distance and incoming direction for one spread
decision, avoiding repeated cyclic searches. The cache is not consulted after
world mutations and is never retained between ticks or liquid families.
`vertical_direction` accepts `down` (the default for existing declarations) or
`up`. Simulation reverses the preferred spread direction, feeding neighbor,
hole/slope queries, wall/replacement directions and source-conversion support.
Upward fluid rises first and spreads beneath ceilings; it retains the same
source/flowing IDs, level amounts and persisted vertical-column flag. The
existing flag is named `falling` but does not by itself encode physical direction.
Each family has its own direction, with the existing loaded/protected-cell and
commit/scheduling boundaries unchanged.

The inversion follows MIT Mantle `InvertedFluid` at
`17c287ad52f7b10154e07fa7570dc7bccd74ca2b`; see THIRD_PARTY_NOTICES.md.
Mantle's `ClientInvertedFluidType` substitutes a `flowing_inverted` texture but
does not mirror vertex geometry; `InvertedFluid` does not override fluid height
or shape. Keep the existing bottom-based surfaces and body/eye sampling rather
than inventing ceiling-anchored volumes. The world renderer now appends
`_inverted` to the declared flowing sprite for upward families; resource authors
must provide that texture. Missing artwork fails mesh-table construction.
The shared current path looks above empty neighboring cells and applies upward
column wall deflection, with the source's f32 arithmetic order. Player and mesh
sampling use the same explicit family direction. Native currents stay downward.
Exact movement/source parity, visuals, gameplay and save/LAN need final validation.

The authoritative adapter supplies loaded cells, support and wall occlusion,
fluid acceptance/replacement rules, protection callbacks and compare-before-write
mutation. Successful writes schedule the cell and neighboring registered
fluids. Placement succeeds only after committing the source block; pickup
returns a fluid only after removing a source. The caller must stage and commit
the associated inventory transaction and guest synchronization.

`RegisteredFluidScheduler` uses a separate host `LevelTicks` queue so custom IDs
cannot alias vanilla enum discriminants. Chunk loading validates the entire
named NBT list before mutation. Saved delays begin when a pending chunk becomes
eligible to tick, not when its bytes load. Saves/unloads during an active batch
are rejected to avoid dropping already-collected ticks. Reloading a reordered
catalog resolves the same names, not the previous numeric slots.

Tests execute placement, downward and lateral propagation, nearest-hole
selection, source-only pickup and draining after source removal through the
production scheduler. Other cases cover distinct families, stale ticks,
protected/unloaded cells, explicit source conversion, timing, chunk save/reload,
paused chunks, wrong-chunk data and active-batch lifecycle restrictions.
New upward cases cover rise/ceiling spread, pickup/draining, ceiling-hole
selection and source support, independent family directions and unloaded upper
cells. Current cases cover above/below neighbor selection, foreign-family
isolation and upward column push. Resource/mesh cases retain ordinary height
and eye semantics while selecting the inverted flowing sprite. An inherited
invalid-policy fixture now uses slope 9, matching the
previously expanded maximum of 8. These new/updated cases were not run: only
brief static review, rustfmt and whitespace review were performed. Builds,
tests, CI inspection, version checks and merges remain deferred.

Remaining integration gates: load source-verified definitions; attach the
catalog, queues and callbacks to authoritative game/chunk lifecycles; bind
custom meshes and immersion; connect local/guest bucket inventory operations;
and run the complete path through those production adapters. Keep this branch
unmerged until the complete implementation is independently reviewed.
