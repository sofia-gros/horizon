# Registered-fluid camera fog (draft)

A declaration may now include `visual.fog`, with finite `start` and `end`
distances and an optional 24-bit RGB `color`. The range is spherical, with a
positive end greater than start and limits inside -65536 to 65536. Absence is
backward compatible and adds no camera behavior. Unsupported fields are rejected.

The render frame samples the actual camera point, not player contact state.
It reads at most the camera cell and the cell above through a loaded-only
provider. A same-family cell above fills the column; otherwise the registered
fluid's own height is used. The surface boundary is outside. Upward fluids keep
the existing bottom-anchored geometry. Invalid coordinates and missing chunks
produce no override. This does not load or modify chunks.

The policy follows MIT Mantle `ClientTextureFluidType`: color multiplies the
existing fog RGB, and start/end only lower existing limits. Horizon applies the
range to its existing spherical environmental channel and caps sky/cloud fog;
its cylindrical render-distance channel is unchanged. The ordinary Fog-off
option runs first, so it cannot erase the custom fluid fog. Native mod color and
render hooks run last. The camera retains its native `FogType` rather than
aliasing a registered liquid to water/lava; water vision, immersion FOV and
native fluid physics do not change.

Source references:

- [Mantle ClientTextureFluidType](https://github.com/SlimeKnights/Mantle/blob/17c287ad52f7b10154e07fa7570dc7bccd74ca2b/src/main/java/slimeknights/mantle/fluid/texture/ClientTextureFluidType.java)
- [Mantle FluidTexture](https://github.com/SlimeKnights/Mantle/blob/17c287ad52f7b10154e07fa7570dc7bccd74ca2b/src/main/java/slimeknights/mantle/fluid/texture/FluidTexture.java)
- [TConstruct FluidTextureProvider](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/data/FluidTextureProvider.java)

The companion content supplies all seventy source ranges: five families at
-8/24, four at 0.25/4, Glass at 0.25/8, and sixty at 0.25/2. The twelve non-white
source tints and explicit Queen's Slime color supply thirteen multipliers.
The other fifty-seven source colors require runtime still-sprite averaging;
that path is deliberately deferred, not replaced with a guessed color. Camera
overlay sprites are also outside this slice. No texture files or game assets
are copied by the host change.

Only brief static review, rustfmt and whitespace inspection were performed.
Added regressions for optional declarations, bad data, camera immersion,
loaded-only sampling, tint multiplication and tighter existing limits are
**not run**. No tests, build, CI inspection, version check or merge was performed.
Live graphics, third-person transitions, resource reloads, shader packs and
exact source event-order equivalence remain unverified. In particular, the
modern host's two-channel fog is an explicit integration, not a claim of
bit-identical source-version fog rendering.
