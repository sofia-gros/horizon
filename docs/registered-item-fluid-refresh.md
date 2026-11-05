# Post-movement registered item-current refresh

Part of sofia-gros/horizon-mods#379, stacked on host draft #65.

Draft #54 added registered currents beside the first native item-fluid refresh.
The existing item tick also refreshes native fluids at its end, after movement,
drag, synchronization comparison and age update. Registered currents now run
beside that second refresh as well, using the item's new body position rather
than cached pre-move contacts. The existing grounded idle branch reaches this
refresh too.

A continuously immersed item can therefore receive two current updates in one
tick, with ordinary movement/drag between them, matching the host's native-refresh
structure. Entering another family selects its force; leaving all registered
fluid adds no second force. The refresh changes velocity, not position. Existing
native immersion/buoyancy, current normalization/minimum drift, missing-query
handling, pickup/age/despawn, collision and fire/contact rules are unchanged.
The caller's existing optional-resource/authority gate remains in force.

This does not add custom item buoyancy or change the native tracker comparison
phase. The post-movement force occurs after that existing comparison, just like
the native second push. Exact source tick-order/network parity is not claimed.

## Source and verification

Original Rust wiring of the existing host item refresh phases and registered
current adapter. The MIT Tinkers' Construct motion-scale policy attribution at
f4e77d43f39ba1047bf93ae816b7e1beaab514c2, Copyright (c) 2022 SlimeKnights,
remains unchanged. No additional external implementation, Forge source or game
assets were read or copied.

Brief static review, rustfmt and git diff --check only. Two unrun tests exercise
fresh item-body sampling and current kernels across a family change, leaving the
fluid, successive post-drag updates and unchanged-position refreshes. They do
not execute the real ItemEntity tick or grounded-idle branch. No tests, builds,
injection planning, CI/version checks, integration or merge performed; the
separate batch worker owns consolidated verification. Previously reported native
item/stand test-signature omissions remain for that batch repair.
