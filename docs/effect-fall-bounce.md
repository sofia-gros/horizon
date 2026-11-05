# Opt-in player effect landing (draft)

Native mods can set `StatusFlags.bounce_on_fall` after deriving the player's
effect/equipment flags. It defaults to false and no built-in effect enables it.
This does not replace vanilla block restitution or enable archived mod injections.

The response follows MIT TConstruct `ModifierEvents.bounceOnFall` at
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`: ignore gentle/short falls and steps;
suppressing bounce uses damage multiplier 0.5. Otherwise the authoritative
player receives `gravity * sqrt(fall_distance / gravity)` upward velocity,
with X/Z divided by the source 0.975f value. Predicting clients use the source
0.7f/0.9f reverse-velocity factors. A deferred response is applied once after
native collision restitution, clears fall distance and marks authoritative
velocity for synchronization. The guest move path applies the same response.
Native fall cancellation takes precedence. Creative flight immunity and normal
effects that reset fall distance remain intact.

This slice covers player landing, not the source's full bounce handler. Repeated
air-momentum preservation, bounce sounds, non-player entities and complete
client/server timing parity remain separate. The caller owns enabling/resetting
the flag from current effects. The paired mod work enables Bouncy; armor behavior
is not implicitly activated.

Brief static review and formatting only. Deferred regressions cover threshold,
crouch and rebound arithmetic plus a real motion landing/restitution path. No
tests, builds, CI inspection, version checks or merges were run.
