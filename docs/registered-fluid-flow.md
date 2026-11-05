# Registered fluid flow and scheduling (draft)

This describes the simulation layer built on the registered fluid catalog.
The ordinary downward path has separate draft resource, game-world, renderer,
contact and bucket adapters. The new upward path is simulation-only and is
deliberately rejected by the live resource loader until its remaining adapters
are ready.

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
Rendering, body/eye geometry, currents and any inverted travel policies remain
unimplemented. `RegisteredFluidResources::from_reader` rejects `up` rather than
silently rendering a rising liquid on the bottom of the cell. No Ichor or
Cinderslime declaration is enabled by this patch.

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
cells. An inherited invalid-policy fixture now uses slope 9, matching the
previously expanded maximum of 8. These new/updated cases were not run: only
brief static review, rustfmt and whitespace review were performed. Builds,
tests, CI inspection, version checks and merges remain deferred.

Remaining integration gates: load source-verified definitions; attach the
catalog, queues and callbacks to authoritative game/chunk lifecycles; bind
custom meshes and immersion; connect local/guest bucket inventory operations;
and run the complete path through those production adapters. Keep this branch
unmerged until the complete implementation is independently reviewed.
