# Native advancement item metadata

`advancements::ItemView` carries an optional owned snapshot of an item's
`minecraft:custom_data` component. The shared gameplay `item_view` projection
copies the component without changing the stack or interpreting its contents.
Stacks without that component and default views use `None`.

This gives native mod predicates access to metadata already present in the
authoritative item stack, such as a mod's installed-upgrade state. It does not
add a JSON predicate type, a vanilla custom-data matcher, a save field, a
separate network payload or a new permission. Existing vanilla predicate
evaluation is unchanged. Consumers remain responsible for recognizing their
own schema and for failing closed on malformed or unsupported values.

The snapshot is owned: later stack changes do not alter an already projected
view, and cloning the view clones its payload. Existing direct struct literals
outside Horizon need to initialize the new field or use `..Default::default()`.

## Inventory events

The existing inventory-change fingerprint includes the borrowed custom-data
payload alongside item IDs and counts. An in-place custom-data addition, edit,
or removal therefore reaches the normal criterion evaluation on the next
inventory check, even when the item ID/count stays fixed. Inventory, armor,
and offhand keep the same slot order. The unchanged fingerprint still skips
evaluation; tag reloads alone do not create an inventory change.

NBT hashing follows structural equality, with enum variants, ordered compound
keys, lossless UTF-16 strings, and floating-point bit patterns kept distinct.
It does not serialize or clone the payload in the unchanged-inventory path.
The fingerprint is process-local, not persisted or sent. The existing
per-player `last_inventory` swap continues to isolate LAN actors.

This is bounded to the newly exposed custom-data payload, not all item
components. It does not add metadata predicates, change progress/rewards,
alter remote-client authority, or fix event detection for unrelated component
edits. Hash collisions retain the same theoretical limitation as the prior
64-bit inventory fingerprint. Large custom payloads add per-check hashing
work; performance and LAN behavior have not been exercised.

Focused snapshot/default/clone and unchanged-matching regression coverage was
added, along with fingerprint mutation/stability, slot, armor/offhand, and NBT
hash coverage, but not run. Builds, CI inspection and version checks were also
deferred at the user's request. Full host compilation, native-mod integration,
actual inventory events and LAN remain unverified.
