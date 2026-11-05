# Registered fluids in content mods (draft)

A content mod can provide `data/<modid>/registered_fluid/<name>.json`. Each file
contains one complete fluid entry from the format-1 resource schema, including
source, flowing, block, bucket, policy and visual fields. Source, flowing, block
and bucket IDs must belong to that mod's own namespace. The existing content
fence still applies; no new native module is needed.

The content builder merges these entries with any base registered-fluid resource,
rejects duplicate identities and validates the aggregate against the newly built
block registry before writing `horizon/registered_fluids.json`. Invalid state
families or policies fail the overlay build. Item existence and sprite bindings
are validated by their normal resource owners when loading the completed root.

The mod must also declare the sixteen-level liquid block and bucket item through
its normal block/item content. Merely adding this file does not manufacture those
registrations, textures or contact behavior. No TConstruct fluid is enabled by
this patch: the first source-verified declaration still needs its physics/contact
implementation before release.

Focused validation: aggregate merging, duplicate/foreign identity rejection,
nonmutation on failure and final registry binding test passed. Whitespace and
public-tree checks passed. Content-builder compilation and full overlay fixtures
are deferred to the requested batched integration check; keep this draft unmerged.
