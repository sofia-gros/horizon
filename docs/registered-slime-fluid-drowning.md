# Registered-fluid mob drowning immunity tags

Part of sofia-gros/horizon-mods#379, stacked on the registered-fluid host drafts.

A declaration may set top-level `drowning_immune_tag` to a namespaced entity-type
tag, for example `forge:slimes`. Omitted/null keeps the existing behavior. Invalid
IDs or non-string values fail resource loading. Source and flowing identities
resolve to the same tag. The tag alone grants no drowning, travel or other policy.

The common-mob eye-drowning adapter still requires eye immersion and
`interaction.can_drown`. It now excludes a contact if the current mob's actual
entity-type ID belongs to that family's declared tag in the loaded MobData tags.
There is no species/name heuristic and an absent tag grants no immunity. Another
non-exempt drowning family can still consume air. Existing native-water air
logic, Water Breathing, inherent breathing, and special aquatic paths are unchanged.
Immunity therefore applies only to the declaring registered family, not globally.

## Source and content

MIT Tinkers' Construct `SlimeFluidType.canDrownIn` at
[f4e77d43f39ba1047bf93ae816b7e1beaab514c2](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/fluids/SlimeFluidType.java)
excludes `TinkerTags.EntityTypes.SLIMES`. Its common tag is `forge:slimes`, not
`tconstruct:slimes`. Source members are vanilla slime/magma cube and TConstruct
sky slime, ender slime and terracube. `TinkerFluids` plus
`FluidDeferredRegisterExtension.registerSlime` assign this type to Earth/Sky/Ender
slime, Magma and Ichor. Cinderslime and other cool fluids are not included.

Copyright (c) 2022 SlimeKnights, MIT; the source files and pinned LICENSE were
read. This is an explicit resource-policy adaptation and original host tag/air
wiring. No Forge implementation was read or copied. A matching content draft
adds those five declarations and the source tag. This host patch alone leaves
all existing declarations unchanged.

## Limits and deferred validation

Only common mobs consume this optional tag here. Arbitrary data-pack tagging of
players, custom stand effect/air handling, item buoyancy and exact source event
phase parity are outside scope. Source-registration of custom slime entities
remains separate; retaining their tag names does not register them.

Brief static review, rustfmt and `git diff --check` only. Three unit tests were
added for schema/identity validation, per-family membership/eye gates and
common-air refill without damage using loaded tag data. Tests/builds/injection
planning/CI/version checks and merge are deferred to separate batch integration.
