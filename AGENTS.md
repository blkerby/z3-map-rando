# General guidelines

- If anything is unclear, please ask the user for clarification rather than
  guessing.
- Interpret the Ponytail approach as aiming to minimize the complexity of the
  project after applying a change, not minimizing the complexity of a change
  itself.
- Please apply the Ponytail approach to Markdown docs too: keep things as simple
  as possible.
- Ask for confirmation before diving into making changes, unless the exact
  desired code change is already clear from the user's request. This includes
  follow-up debugging after a reported regression: investigate and explain the
  likely cause first, but do not apply a speculative fix or design change until
  the user confirms that specific change.

# Rust style guidelines

- Do not add permanent tests or validation unless the user explicitly requests
  them. This includes `assert!`, `debug_assert!`, `ensure!`, validation branches,
  schema checks, invariant checks, and new error paths, even when similar checks
  already exist nearby or the input comes from a file. Checks added temporarily
  while developing must be removed before completing the task. If a permanent
  check appears necessary for security or preventing data loss, ask the user
  before adding it.
- Avoid iterator chains that combine operations such as mapping, filtering, and
  collecting. Prefer loops instead. Simple one-liners like
  `.into_iter().collect()` are ok.
- Avoid creating helper functions that are only used in one place or which would
  be simpler to inline.
- Likewise, avoid creating "wrapper" functions which do nothing other than
  call out to one other function.
- Avoid casts or `from`-conversions of `bool` to integer types. Use `if-else`
  instead.
- Function names should start with a verb.

# ASM guidelines

- In hooked code, always check all used registers and scratch RAM to see if they
  are free or if they need to be saved/restored.
- Be careful with stack-affecting operations such as PHA/PLA, JSR/RTS, JSL/RTL:
  make sure they are properly balanced under all possible control flow paths.

## Style

- Name Asar free-space bounds `free_space_bank_<bank>_start` and
  `free_space_bank_<bank>_end`, with an explicit lowercase hexadecimal bank
  such as `free_space_bank_a0_start`. Number multiple ranges as
  `start_1`/`end_1`, `start_2`/`end_2`, and so on. Prefer to use fewer
  free-space blocks, using just one when possible. Prefer bank `$A0` for new
  code. For vanilla banks, prefer not to reuse the space
  freed by obsoleted routines; instead, prefer doing JML to a non-vanilla bank
  (e.g. `$A0`). Short jumps/calls can be used if it helps speed up a hook in a
  performance-sensitive part of the code (such as the NMI handler).
- Anchor every code address exported through `symbols.inc` in its defining
  patch, and assert that the defining label equals the exported symbol. If the
  export starts a free-space block, keep the block's start address independent,
  use `org !free_space_bank_<bank>_start`, and then assert the label equality.
  Before an export within a block, use `assert pc() <= !ExportedRoutine`
  followed by `org !ExportedRoutine`; use `==` instead of `<=` when fallthrough
  requires no gap. Retain the final free-space end assertion so the anchors
  form a monotonic chain from the independently declared start through every
  exported entry to the end. A direct replacement in vanilla space may use
  `org !ExportedRoutine` directly.
- Where a control transfer is injected into vanilla code, use a label of the
  form `hook_<point>` after the Asar `org`, where `<point>` describes the
  process being interrupted, not the displaced instructions specifically. Keep
  generic routines not tied to one hook point action-named, even when a hook
  calls them directly. When a hook runs multiple instructions displaced at its
  `org` site, precede each such block with `; Run hi-jacked instructions:` and
  separate the block from other instructions with blank lines. For a single
  displaced instruction, append `; Run hi-jacked instruction` to that
  instruction instead.
- When patching vanilla code, do not use more than 3 NOP instructions in a row.
  Instead use BRA.
- For labels that only identify where patched code returns to existing vanilla
  code, use `return_<point>`, where `<point>` describes the point or process
  being returned to.
