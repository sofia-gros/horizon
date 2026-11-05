# Registered fluid player interaction (draft)

The optional `interaction` declaration binds one explicit policy to the source
and flowing IDs of a family. It supplies travel acceleration, horizontal and
vertical drag, gravity scale, jump impulse, current scale, swim/drowning flags,
extinguishing, fall-distance scaling, optional burn seconds/damage and an optional
sustained contact effect. Invalid
numbers fail resource load. Missing policy keeps observations inert.

Player base ticks sample loaded cells, preserve independent currents per family,
apply configured push and fall scaling, and respect the boat's sheltered volume.
Native water/lava tags stay unchanged. A configured body contact uses its own
travel path, jump and collision escape; drowning is selected by the eye's family.
Block-inside burning uses that family's positive body-height guard, the declared
burn time and damage through the existing fire/health path. The same motion code
serves predicting and authoritative players.

An `effect` object accepts `minecraft:poison`, `minecraft:slowness` or
`minecraft:blindness`, or a namespaced effect declared by an enabled content
mod. `duration_ticks` ranges from 1 through 72000 and `amplifier` from 0 through
255. Optional boolean `no_curatives` defaults to false.
Omission keeps earlier declarations unchanged. Malformed IDs and unsupported
fields fail parsing. Other vanilla effects,
including instantaneous effects, remain rejected. The mod-overlay builder
resolves custom IDs against enabled content declarations, and the game resource
loader resolves them against the live effect registry. Missing effects fail
loading rather than becoming inert status entries. Core-only callers of
`RegisteredFluidResources::from_reader` must call `validate_effects` with their
registry before making the resources live. Positive body height for the block's
own family is required; eye-only
or other-family observations cannot trigger it. Source and flowing blocks share
the same policy.

The movement path emits a catalog-scoped fluid identity, and the authoritative
player dispatcher resolves that family's effect through the existing registry
and effect merge/attribute path. Stale catalog identities cannot target a new
family. Custom effects use the existing native effect-added/started/tick hooks,
network IDs, HUD and save paths; the host does not invent their gameplay
behavior. Predicting clients do not add effects
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
Mobs are not handled by this player-only extension. Custom-effect declaration
and dispatch support does not implement TConstruct Bouncy, Ricochet or
Enderference. Those effect definitions/hooks and their four fluid declarations
remain separate work.

Source: MIT TConstruct f4e77d43, `TinkerFluids`, `BurningLiquidBlock.entityInside`
and `MobEffectLiquidBlock.entityInside`.
The generic configurable travel integrator uses Horizon's existing movement and
fluid-falling primitives, not Forge implementation code. Matching Forge defaults,
mixed-fluid precedence, and movement sequences still need an independent oracle.

Remaining: mobs/items/projectiles, custom potion effects, item-specific curative
lists and mixed-source merge parity,
camera fog, fluid mixing/solidification, dispenser use, and live save/reload/LAN acceptance. This is not complete
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

The custom-effect contact extension received brief static review and formatting
only. Its registry-resolution/family-isolation regressions have not been run;
no tests, builds, CI inspection, version checks or merges were performed.

The burn-audio extension follows `BurningLiquidBlock.entityInside`: after the
normal health path accepts lava-type contact damage, emit the additional
`minecraft:entity.generic.burn` sound at volume 0.4 and f32 pitch
`2.0 + level.random.nextFloat() * 0.4`. A rejected cooldown, immunity, native
damage cancellation or predicting-client hit emits nothing and consumes no
pitch random draw. A stronger accepted cooldown hit still emits the extra
sound, independently of whether a new hurt animation/sound was generated.

The event retains its contact position and sampled pitch until authoritative
dispatch. Like the source player's `playSound`, the packet uses the player
sound category and excludes the hurt player; it is separate from that player's
ordinary damage-packet hurt sound. The server window plays its local copy
without capturing another broadcast. No client-side speculative burn sound is
invented. All declarations with a burn policy use this path; native lava and
non-burning registered fluids are unchanged. Mobs, vehicles and dropped items
remain outside this player-only registered-fluid path.

Brief static review and formatting only. A deferred real-health-path regression
covers fresh/stronger accepted hits, cooldown rejection, invulnerability,
spectator, prediction, fire resistance and pitch RNG consumption. It has not
been run; no tests, builds, CI inspection, version checks or merges were made.
Actual audio audibility, packet audience and gameplay/LAN remain unverified.
