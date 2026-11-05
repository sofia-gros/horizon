# Registered liquid data layer (draft)

This branch begins the host work needed by
[the TConstruct world-fluid task](https://github.com/sofia-gros/horizon-mods/issues/379).
It does not enable custom liquids in a running client or server.

`horizon_world::registered_fluids::RegisteredFluids` builds an immutable catalog
against one borrowed block registry. Each definition names a source fluid, its
flowing partner, and an existing liquid block with all sixteen level states.
Registration rejects duplicate names, shared block bindings, malformed names,
missing levels, and blocks with unrelated properties. It does not invent
physics, textures, interactions, buckets, or TConstruct-specific identities.

Runtime identities are local to one catalog. Passing a state from another
catalog fails, even if its numeric slot happens to match. Saved states use the
full fluid name and validated amount/falling values, so registration order can
change without changing the saved fluid. Missing families fail to resolve
rather than becoming water, lava, or air. Callers must retain unresolved data
and surface the missing dependency instead of discarding it.

Level decoding and canonical block conversion follow the existing vanilla
`FluidState` convention: levels 8 through 15 decode as full falling liquid and
write back as level 8. Source and flowing identities remain distinct.

## Integration gates before enabling or merging

1. Wire the tested named-NBT tick codec into the live scheduler and resource reload;
   never reuse the vanilla enum's discriminants for custom liquids.
2. Extend simulation and block bindings with explicit source-specified flow,
   source-conversion and replacement behavior. Never use the lava fallback.
3. Bind custom sprites, tint and lighting in the mesher and immersion handling.
4. Complete authoritative local/guest bucket placement and source pickup with
   atomic inventory/world updates and persistence tests.
5. Enable one source-verified fluid through the mod resource registration path,
   then test its full placement, flow, pickup and reload cycle. Independent
   review is required before merging; this data-layer draft is not player-ready.

Validation: `cargo test --locked -p horizon-world --lib registered_fluids`.
Tests cover every level, pair/family identity, reordered catalogs and ticks, invalid or
missing saved data, incomplete bindings and malformed definitions.
