# JSON schemas

This directory holds JSON schemas for the project's JSON formats. Settings validation is the initial implementation scope, tracked in [the browser settings plan](../plans/browser-settings.md#milestone-3-implement-validation-and-recovery). A future `seed-data.json` schema for saved seed records belongs here too.

The settings schemas use JSON Schema Draft 2020-12. [settings/generation.json](settings/generation.json) describes complete generation values after preset resolution and references [settings/skill.json](settings/skill.json) for skill assumptions. Small subordinate structures are inlined. A customization schema belongs under `settings/` when the real customization model is defined.

For settings, Rust structs and schemas are maintained separately. Fixed objects reject additional properties and require fields guaranteed by resolution. Data-driven maps describe their values generically; Rust semantic verification checks member identities, permitted proficiency levels, and item-specific minimums using prepared data. Specific technique, room, notable, and item IDs do not belong in the settings schemas.

The schemas' `urn:z3-map-rando:schema:settings:*` identifiers name resources for an in-memory registry; they are not retrieval addresses. Native and browser settings validators must package the resources referenced by their entry points and resolve references without network requests. Validator integration is pending.
