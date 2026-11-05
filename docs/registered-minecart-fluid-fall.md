# Registered-fluid minecart fall-distance scaling

Part of sofia-gros/horizon-mods#379, stacked on host draft #64.

At the existing authoritative post-movement fluid refresh, each positive body
family now multiplies the minecart's current fall_distance by its declared
fall_distance_scale. Source and flowing cells of one family coalesce; independent
families compose. A zero factor resets, one preserves, and continued contact
scales again on each refresh. NoGravity and a zero motion scale do not disable
fall policy.

The current adapter's contact snapshot is reused. If current sampling fails or
the entity is not pushable, a bounded loaded/in-height geometry-only query still
allows the fall policy to apply. It adds no fallback force. Eye-only, missing,
above-surface and invalid geometry contacts do not alter fall distance.

This uses the existing cart accumulator, including the distance a later TNT-cart
landing checks. The movement phase precedes this refresh: an already-recorded
fall_blast is deliberately not canceled or recomputed by fluid contact afterward.
Native water still resets the accumulator after registered scaling; rail,
bubble-column and landing resets retain their order. Fuse and current/blast
speed logic, native water flags and all cart variants remain unchanged.

Remote prediction remains excluded. No new fall damage for ordinary carts,
custom buoyancy/travel, status effects, persistence, network fields or schema
are introduced. Exact source tick-order parity is not claimed.

## Source and verification

Original Rust integration with the existing host cart accumulator and registered
interaction contract, using the MIT Tinkers' Construct policy already attributed
at f4e77d43f39ba1047bf93ae816b7e1beaab514c2, Copyright (c) 2022 SlimeKnights.
No additional external implementation, Forge source or game assets were read or
copied.

Brief static review, rustfmt and git diff --check only. Three unrun unit tests
exercise actual cart sampling and the scaling helper: family coalescing,
repeated/mixed factors, zero/identity, zero-force geometry, pending TNT-blast
preservation and excluded contacts. They do not verify the live world fallback,
Game tick or later explosion outcome. Tests, builds, injection planning,
CI/version checks, integration and merge belong to the separate batch workflow.
