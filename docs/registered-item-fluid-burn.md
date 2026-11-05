# Registered-fluid burn contact for dropped items

Part of sofia-gros/horizon-mods#379; stacked on host PR #51.

The authoritative dropped-item environment tick now consumes declared burn
policies. This covers the content stack's existing 58 burning families without
adding native fluid tags or changing their declarations. Registration alone,
non-burning fluids and living-only contact effects cannot damage an item.

The fixed item box is deflated by 0.001 and sampled through the registered
catalog with finite-coordinate checks, an 8192-read budget and loaded/in-height
world reads. Only positive body height in the exact block's family permits
contact. Eye-only and surface probes are not block-inside callbacks. Each
intersecting body block can apply one hit, stopping when item health is depleted.
This samples the final item box; it does not add swept movement contact parity.

The source is MIT Tinkers' Construct `BurningLiquidBlock.entityInside` at
[f4e77d43f39ba1047bf93ae816b7e1beaab514c2](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/block/BurningLiquidBlock.java).
Configured seconds ignite before lava-source damage. The host's existing
categorical item fire-resistance check prevents both; an explicit lava-only
resistance rejects damage but still allows ignition. Existing Netherite and
component checks remain in use. Accepted damage subtracts the configured float
from integer health, including fractional damage, then casts back to integer.
The source callback can accept zero damage; it still ignites and emits sound.

Accepted hits, including the lethal hit, each emit generic burn at volume 0.4,
with pitch `2 + levelRandom.nextFloat() * 0.4`. Local and LAN/server dispatch use
the neutral category. Registered sounds do not inherit native lava's ten-tick
sound cadence. Native lava's own behavior is unchanged. Items have no separate
silent-entity flag in the existing host model; this slice does not add one.

Work remains inside `ItemEntity::environment_tick`, preserving the existing
Worldbound HEAD cancellation from mods PR #817. That hook's entity/world binding
and return shape are unchanged. The method gains optional registered resources
and a sound-count output; old native-only tests pass `None` and an unused counter.
The game only supplies registered resources on the authoritative side. No new
item fields, serialization, merging, pickup or lifetime behavior are introduced.
Native injection compatibility was reviewed statically, not run.

Four unrun resource-backed helper regressions cover lethal damage and sound
counts, immunity versus damage resistance, source/flowing surface thresholds,
non-burning families, multiple body cells, absent cells and invalid coordinates.
Existing native environment tests are retained. Formatting, whitespace checks
and brief source review were performed. Tests, builds, native injection planning,
CI inspection, version checks and merge were not performed as requested.

Registered-fluid item currents, buoyancy, extinguishing, projectile contacts,
and full source movement/saving/LAN integration validation remain separate work.
