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

## Super Bunny

### Definition and behavior

**Super Bunny** is the mismatch `BUNNY != 0` with the normal handler
`LINKDO = $00`. `RABBIT` is normally also nonzero, but it is not part of the
minimum definition. In this state, the normal handler supplies sword controls,
A-button actions, ordinary movement, and collisions while the `BUNNY` checks
still select Bunny graphics and impose presentation-based restrictions. In
particular, most Y-items remain blocked. It is therefore not simply “Bunny with
every Link ability”; its exact behavior is the union of normal-handler behavior
and Bunny-flag restrictions.

An action can temporarily replace `$00` with another `LINKDO` value without
clearing `BUNNY`. That suspends the defining tuple while the action runs. If the
action returns to `$00` and leaves `BUNNY` set, Super Bunny resumes.

### Entering Super Bunny

At the component level there are only two ways to form the mismatch: select the
normal handler without clearing an existing `BUNNY`, or set `BUNNY` while the
normal handler is already selected. The source contains the following concrete
routes.

1. **Recoil through a full underworld entrance with zero BG1 horizontal
   scroll.** This is the canonical stable Super Bunny setup. Enter the room in
   recoil handler `$02` while `BUNNY` is set. `CheckIfBunny` first chooses
   `$00`, then mistakenly tests direct-page `$E0` (`BG1H`) instead of long
   address `$02E0` (`BUNNY`). If `BG1H == 0`, it leaves `LINKDO = $00` without
   changing `BUNNY` (`bank_02.asm:359-372`, `bank_1C.asm:15276-15301`, and
   `symbols_wram.asm:669`):

   ```text
   if LINKDO == $02:
       LINKDO = $00
       if BG1H != 0:              # bug: should read BUNNY
           LINKDO = PEARL ? $1C : $17
   ```

   The Moon Pearl does not affect the zero-`BG1H` branch. Horizontal camera
   state, rather than Bunny state, determines whether this setup succeeds.

2. **Cross from the Dark World to the Light World.** The world-crossing handler
   selects `$00` for a Light World destination before the Link-poof ancilla
   clears `BUNNY` and `RABBIT`. A Bunny therefore has a Super-Bunny-like tuple
   during the transition (`bank_07.asm:8738-8753` and
   `bank_08.asm:16635-16690`). This is an intentional, short-lived staging
   state, not the persistent controllable glitch: completion of the poof makes
   Link normal.

3. **Finish an item-receipt state while Bunny presentation remains set.** The
   item-receipt finalizer selects `$00` (or swimming `$04` in deep water) but
   does not clear `BUNNY` (`bank_08.asm:13624-13639`). The Moon Pearl item code
   itself clears `RABBIT`, not `BUNNY` (`bank_09.asm:1415-1421`), so receiving
   the Pearl from a nonstandard Bunny-capable setup is a specific instance that
   leaves `BUNNY = 1`, `RABBIT = 0`, and `LINKDO = $00`. Ordinary Bunny controls
   do not normally let the player initiate every item-receipt path; this route
   is relevant to glitches, scripts, and modified item placement.

4. **Finish another action that unconditionally restores `$00`.** Several
   action handlers restore the default handler without reconciling `BUNNY`:
   dash completion (`bank_07.asm:3268-3292`), spin completion
   (`bank_07.asm:8390-8420`), and hookshot completion
   (`bank_07.asm:8994-9050`) are representative examples. A normal Bunny cannot
   start most of these actions, so this route requires an already active action,
   another state mismatch, or a glitch that starts the action while `BUNNY` is
   set. This is one general mechanism, not a distinct Bunny form for every
   action handler.

   For completeness, the other direct `$00` restorations that can mechanically
   have this effect if reached with `BUNNY` still set occur in special recoil
   cleanup (`bank_05.asm:17695-17703`); prayer, dash/bonk, water/pit, zap,
   medallion, carrying, and other action cleanup (`bank_07.asm:1324-1329`,
   `bank_07.asm:3413-3418`, `bank_07.asm:3631-3636`,
   `bank_07.asm:4585-4592`, `bank_07.asm:5271-5277`,
   `bank_07.asm:9190-9195`, `bank_07.asm:10334-10341`,
   `bank_07.asm:10802-10912`, and `bank_07.asm:16768-16775`); overworld/module
   cleanup (`bank_02.asm:1963-1968` and `bank_02.asm:5905-5910`); ancilla
   completion (`bank_08.asm:9248-9255`, `bank_08.asm:10822-10829`,
   `bank_08.asm:11099-11107`, `bank_08.asm:14991-14996`, and
   `bank_08.asm:18670-18675`); and isolated sprite/item cleanup
   (`bank_09.asm:4502-4509`, `bank_09.asm:9030-9037`, and
   `bank_0B.asm:16960-16967`). Most are not independently reachable from
   ordinary Bunny controls. They are listed because, once reached through an
   overlay or glitch, they all create the same component mismatch.

5. **Direct state mutation.** A debugger, practice hack, save-state edit, or
   modified game can create Super Bunny by setting `BUNNY` nonzero while
   `LINKDO` is `$00`, or by setting `LINKDO` to `$00` while retaining `BUNNY`.
   Vanilla code has no ordinary stable path that merely turns on `BUNNY` while
   leaving the default handler selected: normal world and temporary-Bunny
   transformations also select a Bunny handler.

### Exiting Super Bunny

Super Bunny ends when code reconciles either half of the mismatch. Clearing
`BUNNY` while retaining `$00` produces normal Link; selecting `$17` while
retaining `BUNNY` produces permanent Bunny. The important source paths are:

1. **Take damage and finish recoil.** Damage replaces `$00` with recoil `$02`.
   On landing, `SplashUponLanding` reads the real `BUNNY`: without the Pearl it
   restores `$17`, producing permanent Bunny; with the Pearl it chooses `$1C`
   (`bank_07.asm:3147-3181`). A zero temporary-Bunny duration then makes `$1C`
   clear the Bunny identity and return to `$00` (`bank_07.asm:678-725`). Thus
   ordinary recoil normally resolves Super Bunny instead of returning to it.

2. **Recover from a pit, ledge hop, or related displacement.** These recovery
   paths likewise derive `$00`, `$17`, or `$1C` from `BUNNY` and `PEARL`, so a
   pearlless Super Bunny normally becomes permanent Bunny and a Pearl-protected
   one is driven toward normal Link (`bank_07.asm:4004-4032` and
   `bank_07.asm:4048-4070`).

3. **Repeat the recoil entrance transition.** `CheckIfBunny` can either preserve
   or resolve the glitch. With `BG1H == 0` it chooses `$00` again and recreates
   Super Bunny. With nonzero `BG1H`, it selects permanent Bunny `$17` without a
   Pearl or temporary Bunny `$1C` with one (`bank_1C.asm:15276-15301`).

4. **Let a world transformation or form reset complete.** Crossing to the Light
   World selects `$00`, then its poof clears `BUNNY` and `RABBIT`. Loading the
   pearlless Dark World instead sets both flags and `$17`, producing ordinary
   permanent Bunny (`bank_02.asm:732-752`, `bank_07.asm:8738-8753`, and
   `bank_08.asm:16635-16690`). When the Pearl is owned,
   `AdjustLinkBunnyStatus` clears the handler, both form flags, and temporary
   timers, producing normal Link (`bank_02.asm:795-814`).

5. **Run another explicit Bunny reset.** For example, `CheckAbilityToSwim`
   clears `BUNNY` when the Pearl is present (`bank_01.asm:23646-23664`), and
   death preparation clears immediate Bunny presentation and temporary-form
   data (`bank_1C.asm:15029-15054`). Whether Link is immediately controllable
   afterward depends on the surrounding transition, but the Super Bunny tuple
   itself is gone.

6. **Directly reconcile the components.** Clearing `BUNNY` yields normal Link;
   changing `LINKDO` to `$17` yields permanent Bunny. Direct RAM edits and
   modified code can do either. Merely entering a temporary action handler is
   not necessarily an exit: if its finalizer restores `$00` without clearing
   `BUNNY`, Super Bunny returns.

## Lonk

### Definition and behavior

**Lonk** is the inverse mismatch: `BUNNY = 0` with permanent-Bunny handler
`LINKDO = $17`. `RABBIT` may be either zero or nonzero. The `$17` handler omits
the normal sword and A-button paths, but it still calls `HandleYItem`. Because
that item routine checks `BUNNY` rather than `LINKDO`, most Y-items are allowed
for Lonk. Drawing also checks `BUNNY`, so Lonk normally has Link graphics,
although room palette code can still select Bunny colors when a retained
`RABBIT` is nonzero. Animation stepping separately recognizes `$17`, creating
another mixture of Link presentation and Bunny-handler behavior.

The temporary-Lonk analogue, `BUNNY = 0` with `LINKDO = $1C`, is listed
separately in the state enumeration. It falls into the Bunny handler while its
timer is nonzero, but it is not Lonk because its primary handler is temporary
Bunny rather than permanent Bunny.

### Entering Lonk

At the component level, Lonk forms whenever code selects `$17` without setting
`BUNNY`, or clears `BUNNY` without replacing an existing `$17`. The
source-visible routes are:

1. **Recoil through a full underworld entrance with nonzero BG1 horizontal
   scroll and no Moon Pearl.** This is the canonical stable setup. Enter in
   recoil `$02` with `BUNNY = 0`. Because `CheckIfBunny` reads `BG1H` by
   mistake, nonzero `BG1H` selects `$17`; lack of the Pearl prevents the later
   `$1C` selection. The routine never sets `BUNNY`, leaving controllable Lonk
   (`bank_02.asm:359-372` and `bank_1C.asm:15276-15301`). If the Pearl is owned,
   the same setup selects `$1C` and makes the temporary-Lonk analogue instead.

2. **Cross into the Dark World without the Moon Pearl.** The world-crossing
   handler selects `$17` before the Link-poof ancilla sets `BUNNY` and `RABBIT`
   (`bank_07.asm:8738-8753` and `bank_08.asm:16635-16690`). This produces a
   brief, intentional Lonk-like tuple during the transition. When the poof
   completes, `BUNNY` becomes one and the state becomes ordinary permanent
   Bunny; it is not the persistent room-entry glitch.

3. **Recover from a damaging pit while only backup Bunny identity remains.**
   `ResetStateAfterDamagingPit` consults `RABBIT`, not `BUNNY`. With
   `RABBIT != 0`, no Pearl, and a preexisting `BUNNY = 0` mismatch, it selects
   `$17` without restoring `BUNNY`, producing or preserving Lonk
   (`bank_07.asm:5018-5060`). Normal permanent Bunny has both flags set, so the
   pit routine does not create Lonk from an otherwise consistent state.

4. **Clear presentation during an existing `$17` state.** `PrepareToDie`, for
   example, clears `BUNNY` and retains `RABBIT` when the Pearl is absent
   (`bank_1C.asm:15029-15054`). That creates the Lonk tuple during the death
   sequence, but not a normal controllable Lonk because the death module owns
   control. Other scripted or modified paths that clear `$02E0` without
   replacing `$5D` have the same component-level effect.

5. **Direct state mutation.** A debugger, practice hack, save-state edit, or
   modified game can set `LINKDO = $17` while leaving `BUNNY = 0`, or clear
   `BUNNY` while retaining `$17`.

### Exiting Lonk

Lonk ends when `LINKDO` ceases to be `$17` or `BUNNY` becomes nonzero. Some
transitions only suspend it; a plain room or screen transition that changes
neither component can preserve it.

1. **Take damage and finish recoil.** The Bunny handler routes damage through
   `LinkState_Bunny_recache`. Without the Pearl, that routine selects recoil
   `$02` but does not set `BUNNY`; landing then sees `BUNNY = 0` and selects
   `$00`, producing normal Link (`bank_07.asm:725-787` and
   `bank_07.asm:3147-3181`). With the Pearl, the recache path clears retained
   Bunny identity and selects `$00` directly.

2. **Enter deep water or complete a displacement recovery.** Deep water also
   routes the `$17` handler through `LinkState_Bunny_recache`. Ordinary landing,
   pit, and ledge-hop recovery paths that inspect `BUNNY` see zero and select
   `$00` (`bank_07.asm:3147-3181`, `bank_07.asm:4004-4032`, and
   `bank_07.asm:4048-4070`). The damaging-pit routine is the exception: if
   `RABBIT` is still set and the Pearl is absent, it can restore `$17` and
   preserve Lonk.

3. **Use an allowed Y-item that owns `LINKDO`.** Lonk's Bunny handler calls
   `HandleYItem`, and `BUNNY = 0` bypasses the normal Bunny item block. An item
   such as the Hookshot can replace `$17` with its action state; Hookshot
   completion restores `$00` without setting `BUNNY`, leaving normal Link
   (`bank_07.asm:5610-5637` and `bank_07.asm:8994-9050`). Items that do not
   replace or ultimately change `LINKDO` do not by themselves exit Lonk.

4. **Cross worlds or reload a form-authoritative context.** Crossing to the
   Light World selects `$00` and the poof clears the form flags. A pearlless
   Dark World load or completion of a Dark World poof sets `BUNNY` and
   `RABBIT`, converting `$17` Lonk into ordinary permanent Bunny
   (`bank_02.asm:732-752`, `bank_07.asm:8738-8753`, and
   `bank_08.asm:16635-16690`). With the Pearl, `AdjustLinkBunnyStatus` forces
   `$00` and clears all Bunny state (`bank_02.asm:795-814`).

5. **Repeat the recoil entrance transition.** Once Lonk is first moved into
   recoil `$02`, entering a full room with `BG1H == 0` selects `$00` and resolves
   it to normal Link. With nonzero `BG1H` and no Pearl, the bug selects `$17`
   again and recreates Lonk (`bank_1C.asm:15276-15301`). Entering a room while
   still in `$17` does not invoke this decision, because `CheckIfBunny` only
   acts on recoil `$02`.

6. **Directly reconcile the components.** Setting `BUNNY` makes the tuple
   ordinary permanent Bunny; setting `LINKDO = $00` makes it normal Link.
   Temporary action handlers are not necessarily permanent exits: the final
   handler and the value of `BUNNY` determine what remains when the action ends.

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
