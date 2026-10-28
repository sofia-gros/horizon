# Item appearances

Content mods can choose a client item model for an individual stack based on
JSON stored in their own `minecraft:custom_data` namespace. This affects the
shared item renderer used for inventory, held items, and dropped items; it does
not change the item's identity or gameplay. It is available in this branch,
not in released versions of Horizon.

Place a JSON file at `assets/<namespace>/horizon/item_appearances/<name>.json`:

```json
{
  "item": "minecraft:wooden_pickaxe",
  "path": ["example", "state"],
  "cases": [
    {
      "when": {"/kind": "example:tool", "/broken": false},
      "model": "example:my_tool"
    }
  ]
}
```

The path's first segment must be the mod-owned namespace. The leaf is a JSON
string (at most 4096 bytes), and each `when` key is a JSON Pointer whose value
must match exactly. The selected model is a client item definition in the same
namespace, at `assets/example/items/my_tool.json`; it can use normal composite
or layered models. Define cases so that at most one matches. Missing data,
invalid JSON, unknown models, invalid rules, or multiple matches leave the
original item model in place. Selection does not grant the mod permission to
change other namespaces' data or item models. Like all visual assets, a model
is not proof of an item's server-side state.
