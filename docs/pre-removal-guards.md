# Pre-removal guard boundary

`LevelAccess::allow_block_removal(pos, state)` is a read-only, default-allow
preflight. `GameLevel` exposes an explicit implementation for native integration
(for example a source mixin checking a machine's saved inventory). This change
alone does not install a machine policy or make unsupported inventories safe.

Checks precede ordinary level storage writes, destroy-block drops and particles, standalone block loot,
player destruction side effects, and explosion loot/entity handling. Pistons
preflight the base/head, payload sources, destinations and destroyed positions
before changing the base or finalizing moving entities. Retraction resolves its
payload with a virtual absent head and checks pending moving-block states too.

An implementation must not mutate state, draw randomness, or change its answer
for an unchanged state because another position was written. It may be called
more than once. It should only reject the protected state, not incidental
notification order. Refusal is not a rollback mechanism. Direct raw-storage
callers, world-generation writes and multi-block application operations outside
these entry points remain responsible for their own preflight/atomicity.

The eight cases in `python3 tools/test_removal_preflight.py` compile production
code excerpts using a small in-memory adapter. They exercise repeated denied
and allowed destruction, drop counts, direct-write refusal, piston footprint
preflight and projected moving states. The resolver/world, resource facts and
post-preflight piston movement are mocks; these tests are not full gameplay or
full piston-motion tests. They run in CI without prepared game resources.

The existing resource-dependent level suite needs prepared Horizon resources.
A missing resource directory is not evidence those tests passed. Interactive
explosions, commands, client/LAN synchronization and downstream mod integration
remain required before relying on this boundary for a production inventory.
