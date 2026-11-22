# Adapt to the Bunny logic update

Source revision `a1bf514` replaces `glove` with `lift` and `carry`, adds item-use requirements, and expresses Bunny capabilities and conversions in ordered strat requirements.

- [x] Compile the new requirements, including item-use constraints on equipment and consumable actions. Apply the legacy `isBunny` gate only to authored dungeon strats. Dungeon strats require Link unless marked `yes` or `any`, in which case they accept the incoming form. Keep the normal technique gate on one-time conversions.
- [x] Track Y and A/B usability on each traversal route, including key assignment. Preserve form across ordinary movement and apply form changes on the relevant transitions. After collecting Moon Pearl, retain local states and revisit the Pearl-dependent resets. Save-and-quit and retracing the route provide Link access without granting a local form change at the pickup.
- [x] Find a metric array with five four-level columns and twelve binary columns. Require all exact pairs and all low/high triples, grouping resource levels 0/1 and 2/3. Verify any construction independently and record the proven minimum or the remaining gap.
- [x] Rebuild the affected catalogs. Check Pearl-less and Pearl-reset routes, the split Bonk Fairies entrance, Skull Woods traversal, and fixed vanilla and random-layout seed generation. Check key assignment, retries, and runtime. Patch a smoke-test ROM if a verified base is available.

## Metric array search

A SAT search that holds the original 18 rows fixed finds that adding just the two capability columns is infeasible. Allowing one new row produces a 19-row construction. An independent check verified all 136 exact pair projections and all 680 low/high triple projections.

The minimum is at least 18. Collapsing resource levels 0/1 and 2/3 makes the 17 columns a strength-three binary covering array with the same row count. Corollary 2.5.21 in [Ludwig Kampel's thesis](https://repositum.tuwien.at/bitstream/20.500.12708/214325/1/Kampel%20Ludwig%20-%202024%20-%20Combinatorial%20design%20theory%20and%20applications%20for%20software...pdf) proves that this binary projection needs at least 18 rows. The remaining question is whether 18 rows can also satisfy the exact four-level pairs.

The full 18-row SAT search reached its five-minute limit. Bounded searches that tried refining binary constructions or repairing near-solutions also ended without an 18-row witness. The implementation uses the verified 19-row table, and the minimum is either 18 or 19.

## Verification

The rebuilt catalogs contain 257 item locations. A traversal probe reached the Superbunny Cave top chest as Superbunny without Pearl, placed Pearl there, and reached the chest as Link after requeueing form resets. The probe also checked that a retained Bunny still cannot place bombs with Pearl, the four marked Skull Woods hallway strats preserve Bunny, the unmarked pot-key strat rejects Superbunny, and the Light and Dark Bonk Fairies entrances connect from their respective nodes.

The following release CLI comparisons use seeds 0, 1, and 2 with the default skill preset. Both revisions placed all 257 items and 61 distinct door-specific keys on every run. Timings are illustrative wall-clock samples for the complete seed-generation commands.

| Layout | Previous attempts | Updated attempts | Previous seconds | Updated seconds |
| --- | --- | --- | --- | --- |
| Vanilla | 2, 2, 5 | 1, 2, 5 | 0.44, 0.55, 0.84 | 0.28, 0.41, 0.72 |
| Random | 2, 2, 19 | 15, 12, 2 | 0.58, 0.61, 3.37 | 2.19, 1.79, 0.63 |

The workspace test suite passed, and vanilla seed 2 also completed with the hard skill preset. The random-layout seed 2 smoke test produced a 4 MiB ROM from a base ROM with the expected SHA-256.
