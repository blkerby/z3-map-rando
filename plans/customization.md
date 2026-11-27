# Racing rulesets

Status: proposal; not implemented. Customization currently covers presentation/control preferences (`schema/settings/customization.json`, read by the patcher as `CustomizationSettings`, with built-in presets under `data/presets/customization/`). Post-generation gameplay and logic adjustments are handled directly by unlocking generation settings on the seed page (transient local edits that take effect in the saved ROM and Spoiler, and can be saved as presets or carried into a new game). This plan covers future racing rulesets. Persisted formats are specified in [settings](../docs/data-formats/settings.md) and [seed data](../docs/data-formats/seed-data.md).

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
