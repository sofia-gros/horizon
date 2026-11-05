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

Focused snapshot/default/clone and unchanged-matching regression coverage was
added but not run. Builds, CI inspection and version checks were also deferred
at the user's request. Full host compilation, native-mod integration, actual
inventory events and LAN remain unverified.
