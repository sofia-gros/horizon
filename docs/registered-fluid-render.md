# Registered fluid mesh adapter (draft)

The asset owner now shares one registered-fluid catalog with gameplay. Meshing
resolves source/flow block levels through that same catalog and uses the existing
fluid surface geometry with explicit still/flow sprites, tint and emissive tier.
Missing sprites fail mesh-table construction instead of substituting lava.

Custom samples keep catalog-scoped family identity. Adjacent custom families and
vanilla water/lava do not erase each other's surfaces or contribute currents to
one another. The shared current-vector algorithm preserves native f32/f64
rounding and falling-wall deflection; native callers retain their existing API.

For `vertical_direction: "up"`, world flowing sprites use the declared path
plus `_inverted`, matching Mantle `ClientInvertedFluidType` at
`17c287ad52f7b10154e07fa7570dc7bccd74ca2b`. Still sprites stay unchanged, and missing
inverted artwork is an error rather than a downward-texture fallback. The mesh's
flow-rotated UVs use the upward family's current sampling. Mantle changes the
flowing texture, not the height/shape implementation: surfaces stay bottom-based
and are not mirrored to the ceiling. Existing corner interpolation, occlusion,
winding, light and body/eye sampling therefore stay unchanged.

This first custom mesh mode is translucent, has no water overlay and uses the
resource's fixed RGB tint. Opaque fluids, animated tint, particle/immersion
rendering and fluid-specific effects are not provided. Actual world light
propagation still comes from the block facts; the visual emission field controls
only the mesh's emissive tier. Resource authors must supply matching block facts.

Validation: all 350 world library tests passed (2 ignored), including distinct
custom-family currents and falling-wall behavior. Full client library cargo check
passed. Renderer test targets are type-checked; graphics tests are not linked or
executed in this stage because of the shared disk budget. No real-game visual
fixture has run. Keep this stacked branch draft and unmerged pending complete
bucket/immersion integration and independent review. Those results predate the
upward-current/texture extension. That extension received only brief static
review, formatting and whitespace review; its new regressions were not run.
No tests, builds, CI inspection, version checks or merges were performed for it.
