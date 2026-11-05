# Registered fluid bucket adapter (draft)

Custom declared bucket items now enter the authoritative item-use path. Empty
buckets raycast registered sources while passing through flowing cells. Full
buckets target the adjacent block; the bounded level adapter accepts air or the
same family, with no waterlogging, washing blocks, evaporation or reactions.

The normal build/protection and native bucket callbacks run before mutation.
Inventory changes only after the world confirms placement or source removal.
Stacked containers shrink by one; the existing inventory helper handles returned
containers and full-inventory drops. Creative placement keeps the full bucket;
creative pickup uses the existing filled-result behavior. Snapshot attempts during
the mutation are rejected. If scheduling fails after a committed block change,
inventory still follows the block before the error propagates.

Guests use the same server item-use path. Their client sends UseItem and waits for
authoritative block/inventory correction rather than predicting custom fluid as
water/lava. Client swing is a target-presence hint; protection and replacement can
still reject the action server-side. No custom fluid sound is guessed.

Validation for this stage is deliberately focused: the production engine's
placement/flow/pickup/drain cycle test passes; rustfmt and whitespace checks pass.
The new game adapter has not yet been compiled or runtime-tested. Per the staged
integration plan, full client compilation and real host/guest inventory tests are
batched after implementation. This is not a claim of player readiness. Immersion,
source-verified content declarations and independent integration review remain.

## Dispensers

The next opt-in slice connects every loaded registered-fluid bucket to a
server-side dispenser. It places into the cell directly in front for all six
facings and collects only registered source cells with an empty bucket. The
item is resolved before removing the source. Native dispense overrides still
run first; ordinary native buckets and droppers retain their existing paths.
No player inventory, creative mode, player hit result or player bucket callback
is used for an automated action. Generic level mutation protection still applies.

Both player and dispenser use the same bounded source transaction and restore
the scheduler after success or error. Dispensers consume one container and use
the existing remainder insertion/drop helper. The selected slot and container
change notification commit before any post-mutation scheduling error is
returned; level events are processed only after that inventory commit. A full
bucket that cannot place uses the ordinary item-ejection fallback (including
the nested click/animation in TConstruct's implementation). Empty-bucket failure
uses its single ordinary ejection/click/animation. Successful transfers emit
`fluid_place`/`fluid_pickup` with no player actor. Fluid-specific bucket sounds
are not guessed.

This is enabled by declared content, with no built-in TConstruct identifiers.
The companion seventy-family declarations are in
[horizon-mods #811](https://github.com/sofia-gros/horizon-mods/pull/811).
The source reference is MIT TConstruct `TinkerFluids.commonSetup` at
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`; the generic host also accepts declared
bucket families beyond that source's explicit dispenser list. Stacked filled
containers use the host's one-container remainder handling rather than assuming
source item stacks are always size one.

### Deferred validation and boundaries

Only a brief static review, rustfmt and whitespace inspection were performed
for the dispenser slice. The new outcome/error-order regression was added but
**not run**. No build, test, CI inspection, version check or merge was performed.
Final integration should cover six facings; source placement and pickup;
flowing-cell rejection; obstruction, other-family and unloaded-chunk rejection;
stacked empties and full remainder inventories; native override cancellation;
vanilla buckets and droppers; save/reload and observing LAN clients. The transfer
still accepts only air or a changing same-family cell: already-full sources do
not consume a bucket, and washing blocks, waterlogging, evaporation, reactions,
source-specific bucket sounds and container extra contents remain outside this
slice. This does not establish live dispenser or multiplayer parity.
