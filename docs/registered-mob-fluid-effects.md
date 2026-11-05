# Standard registered-fluid mob effects (draft)

This slice enables the three existing source contact policies for mobs:
Venom's Poison (100 ticks, amplifier 0), Honey's Slowness (100 ticks, amplifier
0), and Liquid Soul's Blindness (100 ticks, amplifier 0). Values come from the
existing content declarations. No content or compatibility-pin changes are
required. Custom Bouncy, Ricochet and Enderference mob behavior is not enabled.

The bounded loaded-only contact sampler and block-visit pass from the mob burn
slice now collect effects even for fire-immune mobs. Positive immersion in the
exact source family is required; another liquid, shallow space above the
surface, missing chunks or invalid/oversized queries cannot trigger it.
An authoritative event carries the mob and registered identity to the game.
There it resolves the standard effect ID and applies the same current entity-tag
and species immunities used by potions. Dead/removed mobs and remote clients
are ignored. No new immunity rules or native liquid aliases are invented.

Accepted effects use the ordinary active-instance duration/amplifier update
and visible, nonambient flags. The ordinary effect-added hook is notified only
when the active data or flags change, so duplicate visits do not create duplicate
hook notifications. Existing mob effect ticking and effect metadata rendering
handle Poison/Slowness/Blindness; this change invents no special Blindness AI.

## Curative policy

The source gives these contact instances an empty curative-item list. Mobs now
retain an explicit active-instance opt-out alongside their existing effect
list. Stronger or longer accepted replacements take the incoming policy;
ignored weaker/shorter refreshes do not change it. Ordinary accepted instances
remain ordinarily curable. `Mob::cure_effects` removes ordinary instances but
preserves opted-out ones. No new mob milk/drinking behavior is added: consumers
that implement generic mob curing must use this path, while direct removals
and `/effect clear` remain unrestricted.

Saved `active_effects` carry the same optional `horizon:no_curatives` byte used
by players. Missing means ordinary curability. Expiry and direct command
removal prune stale metadata. No protocol format changes are needed because
curing is server-authoritative. Existing mobs do not retain hidden weaker
instances; their restoration and full mixed-source curative parity remain
unimplemented rather than claimed by this active-instance adapter.

Source: MIT Tinkers' Construct
[`MobEffectLiquidBlock.entityInside` at f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/block/MobEffectLiquidBlock.java).
Copyright and full notice are retained. No Forge implementation or game assets
were copied.

Brief static review, rustfmt and whitespace inspection only. Synthetic contact,
active-policy/curing and NBT round-trip regressions are written but **not run**.
No tests, builds, CI inspection, version checks or merge. Live effect behavior,
network/persistence integration and exact source event ordering remain
unverified. Mob movement, drowning, passenger shielding, custom
mob effects, dropped items/projectiles and mixing remain outside this slice.
The #379 high-priority world-fluid item is still incomplete.

Registered cool-fluid mob extinguishing is covered by the later
`registered-mob-fluid-extinguish.md` slice.
