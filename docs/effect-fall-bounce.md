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

This slice does not implement the source's full bounce handler. Bounce sounds,
non-player entities and complete client/server timing parity remain separate.
The caller owns enabling/resetting
the flag from current effects. The paired mod work enables Bouncy; armor behavior
is not implicitly activated.

Brief static review and formatting only. Deferred regressions cover threshold,
crouch and rebound arithmetic plus a real motion landing/restitution path. No
tests, builds, CI inspection, version checks or merges were run.

## Airborne momentum continuation

The opt-in landing now starts per-player momentum state adapted from MIT
TConstruct `SlimeBounceHandler` at the same revision. When airborne, outside
native water and not on a climbable, loss of horizontal speed is corrected
toward the previous magnitude times the source `0.975f` factor. Vertical
velocity is untouched. Acceleration above the stored magnitude is not captured
as a new baseline, and a correction is applied only when its boost exceeds one.
An accepted authoritative correction marks velocity for synchronization.

The source stop timer clears the stored magnitude after more than five ticks
of zero horizontal motion. Ground/water/climbable contact ends the handler
after more than five ticks, reset by returning airborne. Gliding and spectator
mode remove it immediately. Ordinary creative flight is not an extra source
cancel condition. An existing server handler keeps its magnitude across another
rebound; a predicting client's explicit rebound refreshes that magnitude.
A started handler persists when Bouncy expires, as the source does; the status
flag only controls starting a rebound. State is transient, not saved.

The host updates after fluid observations and before supported local/guest
movement, guarded against repeated processing of the same health-clock tick.
Spectator guests also clear state on their early-return path. Native block
restitution and slime-block coefficients are unchanged. No water behavior is
assigned to registered-fluid identities. Predicting passenger ticks, source
one-tick deferred client rebound, exact living-event phase and LAN convergence
still need later validation. Source angle-change audio is not implemented.

Brief static review and formatting only. Pure deferred regressions cover speed
correction, unchanged Y, strict deadlines, air resets, cancellation and repeat
registration; the existing landing fixture also checks continuation after
effect removal and immediate glide cancellation. None were run. No tests,
builds, CI inspection, version checks or merges were performed.
