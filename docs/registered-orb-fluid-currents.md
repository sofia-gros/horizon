# Registered-fluid experience-orb currents

Part of sofia-gros/horizon-mods#379, stacked on host draft #60.

The authoritative experience-orb loop now applies the existing registered-fluid
current adapter before the orb's ordinary gravity, native water/lava handling,
player attraction and collision movement. Each positive configured body family
uses its explicit motion scale and the existing non-player current normalization,
minimum drift and finite-delta filtering. Missing/invalid contacts or flow-query
failures add no registered force; zero motion scales opt out.

Sampling uses the orb's existing 0.5-by-0.5 body and default eye offset, not the
smaller dropped-item box. NoGravity suppresses gravity, not fluid push. The
existing EntityRules push predicate is respected. The game supplies resources
only on the authoritative path; remote prediction is unchanged.

The existing orb tick signature and behavior remain intact: native buoyancy,
lava random impulses, following players, collision/border/stuck handling,
friction, bounce, age, merge rules, experience value/count, pickup and mending.
The adapter adds no water/lava alias, burn damage, custom buoyancy, fall or effect
policy. The existing native-current refresh gap in orb ticking is not changed
by this custom-fluid slice. No exact source tick-order parity is claimed.

## Source and validation

Original Rust wiring of the shared registered current adapter and the explicit
motion-scale policy already attributed to MIT Tinkers' Construct
`TinkerFluids.cool()` / `hot()` at
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, Copyright (c) 2022 SlimeKnights.
No additional external implementation, Forge source or game assets were read or
copied. No new content/schema or saved/network entity fields are introduced.

Brief static review, rustfmt and `git diff --check` only. Two unit tests were
added for actual orb-body geometry, reward metadata preservation, NoGravity push
eligibility and flowing/missing/zero-scale contacts. They exercise the geometry
and current kernel, not the live game loop, and were not run. Builds/tests,
injection planning, CI/version checks, integration and merges remain deferred
to the separate batch workflow.
