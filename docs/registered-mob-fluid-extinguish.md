# Registered-fluid mob extinguishing (draft)

A burning mob now enters its existing water/rain fire-clearing path when its
body has positive contact with a registered family whose explicit interaction
policy has `extinguishes: true`. Eye immersion is not required. The optional
catalog and bounded loaded-only sampler retain distinct family identities;
no registered fluid acquires native water tags, swimming, drowning or
water-sensitive mob damage. Invalid or oversized sampling fails closed.

The current TConstruct content has eleven enabled families: Earth Slime,
Sky Slime, Ender Slime, Ichor, Venom, Honey, Liquid Soul, Beetroot Soup,
Mushroom Stew, Rabbit Stew and Meat Soup. The other fifty-nine declarations
do not extinguish. Existing content is sufficient; no schema, texture, saved
field, protocol or compatibility-pin change is needed.

The adapter runs at the normal mob base-tick water/rain check before periodic
OnFire damage. Existing native water/rain short-circuits avoid a second custom
scan. Only a positive fire timer triggers clearing and the existing extinguish
sound at the mob center, volume 0.7, with its original two mob-random draws.
Already extinguished mobs and negative immunity countdowns do not emit a
sound or consume those draws. A later independent fire/lava or burning-fluid
contact may ignite the mob again in the host's normal order. Mixed-fluid
precedence is not redefined by this change.

The source declaration is MIT Tinkers' Construct
[`TinkerFluids.cool()` at f4e77d43](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/fluids/TinkerFluids.java).
It explicitly opts cool fluids into `canExtinguish(true)`. This is an adapter
to the existing host environment phase, not a reimplementation or assertion
of exact Forge event ordering. No Forge implementation or assets were copied.
Native water/rain sound behavior is retained, including its existing limitations.

Brief static review, rustfmt and whitespace inspection only. New synthetic
regressions cover enabled versus another nonextinguishing family, body-only
immersion, shallow/missing samples, fire clearing, sound/RNG, repeated wet ticks
and negative countdowns. They are **not run**. No tests, builds, CI inspection,
version checks or merge. Live damage/audio, mixed contacts and exact event-phase
parity remain unverified. Mob drowning/travel, passenger shielding, custom
effects, items/projectiles and mixing remain separate; #379 is not complete.
