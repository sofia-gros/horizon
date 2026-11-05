# Registered fluid mesh adapter (draft)

The asset owner now shares one registered-fluid catalog with gameplay. Meshing
resolves source/flow block levels through that same catalog and uses the existing
fluid surface geometry with explicit still/flow sprites, tint and emissive tier.
Missing sprites fail mesh-table construction instead of substituting lava.

Custom samples keep catalog-scoped family identity. Adjacent custom families and
vanilla water/lava do not erase each other's surfaces or contribute currents to
one another. The shared current-vector algorithm preserves native f32/f64
rounding and falling-wall deflection; native callers retain their existing API.

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
bucket/immersion integration and independent review.
