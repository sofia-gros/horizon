# Registered-fluid mob boat-passenger shielding (#379)

This opt-in host slice clips registered-fluid mob observations to the portion
of a boat passenger above the boat hull. It stacks on common float/swim AI.
It reuses the existing player boat-box convention and authoritative Boat state;
it does not add boat physics or change native-water mob sampling.

Before an authoritative mob tick, the game refreshes an ephemeral snapshot of
its immediate boat vehicle's ID, top Y and existing underwater flag. A missing
boat, dismount or non-boat vehicle clears the snapshot. Canceled ticks remain
unchanged. Every observation also checks that the snapshot ID still matches the
mob's current mount, so a stale snapshot cannot shield a dismounted passenger or
a passenger transferred to another vehicle. The snapshot is not written to
entity NBT and is rebuilt before ticking after reload.

For a matching boat that is not marked underwater, the already-deflated body
box starts at the higher of its normal minimum and the boat top. A wholly
sheltered body skips registered queries altogether. A partial body still contacts
liquid above the hull, and eye samples below the hull cannot trigger drowning.
An underwater boat, absent/mismatched snapshot or nonfinite hull height leaves
ordinary registered contact geometry in use.

The shared clipped observations cover body burn/status contact, extinguishing,
travel/jump and float-AI selection, eye drowning, current forces and fall-distance
scaling. Current queries use the same clipped body bounds, including their
geometry-only fallback. All existing loaded-only and budget limits remain.
Dry exit destination probes remain independent of the passenger's hull.

No content, native liquid flags, network event or persisted field is added. This
is original wiring of Horizon's existing boat snapshot and contact adapters;
no external implementation or game assets were read/copied. It does not claim
exact source passenger/event-order parity.

Brief static review, rustfmt and whitespace inspection only. Four added
synthetic regressions cover full shielding without any lookup, no consequent
burn/current/fall/drown/travel/float activation, exposed upper body, sheltered
versus exposed eyes, underwater boats, dismounts, changed/missing snapshots and
nonfinite hull heights. They are **not run** and do not replace live boarding,
boat movement, reload or network tests. No tests, builds, CI inspection, version
checks or merge were performed.

Limits: this uses the boat's existing underwater status, which remains based on
its native water simulation; it does not detect custom-fluid boat submersion or
supply custom-fluid buoyancy/hydrodynamics. Only immediate boat passengers are
clipped, not nested passengers riding another mob aboard a boat. Native-water
mob contact handling is unchanged. Mid-tick movement of the boat is not resampled.
Species-specific registered-fluid behavior, custom mob status effects, boat
hydrodynamics and final integration remain open. #379 is not complete.
