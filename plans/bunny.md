# Bunny state

The game does not represent Bunny as one state. It splits the concept across an
appearance flag, a persistent form flag, Link's current action handler, and a
temporary-form timer. Most ordinary code keeps them consistent, but transitions
and glitches can separate them. In particular:

- **Super Bunny** is Bunny appearance with Link's normal handler.
- **Lonk** is Link appearance with the permanent-Bunny handler.

This document describes the original SNES code represented by this disassembly.
Addresses below are WRAM unless otherwise noted.

## State components

| Symbol | Address | Meaning |
| --- | --- | --- |
| `RABBIT` | `$7E0056` | Persistent/backup Bunny-form flag. Usually equals `BUNNY`, but survives some resets so Bunny can be restored later. |
| `LINKDO` | `$7E005D` | Link's current action-handler index. `$00` is default Link, `$17` is permanent Bunny, and `$1C` is temporary Bunny. Many unrelated actions temporarily replace it. |
| `BUNNY` | `$7E02E0` | Immediate Bunny presentation/restriction flag. It selects Bunny graphics and is consulted by item, swimming, shield, carrying, and other checks. |
| `POOFING` | `$7E02E1` | Set while the Link/cape poof ancilla is active. |
| `POOFTIME` | `$7E02E2` | Poof countdown used during form changes. |
| `TEMPBUNTM` | `$7E03F5` | Low byte of the temporary-Bunny duration. |
| `TEMPBUNTMH` | `$7E03F6` | High byte of the temporary-Bunny duration. |
| `TEMPBUN` | `$7E03F7` | Latch indicating that the incoming temporary-Bunny poof has begun. It is not the duration itself. |
| `PEARL` | `$7EF357` | Saved Moon Pearl ownership. It determines whether world-induced Bunny is required and whether a Bunny state should be permanent or temporary. |

The symbol definitions are in `symbols_wram.asm:312-316`,
`symbols_wram.asm:1164-1174`, `symbols_wram.asm:1919-1927`, and
`symbols_sram.asm:672`.

### Values of `RABBIT`

| Value | Meaning |
| ---: | --- |
| `$00` | No persistent Bunny identity. |
| `$01` | Bunny identity is present. This is the only nonzero value written by normal code. |
| `$02-$FF` | Not normally written, but readers treat any nonzero value like `$01`. |

### Values of `LINKDO`

`LINKDO` dispatches through the table at `bank_07.asm:43-106`:

| Value | Handler | Meaning |
| ---: | --- | --- |
| `$00` | `LinkState_Default` | Ordinary Link control. |
| `$01` | `LinkState_Pits` | Slipping into or falling through a pit. |
| `$02` | `LinkState_Recoil` | Normal recoil/knockback. `CheckIfBunny` specifically examines this value on room entry. |
| `$03` | `LinkState_SpinAttack` | Spin attack. |
| `$04` | `LinkState_Swimming` | Swimming. |
| `$05` | `LinkState_OnIce` | Ice movement. |
| `$06` | `LinkState_Recoil` | Alternate recoil/jumping case; dispatches to the same handler as `$02`. |
| `$07` | `LinkState_Zapped` | Electrocution/zap animation. |
| `$08` | `LinkState_UsingEther` | Casting Ether. |
| `$09` | `LinkState_UsingBombos` | Casting Bombos. |
| `$0A` | `LinkState_UsingQuake` | Casting Quake. |
| `$0B` | `LinkState_HoppingSouthOW` | Southward overworld hop. |
| `$0C` | `LinkState_HoppingHorizontallyOW` | Horizontal overworld hop. |
| `$0D` | `LinkState_HoppingDiagonallyUpOW` | Diagonal upward overworld hop. |
| `$0E` | `LinkState_HoppingDiagonallyDownOW` | Diagonal downward overworld hop. |
| `$0F` | `LinkState_0F` | First special horizontal-ledge hop variant. |
| `$10` | `LinkState_0F` | Second special horizontal-ledge hop variant; dispatches to the same handler as `$0F`. |
| `$11` | `LinkState_Dashing` | Pegasus Boots dash. |
| `$12` | `LinkState_ExitingDash` | Dash exit/bonk recovery. |
| `$13` | `LinkState_Hookshotting` | Hookshot movement. |
| `$14` | `LinkState_CrossingWorlds` | Mirror/portal world crossing. |
| `$15` | `LinkState_ShowingOffItem` | Item-receipt pose; the handler itself only returns. |
| `$16` | `LinkState_Sleeping` | Sleeping/bed state. |
| `$17` | `LinkState_Bunny` | Permanent-Bunny control handler. This does not by itself guarantee Bunny graphics. |
| `$18` | `LinkState_HoldingBigRock` | Holding a large rock. |
| `$19` | `LinkState_ReceivingEther` | Receiving the Ether medallion. |
| `$1A` | `LinkState_ReceivingBombos` | Receiving the Bombos medallion. |
| `$1B` | `LinkState_ReadingDesertTablet` | Reading the Desert tablet. |
| `$1C` | `LinkState_TemporaryBunny` | Temporary-Bunny timer handler, which falls through into `LinkState_Bunny` while time remains. |
| `$1D` | `LinkState_TreePull` | Pulling a tree object. |
| `$1E` | `LinkState_SpinAttack` | Alternate spin-attack entry; dispatches to the same handler as `$03`. |
| `$1F-$FF` | None | Invalid. There is no bounds check before the indexed jump, so these values read a vector past the table and have undefined, potentially crashing behavior. |

The `$0F` and `$10` meanings follow their writers in
`CheckForWeirdLedges_Horizontal` (`bank_07.asm:15366-15385`); the disassembly has
no more specific names for them.

### Values of `BUNNY`

| Value | Meaning |
| ---: | --- |
| `$00` | Use Link presentation and allow checks gated on not being Bunny. |
| `$01` | Use Bunny presentation and restrictions. This is the only nonzero value written by normal code. |
| `$02-$FF` | Not normally written, but readers treat any nonzero value like `$01`. |

### Values of `POOFING`

| Value | Meaning |
| ---: | --- |
| `$00` | No Link/cape poof is active. |
| `$01` | A cape-style Link poof ancilla is active. Normal code sets this value in `AncillaAdd_CapePoof` and clears it when the ancilla finishes. |
| `$02-$FF` | Not normally written, but zero/nonzero tests treat these values as active. |

See `bank_09.asm:3693-3710` and `bank_08.asm:16635-16655`.

### Values of `POOFTIME`

`POOFTIME` is an 8-bit shared cape/Bunny countdown, so its meaning also depends
on `TEMPBUN`, cape state, and the routine currently decrementing it.

| Value | Meaning |
| ---: | --- |
| `$00` | Cleared/idle outside a transition; when a countdown is active, this is its final nonnegative frame. |
| `$01-$13` | Remaining incoming-poof wait after an initial `$14`. |
| `$14` | Initial incoming-poof value used when enabling the cape or starting temporary Bunny. |
| `$15-$1F` | Values reached when an outgoing/cooldown initialization of `$20` is actively decremented. |
| `$20` | Initial outgoing-poof/cooldown value used by temporary-Bunny reversion and forced cape removal. |
| `$21-$7F` | Not normally initialized. If encountered by a decrementing path, acts as an extended positive countdown. |
| `$80-$FE` | Not normally produced by the defined countdowns and has no stable meaning; signed branch tests make behavior context-dependent. |
| `$FF` | Result of decrementing `$00`; the incoming temporary-Bunny path treats this as completion and leaves it in the field after committing state `$1C`. |

The defined initial writes and countdowns are at `bank_07.asm:627-646`,
`bank_07.asm:689-706`, and `bank_07.asm:9447-9560`.

### Values of `TEMPBUNTM` and `TEMPBUNTMH`

These are not independent states. Together they are one little-endian 16-bit
countdown, `TEMPBUNTMH:TEMPBUNTM`.

| Combined value | Meaning |
| ---: | --- |
| `$0000` | No temporary transformation is pending, or the active duration has expired. |
| `$0001-$00FF` | Remaining active temporary-Bunny handler frames. |
| `$0100` | Full duration written by a Bunny Beam hit. |
| `$0101-$FFFF` | Not written by normal Bunny code. Consumers give these values no distinct meaning: they are nonzero, and state `$1C` counts them down as a 16-bit word. |

Viewed separately, `TEMPBUNTM` can take every value `$00-$FF` as the low byte
counts down. `TEMPBUNTMH` is normally `$01` only at the initial `$0100`, then
`$00`; `$02-$FF` have no distinct meaning beyond supplying extra high-order
countdown time. The write is at `bank_1D.asm:1100-1106`, and the 16-bit decrement
is at `bank_07.asm:713-721`.

### Values of `TEMPBUN`

| Value | Meaning |
| ---: | --- |
| `$00` | The incoming temporary-Bunny poof has not started, or has finished. With a nonzero duration, an eligible caller of `HandleBunnyTransformation` begins the poof. |
| `$01` | Incoming poof is in progress. This is the only nonzero value written by normal code. |
| `$02-$FF` | Not normally written, but the `BNE` test treats any nonzero value like `$01`. |

### Values of `PEARL`

| Value | Meaning |
| ---: | --- |
| `$00` | Moon Pearl not owned. World-induced Bunny is required in the Dark World. |
| `$01` | Moon Pearl owned. This is the normal saved item value. |
| `$02-$FF` | Not normally written for this item, but ownership checks treat any nonzero value as owned. |

`RABBIT` and `BUNNY` are both Boolean in ordinary play, although the code
generally tests only zero versus nonzero.

## Complete enumeration of Bunny-like states

This is the complete set of meaningful classes admitted by the form fields.
Arbitrary RAM corruption can of course produce other byte values, but the game
does not give those combinations distinct semantics.

| State | `BUNNY` | `LINKDO` | Typical `RABBIT` | Timer | What it means |
| --- | ---: | ---: | ---: | ---: | --- |
| Normal Link (baseline) | `0` | `$00` | `0` | `0` | Link graphics and normal controls. Included to make the mismatches clear. |
| Permanent Bunny | `1` | `$17` | `1` | normally `0` | Normal pearlless Dark World Bunny. Bunny graphics and Bunny controls. |
| Temporary Bunny | `1` | `$1C` | `1` | nonzero | Moon-Pearl Link transformed by a Bunny Beam. It uses the Bunny movement/control body while the 16-bit timer counts down. |
| Super Bunny | `1` | normally `$00` | `0` or `1` | usually `0` | Bunny graphics/restrictions with the normal Link handler. Normal-handler actions such as sword and A-button interaction become available, but checks that read `BUNNY` still treat the player as Bunny. |
| Lonk | `0` | `$17` | normally `0` | normally `0` | Link presentation and presentation-based permissions with the permanent-Bunny handler. The Bunny handler omits normal A-button and sword processing, but its Y-item call sees `BUNNY == 0`. |
| Temporary-Lonk analogue | `0` | `$1C` | normally `0` | zero or nonzero | Link graphics under the temporary-Bunny handler. `CheckIfBunny` can select this when the Moon Pearl is owned even though `BUNNY` is clear. With a zero timer it immediately reverts; with a nonzero timer it can persist until expiry. This has no separate name in the source. |
| Intentional poof mismatch | changing | `$00`, `$11`, `$14`, or `$17` | changing | varies | During a world or Bunny-Beam poof, the handler and graphics flag are updated at different times. The tuple can briefly look like Super Bunny or Lonk, but control is hidden/frozen by the transition. |
| Bunny action overlay | usually `1` | another handler, such as recoil `$02` or world crossing `$14` | usually `1` | varies | Bunny form remains active while an unrelated action owns `LINKDO`. Recovery code is expected to reconstruct `$17` or `$1C`. This is not a separate form. |
| Dormant Bunny identity | `0` | death/revival handling | `1` | `0` | Death clears immediate Bunny presentation but can retain `RABBIT`; fairy revival uses it to restore `BUNNY = 1` and `LINKDO = $17`. This is storage state, not controllable Bunny. |

The names **Super Bunny** and **Lonk** describe opposite mismatches, not unique
flag bytes. A useful diagnostic rule is:

```text
appearance: BUNNY != 0 ? Bunny : Link
control:    LINKDO == $17/$1C ? Bunny-family : current non-Bunny action
```

`RABBIT` answers a different question: whether Bunny identity should survive a
reset or revival. It is not sufficient to determine either current appearance
or current controls.

## Permanent world-induced Bunny

### Entering

An overworld load checks Moon Pearl ownership and the saved world. If Link has no
Pearl and the destination is the Dark World, it sets all three ordinary Bunny
components:

```text
BUNNY  = 1
RABBIT = 1
LINKDO = $17
```

It also loads the Bunny equipment palette (`bank_02.asm:732-752`).

Mirror/world crossing is split across two routines. `LinkState_CrossingWorlds`
chooses `$17` when the destination screen has Dark World bit `$40` and Link has
no Pearl (`bank_07.asm:8738-8753`). The poof ancilla later sets `BUNNY` and
`RABBIT` from that same screen bit and refreshes the appropriate palette
(`bank_08.asm:16635-16690`). Because those writes occur at different times, a
world poof deliberately passes through a short mixed state.

### Leaving or obtaining protection

`AdjustLinkBunnyStatus` clears `LINKDO`, both timer bytes, `TEMPBUN`, `RABBIT`,
and `BUNNY` when the Moon Pearl is owned (`bank_02.asm:795-814`). It is called
during overworld loading and game-over recovery.

The Moon Pearl item-receipt code itself only clears `RABBIT`
(`bank_09.asm:1415-1421`). It does not clear `BUNNY`. In a nonstandard setup
where the Pearl is received while Bunny presentation is active, the item-receipt
ancilla later restores `LINKDO = $00`, or swimming `$04` if the deep-water flag
is set (`bank_08.asm:13624-13639`). The usual `$00` result leaves a Super-Bunny
combination until another reconciliation path, such as overworld loading or
`CheckAbilityToSwim`, clears `BUNNY`.

## Temporary Bunny

### Bunny Beam hit

The active Bunny Beam calls the ordinary Link-damage collision helper. On a hit
it deletes itself and writes the 16-bit duration as `$0100`:

```text
TEMPBUNTMH:TEMPBUNTM = $0100
```

See `BunnyBeam` at `bank_1D.asm:1028-1110`. The stray `LDA #$80` before
`STZ $03F5` has no effect.

The default and dashing handlers call `HandleBunnyTransformation`
(`bank_07.asm:230` and `bank_07.asm:3261`). When the duration is nonzero and
`TEMPBUN` is clear, that routine:

1. Cancels carrying, active properties, selected ancillae, and dashing.
2. Starts a poof and sound.
3. Sets `POOFTIME = $14`, damage immunity, and `TEMPBUN = 1`.
4. Returns carry set so the caller does no normal Link processing during the poof.

`ResetLinkProperties_A` clears `BUNNY`, `RABBIT`, and the low timer byte but not
the high timer byte (`bank_07.asm:23222-23239`). Thus the initial `$0100`
duration survives the reset.

The same call and later calls decrement `POOFTIME`. After it goes negative, the
routine commits the temporary form:

```text
LINKDO = $1C
BUNNY  = 1
RABBIT = 1
TEMPBUN = 0
```

and loads the Bunny palette (`bank_07.asm:567-677`).

### Active duration and expiry

`LinkState_TemporaryBunny` tests the 16-bit duration. While it is nonzero, it
decrements the word at `$03F5` and falls directly into `LinkState_Bunny`, so the
temporary form shares the Bunny movement/control body (`bank_07.asm:678-725`).
The `$0100` count is 256 handler frames, about 4.27 seconds at 60 Hz, after the
incoming poof.

When the timer is zero, the state starts the return poof, restores
`LINKDO = $00`, clears `RABBIT`, `BUNNY`, and `TEMPBUN`, reloads Link's normal
palette, and immediately branches into `LinkState_Default`. The outgoing poof
is therefore visual; normal state is restored without waiting for it to finish.

### Early cancellation

`LinkState_Bunny_recache` is the central cancellation/reconciliation path. It
always clears `TEMPBUN` and both timer bytes. If the Moon Pearl is owned, it also
clears `RABBIT`, selects `LINKDO = $00`, and reloads Link's palette. Without the
Pearl it retains Bunny identity and selects recoil `$02`, whose landing recovery
will reconstruct permanent Bunny (`bank_07.asm:725-787`).

Temporary Bunny reaches this path when damaged or forced into deep water. Pit
and spike handling also clear the temporary timer and, when the Pearl is owned,
clear both form flags (`bank_07.asm:4083-4101` and `bank_07.asm:16878-16890`).

## Controls, graphics, and restrictions

### Handler-controlled behavior

`LinkState_Default` handles A-button actions, tossing, sword controls, items,
movement, and collisions. `LinkState_Bunny` is much smaller: it handles recoil,
movement/collision, animation, and `HandleYItem`, but never runs the ordinary
A-button or sword paths (`bank_07.asm:220-566` versus `bank_07.asm:725-824`).
This handler split is what gives Super Bunny extra normal-Link actions and makes
Lonk lose them.

The Bunny handler still calls `HandleYItem`. That routine independently checks
`BUNNY`: when it is set, all selected items except bottle `$0B` and mirror `$14`
return immediately (`bank_07.asm:5610-5637`). Consequently:

- Permanent and temporary Bunny can reach only the bottle/mirror exceptions.
- Super Bunny gains normal-handler sword/A behavior, but still fails most
  Y-item checks.
- Lonk lacks normal-handler sword/A behavior, but its Y-item call is not blocked
  by Bunny presentation.

### Presentation-controlled behavior

The drawing code tests `BUNNY` and selects action-animation set `$21` for Bunny
graphics (`bank_0D.asm:3529-3547`). Palette refresh uses fixed Bunny mail palette
`$0303` while retaining the equipped sword palette (`bank_02.asm:21904-21915`).

Animation stepping is not keyed identically: the full animation handler uses
the special four-step Bunny animation only when `LINKDO == $17`
(`bank_07.asm:21138-21143` and `bank_07.asm:21284-21331`). State `$1C` falls
through the Bunny control body but uses the general animation-step logic. This
is another example of the form not being centralized.

Other important checks read `BUNNY`, not `LINKDO`:

- `CheckAbilityToSwim` treats Bunny as unable to swim and clears `BUNNY` if the
  Pearl is now owned (`bank_01.asm:23646-23664`).
- Water-entry code refuses to enter swimming state while `BUNNY` is set
  (`bank_07.asm:13141-13149` and `bank_07.asm:15190-15198`).
- Carried sprites are forcibly thrown (`bank_06.asm:18636-18649`).
- Bunny presentation bypasses shield-blocking and takes the hit
  (`bank_06.asm:22476-22489`).
- Kiki will not activate (`bank_1E.asm:17548-17559`).

Room palette loading checks `BUNNY | RABBIT`, so retained Bunny identity can
keep the Bunny palette even when immediate presentation was cleared
(`bank_02.asm:267-275`).

## Lonk and the room-entry bug

`CheckIfBunny` runs during a full underworld room load, immediately after
`ResetLampconeAndLink` (`bank_02.asm:359-372`). It only acts when Link entered in
recoil handler `$02`. Its intended discriminator is `BUNNY`, but the code reads
direct page `$E0`, which is `BG1H`, the low byte of the BG1 horizontal scroll:

```text
if LINKDO == $02:
    LINKDO = $00
    if BG1H != 0:                 # bug: should read BUNNY
        LINKDO = PEARL ? $1C : $17
```

The implementation and its explicit Lonk comment are at
`bank_1C.asm:15276-15301`; `BG1H` is defined at `symbols_wram.asm:669`.

The outcomes cover three glitched Bunny-like classes:

1. Normal Link, no Pearl, `BG1H != 0`: `BUNNY` remains zero but `LINKDO` becomes
   `$17` — **Lonk**.
2. Bunny presentation, `BG1H == 0`: `LINKDO` becomes `$00` while `BUNNY`
   remains set — **Super Bunny**.
3. Normal Link with the Pearl, `BG1H != 0`: `LINKDO` becomes `$1C` while
   `BUNNY` remains zero — the temporary-Lonk analogue. A zero duration makes it
   revert immediately.

Because `BG1H` is camera state, the same recoil-through-entrance idea can choose
different handlers according to horizontal scroll rather than actual form.

## Recovery after temporary action states

Normal play frequently puts a Bunny into a non-Bunny `LINKDO` action such as
recoil, pit handling, or world crossing. Recovery is decentralized:

- `SplashUponLanding` uses `BUNNY` to choose `$17`; with a Pearl it chooses
  `$1C` instead (`bank_07.asm:3147-3181`).
- Dash/pit recovery similarly chooses `$00`, `$17`, or `$1C` from `BUNNY` and
  `PEARL` (`bank_07.asm:4004-4032` and `bank_07.asm:4048-4070`).
- Recovery from a damaging pit uses `RABBIT` plus lack of Pearl to restore
  `$17` (`bank_07.asm:5033-5052`).
- Fairy revival uses `RABBIT` to restore `$17` and forces `BUNNY = 1`
  (`bank_08.asm:22957-22982`).

This is why neither handler nor appearance alone is authoritative. Correct
recovery depends on which routine runs and which of `BUNNY`, `RABBIT`, and
`PEARL` it happens to consult.

## Death and revival

`PrepareToDie` always clears `BUNNY`, `TEMPBUN`, and both timer bytes. It clears
`RABBIT` only if the Moon Pearl is owned (`bank_1C.asm:15029-15054`). Therefore a
pearlless permanent Bunny dies with a dormant `RABBIT = 1` identity.

If a bottled fairy revives Link, `RevivalFairy_MonitorHP` checks `RABBIT`; when
set, it restores `LINKDO = $17` and `BUNNY = 1`. Otherwise it restores default
Link or swimming state (`bank_08.asm:22942-22982`). Game-over recovery also runs
the Moon-Pearl reconciliation routines (`bank_09.asm:21430-21449`).

## Practical invariants

- `BUNNY` determines what is drawn and many immediate restrictions.
- `LINKDO` determines which control routine executes this frame.
- `RABBIT` preserves Bunny identity across destructive state changes.
- `TEMPBUNTMH:TEMPBUNTM` determines remaining temporary duration, but a nonzero
  timer may also mean the incoming poof is pending.
- `TEMPBUN` distinguishes the incoming-poof phase from an unstarted Beam timer.
- `PEARL` decides whether reconstructed Bunny should be permanent, temporary,
  or removed.
- Equality of `RABBIT` and `BUNNY` is common, not guaranteed.
- `LINKDO == $17` does not prove Bunny graphics (Lonk), and `BUNNY != 0` does
  not prove Bunny controls (Super Bunny or an action overlay).
