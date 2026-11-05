# Registered-fluid mob burn contact (draft)

Optional registered-fluid resources now reach the authoritative mob world.
The normal mob block-contact pass samples the current deflated bounding box
once, with a maximum of 8192 reads, then checks each visited registered block
against the positive height of that exact source family. Same-family columns,
flowing levels and upward-fluid geometry use the existing contact sampler.
Missing chunks, out-of-height cells, invalid or oversized queries cannot cause
a burn. No native water/lava identity or movement flags are assigned.

An explicit `interaction.burn` policy ignites for its declared seconds through
existing mob burning-time/protection handling and calls the normal lava damage
path. Fire-immune mobs skip the adapter. Armor, invulnerability, damage hooks,
hurt/death events and normal death/drop handling remain in that damage path.

One prerequisite defect is also fixed in the shared mob damage path: active
Fire Resistance now rejects fire damage before the native incoming-damage hook,
matching the existing `horizon_world::living_hurt::hurt_server` guard. This is
needed for both the fluid lava hit and its subsequent periodic OnFire damage.
It applies to ordinary mob fire damage too, even without registered fluids;
other ordinary mob behavior is unchanged. Ignition still happens before damage
rejection, as in the source block.

Accepted hits draw `2.0f32 + level_random.next_float() * 0.4f32` and emit the
additional generic burn sound at volume 0.4 at the mob's feet. Rejected hits
consume no pitch random. Silent mobs consume the accepted-hit pitch but emit
no sound. A dedicated event keeps hostile/neutral mob sound categories rather
than inferring the category from the generic sound ID. The existing native
sound queue sends the same event to nearby LAN players without duplicate generic
audio capture. The ordinary hurt sound remains separate.

Source: MIT Tinkers' Construct
[`BurningLiquidBlock.entityInside` at f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/block/BurningLiquidBlock.java).
Its project license was read at that revision. No Forge implementation or game
assets were copied. Existing content policies provide all burn values; no new
content declaration, version pin, saved field or protocol format is required.

Brief static review, rustfmt and whitespace inspection only. Synthetic
regressions cover matching/nonmatching families, source/flowing blocks, shallow
surfaces, missing samples, immunity, invulnerability, Fire Resistance, silence,
and accepted/rejected RNG consumption. They are **not run**. No tests, builds,
CI inspection, version checks or merge.

Exact source event-order equivalence remains unverified: this implementation
uses a current-box snapshot during the host's existing swept block-contact pass.
Passenger shielding, custom mob currents/travel, drowning, custom status
effects, separately simulated dragon parts, dropped items/projectiles and mixing
remain separate work. Live damage/death, network audio and save/reload behavior
still require the final integration pass. This does not complete #379.

Standard Poison/Slowness/Blindness contacts are covered by the later
`registered-mob-fluid-effects.md` slice.

Registered cool-fluid mob extinguishing is covered by the later
`registered-mob-fluid-extinguish.md` slice.

Registered-fluid mob current forces are covered by the later
`registered-mob-fluid-currents.md` slice; custom travel and drowning remain open.
