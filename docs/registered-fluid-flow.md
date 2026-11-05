# Registered fluid flow and scheduling (draft)

This builds on the registered fluid catalog. It is an executable simulation
layer, not yet a player-facing feature: no production resource, game-world,
renderer, immersion or bucket-inventory adapter enables it.

`RegisteredFlow` executes the existing host FlowingFluid transition and spread
ordering with catalog-scoped identities. Every family needs an explicit policy:
drop-off, slope-search distance (bounded to eight), tick delay and source
conversion. There is no implicit lava policy, burning or solidification.
The extended bound admits TConstruct's glass policy (drop eight, slope eight,
tick delay thirty) without substituting a different liquid's parameters.
Slope results are cached by cell, distance and incoming direction for one spread
decision, avoiding repeated cyclic searches. The cache is not consulted after
world mutations and is never retained between ticks or liquid families.
Inverted flow and fluid-specific interactions need separate source-verified
policies/adapters and are not represented by this downward-flow implementation.

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

Remaining integration gates: load source-verified definitions; attach the
catalog, queues and callbacks to authoritative game/chunk lifecycles; bind
custom meshes and immersion; connect local/guest bucket inventory operations;
and run the complete path through those production adapters. Keep this branch
unmerged until the complete implementation is independently reviewed.
