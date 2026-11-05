# Registered-fluid kernels on the existing fork main

This is a bounded backport of the data catalog, named scheduler and downward-flow
kernels from native PRs [#14](https://github.com/sofia-gros/horizon/pull/14),
[#15](https://github.com/sofia-gros/horizon/pull/15) and
[#16](https://github.com/sofia-gros/horizon/pull/16).
It starts from fork main `2b3f20bd96231558fbef1f6d6526cfb29418627a`, not the newer
pinned host used by the original stack. No upstream history is merged and none
of the fork-only main changes is replaced.

## Included source commits

- `86ea8251f88d66998fb15c8f11587f2558d1ec3f`: catalog and named persistence.
- `3b8d50ff216549a98b424c94824fab7fe4f40dd1`: reject extra block state variants.
- `f77bd732a0e01ec4588fa2dd9edf3b3ebacc4c74`: named chunk scheduling.
- `847406057c6daea71e64b7ccecf828d6d18fd32e`: flow and source transactions.
- `b7481e6b4e6a26678549fc9a9b66dfa55382727d`: prospective identity checks.

The original source commits are recorded by the cherry-pick trailers. The only
existing production-file change outside the new modules is two exports in
`horizon-world/src/lib.rs`. Existing vanilla callers are unchanged.

Review found one contract gap in the inherited kernel: an in-place flowing-to-source
transition bypassed the destination's exact fluid-identity acceptance predicate.
The backport checks that predicate before any nonempty transition is committed.
A regression failed before this fix, then passed for both rejected conversion
(no cells or scheduled ticks changed) and permitted conversion. Depletion and
unchanged-state behavior are preserved.

## Validation and remaining gates

The final library suite passes 202 tests with one existing ignored test. The
initial repository self-check passed 399 Python tests, all formatting and public-tree
checks, 280 world tests and doc tests. Final post-fix aggregate results are recorded
in the pull request.

Strict Clippy fails with 12 diagnostics in unchanged main-side datafix and aquifer
code. The same command was run against an isolated checkout of the exact base and
reported the same 12 locations. They are not silently suppressed or repaired by
this scope. The full verification gate is therefore not green.

This does not enable registered fluids in a running client or server. Production
resource loading, authoritative lifecycle callbacks, rendering/immersion, bucket
inventory transactions, custom effects, synchronization and real save/reload/LAN
validation remain outside this draft. The original foundation/flow integration
gates remain in effect. TConstruct, PlutoLib, Mixins and released host pins are
unchanged. This is not compatibility for the 70-fluid declaration stack and does
not complete the broader TConstruct world-fluid task.
