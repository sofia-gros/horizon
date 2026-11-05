# Third-party notices

## wgpu-hal

`third_party/wgpu-hal` is [wgpu-hal](https://github.com/gfx-rs/wgpu) 30.0.1 by the gfx-rs
developers, with upstream fixes applied
([gfx-rs/wgpu#10124](https://github.com/gfx-rs/wgpu/pull/10124),
[gfx-rs/wgpu#10714](https://github.com/gfx-rs/wgpu/pull/10714)). It is licensed under MIT or
Apache-2.0; both licenses are in that folder (`LICENSE.MIT`, `LICENSE.APACHE`).

## naga

`third_party/naga` is [naga](https://github.com/gfx-rs/wgpu) 30.0.1 by the gfx-rs developers, with
fixes to its GLSL frontend that shader packs need, each applied in the commit that needed it. It is licensed under MIT or Apache-2.0; both licenses
are in that folder (`LICENSE.MIT`, `LICENSE.APACHE`).

## libmimalloc-sys and mimalloc

`third_party/libmimalloc-sys` is [libmimalloc-sys](https://github.com/purpleprotocol/mimalloc_rust)
0.1.49 by Octavian Oncescu, with the [mimalloc](https://github.com/microsoft/mimalloc) v3 sources it
bundles (Copyright (c) 2018-2025 Microsoft Corporation, Daan Leijen), both under the MIT license
(`LICENSE.txt` and `c_src/mimalloc/v3/LICENSE` in that folder). It holds only the v3 sources. One
change: `mi_popcount` in `include/mimalloc/bits.h` uses mimalloc's portable bit count on MSVC
x86/x64 instead of the POPCNT instruction, which CPUs before Nehalem lack (#7264).

## Pixelify Sans

`crates/horizon-theme/fonts/PixelifySans-*.ttf` is [Pixelify Sans](https://github.com/eifetx/Pixelify-Sans)
by The Pixelify Sans Project Authors, licensed under the SIL Open Font License 1.1
(`crates/horizon-theme/fonts/OFL-PixelifySans.txt`).

## Pixelarticons

`crates/horizon-theme/icons/pixel/*.svg` are icons from
[Pixelarticons](https://github.com/halfmage/pixelarticons) by Gerrit Halfmann, under the MIT
license (`crates/horizon-theme/icons/pixel/LICENSE-pixelarticons.txt`). `star.svg` is Horizon's own, drawn on
the same grid.

## Hugeicons

`crates/horizon-theme/icons/glass/*.svg` are icons from the free set of
[Hugeicons](https://github.com/hugeicons/hugeicons) (`@hugeicons/core-free-icons` 4.4.0), under
the MIT license (`crates/horizon-theme/icons/glass/LICENSE-hugeicons.txt`), exported with a 1.6
stroke in `currentColor`.

`crates/horizon-theme/icons/glass/discord.svg` is Discord's logo as drawn by
[Simple Icons](https://github.com/simple-icons/simple-icons) 13.21.0 (CC0 1.0). The logo is Discord's
trademark, used only on the button that opens Horizon's Discord server.

## AMD FidelityFX SDK (FSR 3.1 frame generation)

`crates/horizon-client-render/src/gfx/frame_gen/optical_flow.wgsl` and `interpolation.wgsl` are
ports to WGSL of the optical flow and frame interpolation shaders of the
[AMD FidelityFX SDK](https://github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK) 1.1.4
(`sdk/include/FidelityFX/gpu/{opticalflow,frameinterpolation}`, at commit
[c6efa6bf7f20](https://github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK/tree/c6efa6bf7f2027b3ec94f28578bb5965eabb9e55)),
and `optical_flow.rs` follows the SDK's `sdk/src/components/opticalflow/ffx_opticalflow.cpp`
(its passes, their order and their dispatch sizes). Copyright (C) 2024 Advanced Micro Devices,
Inc., under the MIT license, whose text heads each file. AMD and FidelityFX are AMD's
trademarks; Horizon is not affiliated with AMD.

## Mod loader logos

The launcher shows the logo of the mod loader an instance plays with, and offers the logos as
instance icons.

- `launcher/assets/logos/{fabric,quilt,neoforge,forge}.png` are the loaders' logos as the launcher
  mockup on [playhorizon.dev/launcher](https://playhorizon.dev/launcher) shows them (horizon-site
  `public/media/launcher/*.webp`, converted to PNG), each its project's own mark:
  [Fabric](https://fabricmc.net/), [Quilt](https://quiltmc.org/) (its art is
  [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/)),
  [NeoForged](https://neoforged.net/) (its art, by Ridanisaurus, is
  [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)) and
  [Minecraft Forge](https://minecraftforge.net/). They name the loader an instance runs.

Each logo belongs to its project. Showing it says which loader an instance runs; it does not mean
the project endorses Horizon.

## Sign in with Microsoft

The launcher's sign-in buttons follow Microsoft's published branding guidelines for apps that sign in
with Microsoft accounts
([Sign in with Microsoft branding](https://learn.microsoft.com/en-us/entra/identity-platform/howto-add-branding-in-apps)).
The four-square Microsoft symbol in `launcher/src/microsoft.rs` reproduces the squares, positions and
colours of Microsoft's official `ms-symbollockup_mssymbol_19.svg` from that page, unmodified. Microsoft
and its logo are trademarks of the Microsoft group of companies; Horizon is not endorsed by Microsoft.

## CIT Resewn

`crates/horizon-optifine/src/cit.rs` and `crates/horizon-client-render/src/cit.rs` follow the
condition semantics of [CIT Resewn Continuation](https://github.com/EthanVisagie/CIT-Resewn-Continuation-1.21.11)
(at `b792f9a`), a continuation of [CIT Resewn](https://github.com/SHsuperCM/CITResewn) by SHsuperCM: the
1.21 component paths, enchantment levels, percent damage, text matching and rule ordering. It is
licensed under the MIT license:

> Copyright (c) 2021 SHsuperCM
>
> Permission is hereby granted, free of charge, to any person obtaining a copy of this software
> and associated documentation files (the "Software"), to deal in the Software without
> restriction, including without limitation the rights to use, copy, modify, merge, publish,
> distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the
> Software is furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in all copies or
> substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING
> BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
> NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
> DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
> OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## FusionExamples

The tile tables in `crates/horizon-optifine/src/fusion/layout.rs` were read off the connecting
texture templates (`templates/template <layout>.png`) of
[FusionExamples](https://github.com/SuperMartijn642/FusionExamples) (at `8c9c56e`), which is
licensed under the MIT license:

> Copyright (c) 2024 Martijn van den Brand
>
> Permission is hereby granted, free of charge, to any person obtaining a copy of this software
> and associated documentation files (the "Software"), to deal in the Software without
> restriction, including without limitation the rights to use, copy, modify, merge, publish,
> distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the
> Software is furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in all copies or
> substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING
> BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
> NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
> DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
> OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## BlueMap

`crates/horizon-client-render/src/bluemap.rs` reads the lowres tiles of a server's
[BlueMap](https://github.com/BlueMap-Minecraft/BlueMap) web map. It follows three of BlueMap's files:
`core/src/main/java/de/bluecolored/bluemap/core/map/lowres/LowresLayer.java` (`saveTile`) for the
tiles' pixel layout, `common/webapp/src/js/util/Utils.js` (`pathFromCoords`) for their paths, and
`core/src/main/java/de/bluecolored/bluemap/core/map/MapSettingsSerializer.java` for a map's
`settings.json`, at commit
[96ae320d1d39](https://github.com/BlueMap-Minecraft/BlueMap/tree/96ae320d1d39581c10058d8222f2290f2fcb5333).
BlueMap is licensed under the MIT license:

> Copyright (c) Blue (Lukas Rieger) <https://bluecolored.de>
> Copyright (c) contributors
>
> Permission is hereby granted, free of charge, to any person obtaining a copy
> of this software and associated documentation files (the "Software"), to deal
> in the Software without restriction, including without limitation the rights
> to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
> copies of the Software, and to permit persons to whom the Software is
> furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in
> all copies or substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
> IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
> FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
> AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
> LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
> OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
> THE SOFTWARE.

## SlimeKnights registered-fluid behavior

`crates/horizon-client-render/src/fog/texture_color.rs` adapts
[`TextureColorHelper.computeAverageColor`](https://github.com/SlimeKnights/Mantle/blob/17c287ad52f7b10154e07fa7570dc7bccd74ca2b/src/main/java/slimeknights/mantle/client/model/TextureColorHelper.java),
Copyright (c) 2022 SlimeKnights. That method's explicit Apache-2.0 license is
preserved in `licenses/mantle-texture-color-Apache-2.0.txt`, in addition to Mantle's
repository MIT notice below. No separate upstream NOTICE file was present.
Changes: Rust RGBA slices, bounded dimensions, RGB-only result and a pack-local
cache before atlas resampling. Physical frame zero, thresholds, x-major f32
accumulation and the source alpha/red/green saturation inputs are preserved.

`crates/horizon-client-render/src/fog.rs` adapts the color multiplication and
minimum start/end overrides of Mantle `ClientTextureFluidType` at
`17c287ad52f7b10154e07fa7570dc7bccd74ca2b` (MIT, copyright below).
`registered_fluids/resources.rs` adds opt-in fog declarations and original
loaded-only camera sampling. Changes: the existing host spherical environmental
channel, sky/cloud caps, and native callbacks last. Camera overlays remain outside this slice. The source
`FluidTexture` data model and TConstruct `FluidTextureProvider` were also read.

`crates/horizon-client/src/game/dispenser.rs` follows the registered bucket
behavior in `TinkerFluids.commonSetup` at the TConstruct revision below: target
the facing cell, return an empty bucket after placement, and use the nested
default dispenser fallback on failure. Changes: data-driven family registration,
the bounded host source transaction, host stacked-container remainder insertion,
and inventory commit before deferred scheduling errors. Existing host empty-bucket
pickup and native override behavior are reused. No fluid-specific sound or
extra-content behavior is inferred.

`crates/horizon-world/src/fall_bounce.rs` adapts the landing thresholds,
suppressed damage multiplier and server/client rebound arithmetic from
`ModifierEvents.bounceOnFall` in the same TConstruct revision below. Changes:
pure opt-in response, finite-input guards and host post-restitution application.
`crates/horizon-world/src/bounce_momentum.rs` adapts the horizontal speed
preservation and stop/end timers of `SlimeBounceHandler` at the same revision.
Changes: explicit per-player state, finite-input and once-per-host-tick guards,
with post-restitution landing retained from the earlier slice. Source angle
tracking now uses the host's Mth.atan2 implementation and emits an optional
caller-selected sound for corrected sharp turns. Landing uses the same event
with source unit volume/pitch. Exact event-phase equivalence remains unverified.


`crates/horizon-client/src/motion/registered_fluids.rs` adapts the family-height
check, ignition, lava-type damage and accepted-hit burn sound/pitch of
`BurningLiquidBlock.entityInside`, and
the matching-family status-effect contact of `MobEffectLiquidBlock.entityInside`, from
[Tinkers' Construct f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2).
Changes: explicit resource policies, catalog-scoped identity, and Horizon player
health/effect dispatch. The sound uses the source f32 pitch draw and player
sound category, through an explicit motion event and authoritative LAN dispatch.
`crates/horizon-client/src/game/registered_fluids.rs`
also follows the source's empty curative-item list by setting an explicit
per-instance opt-out from generic consumable cures. Host merge/persistence
mechanics are original; mixed-source equivalence remains unverified.
No Forge implementation was read or copied for these adapters.

`crates/horizon-world/src/registered_flow.rs` also adapts the upward spread,
feeding-neighbor, ceiling-hole/slope and source-support semantics of
[`Mantle InvertedFluid` at 17c287ad52f7b10154e07fa7570dc7bccd74ca2b](https://github.com/SlimeKnights/Mantle/blob/17c287ad52f7b10154e07fa7570dc7bccd74ca2b/src/main/java/slimeknights/mantle/fluid/InvertedFluid.java).
Changes: one explicit per-family vertical direction inside the existing host
algorithm, with host guards and scheduling. `crates/horizon-world/src/fluid_flow.rs`
adapts the same class's upward current neighbor selection, f32 operation order
and column wall deflection. Native samples retain the old downward behavior.
`registered_fluids/resources.rs` follows
[`ClientInvertedFluidType` at the same revision](https://github.com/SlimeKnights/Mantle/blob/17c287ad52f7b10154e07fa7570dc7bccd74ca2b/src/main/java/slimeknights/mantle/fluid/texture/ClientInvertedFluidType.java)
to select a flowing sprite with the `_inverted` suffix. The type and fluid leave
height/shape geometry unchanged; no ceiling mirroring is added. Mantle carries
the same SlimeKnights copyright and MIT notice:

MIT License

Copyright (c) 2022 SlimeKnights

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.


Registered-fluid camera quads in
`crates/horizon-client-render/src/screen_effects.rs` adapt
[Mantle FluidRenderer.renderCamera at 17c287ad52f7b10154e07fa7570dc7bccd74ca2b](https://github.com/SlimeKnights/Mantle/blob/17c287ad52f7b10154e07fa7570dc7bccd74ca2b/src/main/java/slimeknights/mantle/client/render/FluidRenderer.java).
Camera texture/opacity declarations follow `FluidTexture` and
`ClientTextureFluidType` at that revision. Copyright (c) 2022 SlimeKnights;
the full MIT license immediately above applies. Changes: explicit opt-in
resource declarations, loaded-only eye sampling, shared atlas metadata and the
host's HDR world-tint screen pipeline. Only the source's opaque RGB tint case
is supported; no Forge implementation or game assets were copied.


`crates/horizon-client/src/mobs/registered_fluids.rs` adapts
[Tinkers' Construct BurningLiquidBlock.entityInside](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/block/BurningLiquidBlock.java).
Copyright (c) 2022 SlimeKnights; MIT, full license above. Changes: optional
host resources, bounded loaded-only family-height snapshot, host mob ignition
and lava damage, and categorized local/LAN burn sound dispatch. Fire-immune
mobs are skipped; sound pitch consumes the level random only on accepted hits,
including silent entities before their sound suppression. No Forge implementation
or game assets were read or copied for this adapter.


The standard mob effect-contact adapter in `mobs/registered_fluids.rs` and
`game/registered_fluids.rs` also follows
[Tinkers' Construct MobEffectLiquidBlock.entityInside at f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/block/MobEffectLiquidBlock.java).
Copyright (c) 2022 SlimeKnights, MIT; full license above. Changes: explicit
per-family policies, deferred authoritative mob application, current host
entity-tag immunity checks, and host active-instance curative metadata. Only
Poison, Slowness and Blindness are enabled here. No Forge implementation was
read or copied. Hidden effect-chain and exact event-order parity are unverified.


The mob extinguishing adapter consumes the explicit `canExtinguish(true)`
policy declared by MIT Tinkers' Construct `TinkerFluids.cool()` at
[`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/TinkerFluids.java).
Copyright (c) 2022 SlimeKnights, MIT; full notice above. The host adapter uses
its existing mob water/rain fire-clearing phase and sound handling, with
registered family geometry. Forge's generic-fluid implementation was not read
or copied; exact source event-phase equivalence is not claimed.

The mob current adapter in `mobs/living.rs` consumes the explicit motion-scale
policy declared by MIT Tinkers' Construct `TinkerFluids.cool()` and `hot()` at
[`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/TinkerFluids.java).
Copyright (c) 2022 SlimeKnights; full MIT notice above. Changes: shared existing
loaded-only player flow sampling and Horizon's native non-player current target
and species push predicate. No Forge implementation was read or copied; exact
source tick-phase and mixed-fluid order equivalence are not claimed.

The optional registered-fluid path-node declaration and its mob path adapter in
`registered_fluids/resources.rs` and `mobs/path.rs` follow the explicit
`pathType(LAVA).adjacentPathType(null)` configuration in Tinkers' Construct
`TinkerFluids.hot()` at
[`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/TinkerFluids.java),
Copyright (c) 2022 SlimeKnights, MIT (license reproduced above). The Rust adapter
is original host integration. No Forge implementation was read or copied.

The dropped-item registered-fluid adapter in
`crates/horizon-client/src/entities/registered_fluids.rs` adapts
Tinkers' Construct `BurningLiquidBlock.entityInside` at
[`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/block/BurningLiquidBlock.java),
Copyright (c) 2022 SlimeKnights, MIT (license reproduced above). It retains
positive family-height gating, ignition before lava damage, and the accepted-hit
sound. The Rust item/world integration is original; no Forge implementation was
read or copied.

Dropped-item extinguishing in `entities/registered_fluids.rs` consumes the
explicit `canExtinguish(true)` policy in MIT Tinkers' Construct
`TinkerFluids.cool()` at
[`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/TinkerFluids.java),
Copyright (c) 2022 SlimeKnights (MIT license above). The host phase integration
is original Rust code; no Forge implementation was read or copied.

Registered dropped-item current forces in `entities/fluids.rs` and
`entities/registered_fluids.rs` consume the explicit motion-scale policy in
MIT Tinkers' Construct `TinkerFluids.cool()` and `hot()` at
[`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/TinkerFluids.java),
Copyright (c) 2022 SlimeKnights (MIT license above). This is original wiring to
the host's existing current sampler and non-player current target; no Forge
implementation was read or copied.

Registered armor-stand currents share the original Rust entity adapter above
and the same MIT Tinkers' Construct `TinkerFluids.cool()` / `hot()` motion-scale
policy (Copyright (c) 2022 SlimeKnights, revision
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`). No additional external implementation
was read or copied for the armor-stand wiring.

Registered armor-stand extinguishing in `entities/registered_fluids.rs` consumes
that same MIT Tinkers' Construct `TinkerFluids.cool()` canExtinguish policy
(Copyright (c) 2022 SlimeKnights, revision
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, full MIT license above). This is
original Rust integration with the existing native-water fire-clearing phase;
no additional external implementation was read or copied for this wiring.

Armor-stand fall-distance scaling reuses the registered interaction contract and
original host entity geometry/current adapters above. It adds no external source
adaptation; the existing TConstruct policy attribution remains unchanged.

Armor-stand registered-fluid travel is original Rust wiring of the existing
registered travel contract and native host stand movement/falling kernels.
No additional external source was read or copied; existing policy attribution
above remains unchanged.

Armor-stand registered-fluid ignition in `entities/registered_fluids.rs` adapts
[Tinkers' Construct BurningLiquidBlock.entityInside at f4e77d43f39ba1047bf93ae816b7e1beaab514c2](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/block/BurningLiquidBlock.java),
Copyright (c) 2022 SlimeKnights, MIT (full license above). Changes: bounded
registered-family body sampling, the host stand block-contact exclusions, and
its existing lava-damage rejection. Ignition precedes that rejection, so no
accepted-hit sound is produced. No Forge implementation was read or copied.

Registered-fluid mob drowning immunity follows MIT Tinkers' Construct
[SlimeFluidType.canDrownIn](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/fluids/SlimeFluidType.java)
and its TinkerTags.EntityTypes.SLIMES / generated forge:slimes tag at revision
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, Copyright (c) 2022 SlimeKnights, MIT
(full license above). Changes: explicit optional declaration and the host's
loaded entity-type tags/common-air adapter. No Forge implementation was read.

Experience-orb registered currents reuse the original host current adapter and
MIT Tinkers' Construct motion-scale policy above (Copyright (c) 2022 SlimeKnights,
revision `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`). The orb wiring is original
Rust; no additional external implementation was read or copied.

Primed-TNT registered currents reuse the original host current adapter and MIT
Tinkers' Construct motion-scale policy above (Copyright (c) 2022 SlimeKnights,
revision `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`). The TNT wiring and public
adapter export are original Rust; no additional external implementation was
read or copied.
