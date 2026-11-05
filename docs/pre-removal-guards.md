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

The eighteen cases in `python3 tools/test_removal_preflight.py` compile production
code excerpts using a small in-memory adapter. They exercise repeated denied
and allowed destruction, drop counts, direct-write refusal, piston footprint
preflight and projected moving states. The resolver, retraction decision, autonomous moving-piston tick/finalTick and
command method are production code. World/storage and resource facts are mocks;
these tests are not full gameplay or full piston-motion integration tests. They run in CI without prepared game resources.

The existing resource-dependent level suite needs prepared Horizon resources.
A missing resource directory is not evidence those tests passed. Interactive
explosions, commands, client/LAN synchronization and downstream mod integration
remain required before relying on this boundary for a production inventory.

Command replacement now leaves the old entity attached until the guarded storage
write succeeds (strict and normal paths); failed writes cannot erase inventory.
Autonomous moving-piston ticks/finalTick retain the entity and carried block when
permission is denied. Retraction preflight uses the same source decision as the
execution path: ordinary pistons and sticky event-2 no-pull cases do not check
untouched blocks two positions ahead.

Player destroy callbacks and explosion loot callbacks are followed by a fresh
permission check and exact snapshot comparison of the block state, live encoded
inventory, saved raw entity, opaque source tags and custom name. If a callback
changes that evidence, subsequent removal/loot delivery aborts rather than
using an obsolete snapshot. A callback's own side effects are not rolled back.
