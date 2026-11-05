# Registered-fluid occupied path nodes

Part of sofia-gros/horizon-mods#379; stacked on host PR #50.

A registered-fluid declaration can opt in with `"path_type": "lava"` at the
same level as `source`, `policy`, `visual`, and `interaction`. Omission or null
preserves the block's existing path type. Unknown strings and malformed values
fail resource loading. No water or other path-type policies are enabled here.

`MobWorld::path_type_from_state` resolves source and all flowing levels through
the registered family, then returns the existing `PathType::Lava`. Walking,
flying and amphibious node evaluators therefore use their ordinary lava-node
costs and species-specific maluses. Ordinary mobs reject the cell with the
existing negative lava malus; a species with an explicit lava malus retains
that setting. This is a block-cell classification, not an immersion threshold.

The neighboring-cell scan intentionally retains native block path facts. The
registered lava node does not become a synthetic `FireInNeighbor` hazard.
Native lava, fire, damaging blocks and water still have their previous neighbor
behavior. Existing support-cell and flying-node transformations remain intact.

The source is MIT Tinkers' Construct `TinkerFluids.hot()` at
[f4e77d43f39ba1047bf93ae816b7e1beaab514c2](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/TinkerFluids.java):
`pathType(LAVA).adjacentPathType(null)`. This is original host integration of
those declarations, not a port of Forge's path evaluator implementation.

The paired content change opts in all 59 `hot(...)` families, including
inverted Cinderslime and the non-burning Liquid Soul and Molten Ender. Magma
and the other 10 cool/slime families remain unconfigured. Nothing infers path
policy from burning, swimming, temperature, name, or flow direction.

No fluid tags, native water/lava identity, swim evaluator, collision geometry,
boat behavior, or saved/network data change. This does not implement aquatic
navigation in cool registered fluids or claim full Forge navigation parity.

A resource regression covers omitted policies, all 16 source/flowing block
levels, unknown blocks, independence from interaction policies, and malformed
values. Tests, builds, native injection planning, CI, version checks and merge
are deferred at the user's request. Only formatting, diff checks and brief
manual review were performed.
