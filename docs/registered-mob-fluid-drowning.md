# Registered-fluid common mob drowning (#379)

This opt-in host slice connects `interaction.can_drown` eye immersion to the
existing common mob air-supply path. It stacks on the mob jump/exit slice.
Body contact alone does not remove air. Source and flowing states share the
family policy; another family's body or eye contact cannot enable it. Missing,
invalid or oversized observations do not invent immersion. No native fluid tag,
water flag, saved field, content coefficient or protocol format is changed.

The current eleven enabled content families are beetroot soup, earth slime,
ender slime, honey, ichor, magma, meat soup, mushroom stew, rabbit stew, sky slime
and venom. Other declarations retain their explicit non-drowning policy; burn
or extinguish policy does not imply drowning. In particular, liquid soul and
molten ender are not added to the drowning set.

Common mobs use the existing breath/drown hooks and default consumption of one
air per tick. At the existing -20 threshold, air resets to zero, the ordinary
Drown event is emitted and ordinary 2-point drowning damage uses the existing
mob damage path. Dry breathing refills four air up to 300. Hooks can still
modify/cancel the result; invulnerability still rejects damage normally.

The shared predicate now respects Water Breathing and Breath of the Nautilus.
Water Breathing permits ordinary refill; the nautilus effect holds an air-needing
submerged mob's air steady, and leaving immersion resumes refill. Inherent
water-breathing mobs continue to breathe. This also fixes the previous common
native-water path's omission of those two effects, even without registered
fluids; that is an intentional shared behavior change. Existing axolotl and
`needs_water` species-specific air/suffocation handling is left untouched.

This is original wiring of Horizon's existing declarations, contact geometry,
player breathing-effect semantics and common mob air hooks. No external
implementation or game assets were read/copied. It does not claim exact
Tinkers'/Forge entity-type, oxygen-bonus or event-phase equivalence. The mob's
existing one-air consumption policy remains, without new respiration/oxygen
randomness or aquatic hydration semantics.

Brief static review, rustfmt and whitespace inspection only. Four new synthetic
regressions cover body-only versus matching eye immersion, source/flowing and
non-drowning families, missing cells, air loss/damage/refill, Water Breathing,
nautilus air holding, inherent breathing, invulnerability and maximum air.
They are **not run**. No tests, builds, CI inspection, version checks or merge.
Live damage/death, effect expiry, hook integration and network bubble behavior
remain for the final integration pass.

Remaining work includes fall-distance policy, swimming/float AI activation,
automatic ridden custom-fluid buoyancy, passenger boat shielding, species-
specific fluid behavior and custom mob effects. This does not finish #379.

Mob fall-distance policy is covered by the later
`registered-mob-fluid-fall.md` slice, with refresh-order limits documented there.
