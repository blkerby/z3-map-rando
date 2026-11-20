# Customization and racing rulesets

Status: proposal; not implemented. Customization currently covers only presentation/control preferences: the settings in [`schema/settings/customization.json`](../schema/settings/customization.json), read by the patcher as `CustomizationSettings`, with built-in presets under `data/presets/customization/`. This plan covers gameplay adjustments at patch time, their compatibility with the seed, and future racing rulesets. Persisted formats are specified in [settings](../docs/data-formats/settings.md) and [seed data](../docs/data-formats/seed-data.md).

## Ownership and the customization annotation

Settings changing gameplay rules belong to generation from the outset, even if the current logic does not yet model their consequences. Record their chosen values with the seed as its intended gameplay behavior. Movement abilities, combat rules, and resource behavior are examples. Customization-only settings concern presentation and personal control preferences that do not change those game rules.

A feature such as preserving dash speed through transitions has one definition but two possible concrete values: the seed's recorded generation value and the effective patch-time value after an optional adjustment. They may differ legitimately. This is separate from Skill assumptions: whether the game provides an ability and whether the player can execute a strat using it are different questions.

Initially use one annotation on a generation setting to indicate that it is exposed for temporary customization and included in the patcher's effective settings. A possible spelling is `customizable: true`; the exact syntax remains open, and it may instead be an object containing its compatibility relationship. Do not introduce independent "consumed by patcher" and "overrideable" flags without a concrete setting requiring that distinction. Customization-only presentation/control settings are already patch inputs.

The annotation is semantic metadata, independent of UI section membership. Reorganizing the UI must not change the setting's ownership, identity, or inclusion in saved presets.

Not everything consumed by the patcher is an adjustable setting. World layout, item placements, and other fixed generation results remain seed facts. A setting whose change requires regeneration should not be exposed through the patch-only adjustment mechanism merely because its effects ultimately involve ROM writes. If a genuine configuration field is needed by the patcher but should never be adjustable, reconsider whether separate annotations earn their weight then.

## Effective patch inputs and generation compatibility

A shared coordinator, used by CLI and browser callers, projects only the relevant generation values from the seed, combines them with temporary gameplay adjustments and presentation/control preferences, and evaluates compatibility. The patcher receives the resulting concrete patch settings and required seed artifacts, without needing arbitrary generation settings such as tech/notable assumptions. Interpretation and compatibility policy belong outside the low-level patching implementation.

For a feature whose enabled behavior only adds capabilities, the directional relationship can be:

| Recorded generation value | Effective patch value | Interpretation |
| --- | --- | --- |
| Disabled | Disabled | Matches generation |
| Disabled | Enabled | Adds a capability without invalidating the assumed abilities |
| Enabled | Enabled | Honors the generation assumption |
| Enabled | Disabled | May invalidate the generation assumptions |

Compatibility is feature-specific; do not infer that every gameplay modification only adds capabilities. Other features may require equality or another defined relationship. A potentially incompatible adjustment does not prove the seed is unbeatable, since generation may not actually have needed the ability. Whether to reject it or permit it with a warning is an open product decision.

Recorded gameplay behavior and modeled logic assumptions are not identical. When a later release begins modeling an existing feature, compatibility handling for old seeds must respect the original generation semantics rather than retroactively claiming that an older generator assumed a capability it never modeled. Define any conservative warnings or historical compatibility translations deliberately.

## Gameplay adjustments and persistence

Patch-time gameplay adjustments are excluded from saved customization presets and their imports/exports. Someone generating their own seed selects the intended gameplay settings during generation; someone opening another person's seed initially follows that seed's recorded gameplay settings. Loading or saving a presentation/control preset must not capture, replace, or propagate gameplay adjustments.

Adjustments apply only to the current seed and should survive repeated patching and page refreshes. Use one active-seed draft in `sessionStorage`, tagged with the seed's identity and containing only values differing from that seed's settings. Restore it only when the seed identity matches. Switching seeds discards the previous draft and starts from the new seed's recorded values; switching A to B and back to A does not restore a history of old adjustments.

This is temporary draft storage rather than a reusable preference or a change to the shared seed record. Restored adjustments remain visibly marked and receive the same compatibility checks as new edits. A control can show the effective value, highlight a difference from the seed, and reset to the seed's value; an override equal to the seed value is redundant and can be removed.

Historical seed settings remain frozen while the user's patch-time customization is interpreted for the patcher being used; the customization vocabulary is not bound to the seed's generation-time definitions.

## Racing rulesets

Keep racing conformance separate from generation compatibility. A patch-time change can preserve the ability to complete a seed yet violate the race's restrictions. Rulesets can constrain any applicable patch-time setting, including presentation choices, rather than only gameplay adjustments.

Define rulesets as versioned JSON constraints keyed by setting paths, evaluated by generic code. A particular tournament's rules should require data rather than a special branch in Rust. The following is a provisional example:

```json
{
  "version": 1,
  "constraints": {
    "preserve_dash_speed": {
      "match_seed": true
    },
    "link_sprite": {
      "allowed_values": ["link"]
    },
    "menu_speed": {
      "disallowed_values": ["instant"]
    }
  }
}
```

`match_seed: true` requires the effective patch value to equal the seed's recorded generation value. `allowed_values` requires membership in the given set. `disallowed_values` forbids membership in the given set. All constraints present on a setting apply together: a value listed in both sets is forbidden. A setting without a constraint has no additional restriction from this ruleset, while ordinary validation and generation compatibility still apply. Validate rule keys and operands against the setting definitions; matching a seed value is meaningful only where such a value was recorded.

Start with a small constraint vocabulary. Numeric bounds, conditions, and relationships between settings can be added when an actual ruleset requires them. There is no need for a general expression language initially. New operators require evaluator code; a new ruleset using existing operators does not.

Evaluate the final concrete patch configuration, after inherited seed values, temporary gameplay adjustments, and presentation/control preferences are resolved. A customization operation resulting in the same effective value is not itself a violation. Produce structured findings identifying the setting, constraint, expected value or allowed/disallowed set, and actual value. Keep generation compatibility findings and racing findings as distinct results.

Capture the applied ruleset definition and interpretation version with the seed, rather than referring only to the latest definition of a named ruleset. Later changes to a race's rules must not reinterpret existing seeds. Public ruleset metadata follows the same restrictions on arbitrary user text as the rest of the public seed record.

Conformance evaluation does not choose enforcement. Future behavior could reject nonconforming customization, permit it with an in-game indication on the title screen and/or menu, or use another explicitly selected policy. No enforcement policy is selected yet. Keep the final status available to whichever layer implements that policy; the patcher need not implement the generic evaluator or consume the full generation configuration merely to apply an indication.

```text
seed's relevant gameplay values + temporary adjustments + presentation/control preferences
  -> effective patch settings
  -> generation compatibility findings and racing conformance findings
  -> enforcement decision
  -> narrow patch inputs and any required status indication
```
