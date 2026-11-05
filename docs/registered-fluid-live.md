# Registered fluid host integration (draft)

This stage attaches the reviewed catalog, queue and flow engine to authoritative
worlds. It does not yet provide a player-ready fluid: rendering, immersion,
bucket transactions and source-verified mod declarations remain integration gates.
No declaration ships with this patch, so vanilla worlds do not opt in.

The optional `horizon/registered_fluids.json` resource uses format 1 and an explicit
`fluids` array. Each entry requires source, flowing, block and bucket names, a
policy (drop_off, slope_distance, tick_delay, source_conversion), and visual
metadata (still, flowing, tint, emission). There are no guessed flow defaults.
Unknown fields, malformed state families, duplicate bindings, native fluid or
bucket collisions, missing bucket items and invalid visual ranges fail loading.
Texture existence and actual rendering remain pending the mesh adapter.

Each dimension owns a separate registered tick queue sharing its resource catalog.
Chunk ticks persist by names in `horizon:registered_fluid_ticks` within preserved
chunk extras. Pending delays start when the chunk becomes tickable. Block writes
use the normal authoritative behavior path and protection hooks; neighboring
changes wake registered fluids. The bounded first adapter accepts air or the same
registered family only, with native collision-face checks. It does not wash away
plants, waterlog blocks, mix families or implement reactions.

Snapshot errors propagate before dirty flags are cleared or unloaded chunks are
lost. Active batches reject saving rather than silently dropping collected ticks.
A mid-batch invariant failure is fail-closed and needs explicit recovery before
saving can resume. Invalid/unknown loaded tick data is retained, not overwritten.

Validation: 349 horizon-world library tests passed, 2 ignored; the full
horizon-client library passed cargo check with the existing Linux sysroot headers.
Resource tests cover ownership, strict declarations and native collisions. There
are no prepared game resources in this workspace, so no real-game/server-thread
fixture has run. Runtime lifecycle, save failure recovery, rendering, immersion
and inventory synchronization still require independent review and integration.
