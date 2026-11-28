# Item mapping

The mapping between randomizer items and z3-json-data item names is resolved at catalog-build time, so runtime code only sees `common_types` item types. `ItemKind` names a kind of item, and `Item` is an item placed at a location: it carries the identity a placed item needs, the assigned door for a Small Key and the dungeon's room ID for a Map, Compass, or Big Key. `Item::get_kind` and `Item::from_kind` convert between them, and both serialize with the same snake_case names.

## Item records

The randomizer's item records (`items` in `catalog-build.toml`, `data/items.json` by default) define every item the randomizer places. Each record has:

- `name`: the snake_case `ItemKind` token, e.g. `fire_rod` or `small_key`.
- `upstream_name`: the z3-json-data item whose receipt is used when patching, e.g. `ProgressiveSword`.
- `other_names`: further z3-json-data names that vanilla locations use for the item, e.g. `L1Sword`.
- `min`, `max`: count limits; `max` is the game limit (the most the player can make use of), which bounds future starting items but not item pool counts.
- `category`: display category. `dungeon` (Maps, Compasses, keys) and `prize` (pendants and crystals) items have their own placement and are not part of the item pool; all other items are.
- `tier`: placement tier (`major`, `high`, `mid`, `minor`, `junk`), not yet used by generation; null for dungeon items and prizes.

Records deserialize `name` as `ItemKind`, so an unknown token fails the catalog build.

## Consumers

- **Logic catalog:** each z3-json-data item definition carries the `ItemKind` it resolves to through `upstream_name` and `other_names`, or none for items the randomizer does not place, such as `OcarinaActive`. The generator reads the resolved kinds to classify locations, and creates dungeon items with the dungeon of their vanilla location.
- **Patch catalog:** item encodings (receipt ID and prize bytes) are keyed by `ItemKind`, so every dungeon's items of one kind share an encoding. The patcher looks up ordinary receipts by kind and writes each Small Key’s assigned door into its pickup table.
- **Frontend:** the records are included in `settings_data.json` by the preset preparation step; the item pool card shows the items outside the `dungeon` and `prize` categories.

The catalog builder (`catalog_builder::items`) is the only code that relates item kinds to z3-json-data names. The logic and patch catalogs fingerprint the item records, so editing them rebuilds both.
