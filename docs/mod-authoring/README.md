# Mod authoring in this fork

This guide describes the `sofia-gros/horizon` fork, not a promise that an arbitrary upstream Horizon release has the same API. The pages describe the fork's `main` at commit `fc4848f4d7920390ea3aa8e6a5f804111f93a45d`. Server-side item and workstation APIs are experimental. Check the revision and package requirements before changing versions.

- [Get started](getting-started.md): build, test and run a first mod.
- [Architecture and capabilities](architecture.md): where code runs, registration and trust boundaries.
- [Workstations and inventories](workstations.md): define a GUI and safely plan changes.
- [Items and gameplay hooks](items-and-hooks.md): per-stack state, mining and combat.
- [Compatibility and limits](compatibility.md): versions, conflicts, failures and unsupported cases.
- [Generated API reference](../api/mods-reference.md): exact WIT interfaces, types and capabilities. Its [generation instructions](../api/README.md#references-are-generated) remain authoritative.

For the wider runtime design, manifest schema, complete scenario format and older client API, see [the existing mod guide](../mods.md). For the port's actual feature coverage and screenshots, see [Horizon Mods](https://github.com/sofia-gros/horizon-mods/tree/ce3a5ab5e8a51ea01d1da502ce5f11d0584ef0ff). The fork's development PRs can contain features not yet on `main`; they are not part of this guide's baseline.
