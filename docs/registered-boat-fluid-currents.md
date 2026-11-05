# Registered-fluid boat and raft currents

Part of sofia-gros/horizon-mods#379, stacked on host draft #63.

Host-simulated boats and rafts now receive registered-family current forces
immediately after their existing native fluid refresh, before floatBoat damping,
buoyancy, steering and movement. The shared geometry applies to ordinary/chest
boats and bamboo rafts. The existing entity push predicate is respected;
NoGravity suppresses gravity, not fluid currents.

Sampling uses the actual 1.375-by-0.5625 hull and existing top-of-body fluid-eye
position, with the shared 0.001 deflation, loaded-only bounded query and explicit
per-family motion scale. Positive body contact suffices; eyes need not be under
the surface. The non-player current kernel preserves direction normalization,
minimum drift and finite-delta safeguards. Opposing currents can cancel. Missing
or invalid contacts, query failures and zero motion scales add no registered
force.

Water status, old status, water level, native buoyancy/drag, passenger shielding,
submersion ejection and bubble-column state retain their existing meanings.
Custom liquid is not aliased to water or lava. Applied forces subsequently pass
through the existing float/control/collision logic, rather than being added after
movement. No persistence, network, resource-schema or content declaration changes.

## Movement authority and limits

Only the existing local/host simulation path (remote.is_none()) receives these
forces. Local riding uses the same tick; remote prediction remains unchanged.
LAN guest-driven boats are deliberately not simulated by the server's boat loop,
so this slice does not implement currents for those client-owned movements.
No additional server force is injected into them or their riders.

Custom floating/status, boat burn/extinguish, custom travel, fall policy and
remote/guest movement support remain separate work. Passenger shielding behavior
from previous drafts is not modified. This is current-force support, not a claim
that boats float on every configured fluid.

## Source and verification

Original Rust integration with the existing host boat tick and registered-current
adapter, consuming the MIT Tinkers' Construct TinkerFluids.cool()/hot() motion-scale
policy already attributed at f4e77d43f39ba1047bf93ae816b7e1beaab514c2,
Copyright (c) 2022 SlimeKnights. No additional external implementation, Forge
source or game assets were read or copied. Exact source tick-order parity is not
claimed.

Brief static review, rustfmt and git diff --check only. Two unrun unit tests cover
actual ordinary/chest boat/raft hull sampling, shallow body-only contact,
NoGravity eligibility, non-player normalization, opposing-current cancellation
and missing/above-surface exclusion. They exercise geometry/current kernels, not
the live Game tick, post-current buoyancy or networking. Tests, builds, injection
planning, CI/version checks, integration and merge remain with the separate batch
workflow.
