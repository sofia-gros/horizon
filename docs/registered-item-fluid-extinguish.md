# Registered-fluid extinguishing for dropped items

Part of sofia-gros/horizon-mods#379; stacked on host PR #52.

A burning dropped item now clears its fire timer when it has positive body
contact with a registered family whose `interaction.extinguishes` is true.
The existing 11 declarations are Earth Slime, Sky Slime, Ender Slime, Ichor,
Magma, Venom, Honey, Beetroot Soup, Mushroom Stew, Rabbit Stew and Meat Soup.
Liquid Soul and all other undeclared families do not gain this behavior.

The opt-in check runs in the existing item environment tick after native
water/fire-immunity clearing and before periodic on-fire damage. It does not
heal the item, change its stack, metadata, age, ownership, target or pickup
state, or add any water/lava identity. Native water/fire immunity short-circuits
as before. Unlit items do not sample registered blocks.

The shared burn/extinguish sampler uses the deflated final item box, finite
coordinates, the registered catalog's 8192-read budget, and loaded/in-height
block queries. Positive height must belong to the explicitly extinguishing
family. Missing cells, invalid positions and contact above the liquid surface
cannot put out the item. No chunks are loaded by the query.

Extinguishing is not damage immunity. Later native fire contact or a declared
registered burn can ignite again in the same environment tick. Magma notably
inherits `canExtinguish(true)` from the source slime/cool helper while also using
a burning block; both independent policies remain active. The Worldbound
cancellable entry point is unchanged and still precedes all of this work.
Only the authoritative game supplies registered resources, as in #52.

The source declaration is MIT Tinkers' Construct `TinkerFluids.cool()` at
[f4e77d43f39ba1047bf93ae816b7e1beaab514c2](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/TinkerFluids.java).
This is original host integration of the existing content policy, not a port of
Forge's entity-fluid implementation. Like native item water clearing here,
this adds no extinguish sound. Item currents/buoyancy and swept contacts remain
separate work; no fields or save/network formats are introduced.

Three unrun regressions cover timer clearing without healing/metadata changes,
source/flowing surface thresholds and opt-out/missing/invalid cells, mixed-fluid
reignition and a single-family extinguish+burn policy. Existing burn tests remain.
Brief manual review, rustfmt and whitespace checks were performed. Tests,
builds, native injection planning, CI inspection, version checks and merge were
not performed as requested.
