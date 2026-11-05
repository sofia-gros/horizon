# Registered-fluid camera overlays (draft)

Optional `visual.camera` declares a standalone namespaced `texture` (without
`textures/` or `.png`) and finite `opacity` in (0, 1]. Texture paths must not
contain empty, `.` or `..` segments. Its tint is the visual's existing 24-bit
RGB. Omission preserves existing behavior and does not require `visual.fog`.
The source's arbitrary tint alpha is not represented; all seventy companion
source fluids have opaque tint alpha and opacity 0.1.

The client includes only declared camera images in its full HUD atlas. Content
mod merging writes the declaration document into the same overlay root before
the atlas is built. Resource reload rebuilds both sprites and sampling flags.
Missing or corrupt images are skipped, never drawn as a missing checkerboard.
Fonts-only server atlases do not load these sprites. Texture metadata blur and
clamp choices use the existing standalone-texture sampler.

In first person, outside spectator mode, the player's actual eye is sampled
through loaded-only cells. Fog and overlay sampling share same-family height
and strict surface-boundary handling, including the existing unmirrored upward
fluid geometry. No chunks are loaded, no fluid identity changes, and no overlay
is drawn above the liquid surface. The overlay uses eye lightmap brightness,
the visual RGB tint, source opacity and four texture repeats shifted by
-yaw/64 and pitch/64. It uses the existing hand-projected, alpha-blended HDR
world-tint quad path before fire/HUD overlays. A registered overlay takes
priority over native water overlay input, avoiding two liquid overlays.

Source: [MIT Mantle FluidRenderer.renderCamera](https://github.com/SlimeKnights/Mantle/blob/17c287ad52f7b10154e07fa7570dc7bccd74ca2b/src/main/java/slimeknights/mantle/client/render/FluidRenderer.java),
`FluidTexture`, and `ClientTextureFluidType` at the same pinned revision.
The companion content uses 61 exact upstream generated static camera textures
for 70 families. Its source generator copies the first physical still frame,
not an animated sprite sheet. No assets or Forge implementation are copied here.

Brief static review, rustfmt and whitespace inspection only. Schema, immersion,
quad UV/color/sampling and native-water priority regressions are written but
**not run**. No tests, build, CI inspection, version checks or merge. Live
rendering, HDR appearance, pack reload and source event gating/order remain
unverified and belong to the final integration pass. Nonplayer interactions,
fluid mixing and broader #379 parity are not completed by this change.
