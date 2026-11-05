# Registered-fluid camera fog (draft)

A declaration may now include `visual.fog`, with finite `start` and `end`
distances, an optional 24-bit RGB `color`, and optional `calculate_color`
(default false). The range is spherical, with a
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
The other fifty-seven families now opt into runtime still-sprite color
calculation. Standalone camera overlays are described in `registered-fluid-camera.md`. No
texture files or game assets are copied by these host changes.

Only brief static review, rustfmt and whitespace inspection were performed.
Added regressions for optional declarations, bad data, camera immersion,
loaded-only sampling, tint multiplication and tighter existing limits are
**not run**. No tests, build, CI inspection, version check or merge was performed.
Live graphics, third-person transitions, resource reloads, shader packs and
exact source event-order equivalence remain unverified. In particular, the
modern host's two-channel fog is an explicit integration, not a claim of
bit-identical source-version fog rendering.

## Automatic sprite colors

`calculate_color: true` resolves a missing explicit color from the loaded still
sprite. An explicit color always wins. The client computes one value per texture
while loading `RenderAssets`, before atlas resizing, and stores it with that
resource pack. Reload replaces the cache; frame rendering only looks up a value.
A server builds no automatic color cache. Declared still and normal/inverted
flowing sprites are now explicitly inserted by the resource loader, rather than
assuming procedural liquid textures appear on ordinary model faces.

The calculation adapts Mantle `TextureColorHelper.computeAverageColor` at
`17c287ad52f7b10154e07fa7570dc7bccd74ca2b`. That method explicitly carries the
Apache-2.0 license, included in `licenses/mantle-texture-color-Apache-2.0.txt`.
It samples physical frame zero at native resolution, independent of the
animation's first displayed frame, and uses x-major f32 accumulation. Pixels
must have alpha greater than 127 and an RGB maximum greater than 31. The source
passes **alpha, red, green** into RGBtoHSB for the saturation weight; this odd
argument order is deliberately preserved, not corrected. It truncates averaged
RGB channels, and returns black when no positive weight remains. In particular,
opaque white and opaque yellow have zero source weight.

Missing/corrupt sprites do not turn their fallback checkerboard into fog color;
no calculated override is applied. The color cache remains outside the immutable
simulation registry. It is not saved or synchronized. New tests for source
weights, thresholds, physical frame selection, missing sprites, explicit-color
priority and changing cache results are written but not run. Actual pack reload,
world rendering, animated resource packs and shader compatibility still await
the final integration pass. Source-version texture API equivalence is unverified;
no Forge implementation was read to fill that gap.
