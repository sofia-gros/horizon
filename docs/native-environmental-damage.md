# Native environmental mob damage

`mobs::native_damage::NativeDamage::from_catalog` resolves a native code-owned,
static namespaced damage type against an immutable world damage catalog. Wrap
it in `DamageKind::Native` and use `Mob::damage` or `Mob::damage_with`. The normal
mob damage path remains authoritative: invulnerability, armor, Resistance,
hurt cooldown, absorption, death protection, native cancellation/modification
hooks, combat tracking and events remain in place. The source's real identifier
reaches native damage hooks and loot context; it is not a vanilla Magic alias.

This first bridge is deliberately environmental. It rejects vanilla identities,
missing definitions, difficulty scaling other than `never`, non-default death
message rules, unsupported Minecraft damage tags, a causing attacker, and
negative or non-finite input amounts. Supported tags are enumerated in the
module. Do not use it to approximate projectiles, explosions, freezing, direct
attacks or other unsupported damage semantics. A rejected catalog must disable
the caller's action, not select a vanilla fallback.

The descriptor copies resolved tag bits and borrows only a static native type
name. It does not register global state, leak strings, or depend on a later
world's catalog. Rebuild descriptors after a world resource reload. Historical
combat records keep the source snapshot of their original hit. Damage
identifiers, data-pack loading, and entity/tank transactions remain the native
mod's responsibility; this bridge does not create a new network registry.

Vanilla variants retain their previous paths. The new behavior is opt-in.
Tests exercise actual `Mob::damage` with fire immunity, Fire Resistance,
Witch resistance, armor, cooldown, invulnerability, Resistance, source retention,
invalid input rejection and the existing native cancellation/modification hook.
