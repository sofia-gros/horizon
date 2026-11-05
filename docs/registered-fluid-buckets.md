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
