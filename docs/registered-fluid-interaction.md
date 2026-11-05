# Registered fluid player interaction (draft)

The optional `interaction` declaration binds one explicit policy to the source
and flowing IDs of a family. It supplies travel acceleration, horizontal and
vertical drag, gravity scale, jump impulse, current scale, swim/drowning flags,
extinguishing, fall-distance scaling, optional burn seconds/damage and an optional
sustained vanilla contact effect. Invalid
numbers fail resource load. Missing policy keeps observations inert.

Player base ticks sample loaded cells, preserve independent currents per family,
apply configured push and fall scaling, and respect the boat's sheltered volume.
Native water/lava tags stay unchanged. A configured body contact uses its own
travel path, jump and collision escape; drowning is selected by the eye's family.
Block-inside burning uses that family's positive body-height guard, the declared
burn time and damage through the existing fire/health path. The same motion code
serves predicting and authoritative players.

An `effect` object accepts only `minecraft:poison`, `minecraft:slowness` or
`minecraft:blindness`, `duration_ticks` from 1 through 72000, and an `amplifier`
from 0 through 255. Optional boolean `no_curatives` defaults to false.
Omission keeps earlier declarations unchanged. Unknown
effects and unsupported fields fail resource load rather than silently doing
nothing. Positive body height for the block's own family is required; eye-only
or other-family observations cannot trigger it. Source and flowing blocks share
the same policy.

The movement path emits a request, and the authoritative player dispatcher uses
the existing effect merge/attribute path. Predicting clients do not add effects
themselves. Continued contact refreshes duration and preserves normal stronger
effect/hidden-effect merging. Standard effect sync, persistence and expiration
are reused; full game/LAN behavior has not been validated.

`MobEffectLiquidBlock` supplies an empty curative-item list. A declaration can
now opt into that restriction with `no_curatives: true`. The authoritative
player retains this flag on each effect instance: milk-style generic curing
leaves it active, including after leaving the liquid. Explicit targeted
removals, commands, death and normal expiry still remove effects. This is a
binary opt-out, not support for arbitrary item-specific curative lists.

The flag follows the accepted active duration/amplifier on replacement or
same-strength extension; an ignored shorter application cannot overwrite it.
Hidden instances retain their own flag and restore it when they become active.
Mixed-source curative merge equivalence has not been checked against Forge.
The active instance flag saves as optional `horizon:no_curatives` in the player
effect compound; absent means false, leaving vanilla saves unchanged. Owner
and guest playerdata use the same path. The existing save model still does not
persist hidden effect chains. Standard packets do not carry this private flag;
remote clients wait for authoritative generic-cure removals rather than clear
protected effects speculatively. Live save/reload/LAN validation remains pending.
Mobs and custom TConstruct effects are not handled by this player-only extension.

Source: MIT TConstruct f4e77d43, `TinkerFluids`, `BurningLiquidBlock.entityInside`
and `MobEffectLiquidBlock.entityInside`.
The generic configurable travel integrator uses Horizon's existing movement and
fluid-falling primitives, not Forge implementation code. Matching Forge defaults,
mixed-fluid precedence, and movement sequences still need an independent oracle.

Remaining: mobs/items/projectiles, custom potion effects, item-specific curative
lists and mixed-source merge parity,
source-specific burn audio, camera fog, fluid mixing/solidification, dispenser use, and live save/reload/LAN acceptance. This is not complete
#379 support and must stay draft. The paired new slice declares Venom, Honey and
Liquid Soul; their policy enables Poison, Slowness and Blindness respectively.

Validation: an initial focused registered-fluid unit run passed 19 tests before
final helper changes. Newly added policy/current regressions and the final batch,
full tests, CI review and merge remain deferred. Rust formatting and whitespace
were checked. `cargo check -p horizon-client-lib --lib --no-default-features --offline`
passed for the earlier runtime changes. The contact-effect extension has only
received brief static review and formatting; its new resource/contact cases are
deferred. No tests, builds, CI inspection, version checks or merges were run for
that extension. Do not treat earlier checks as its validation result.

The curative-policy extension also received only brief static review and
formatting. Its regression cases are included but have not been run.
