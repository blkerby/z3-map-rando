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
- Unless the user specifies otherwise, any tests added to verify changes should
  be treated as temporary and removed before completing the task.
- Name Asar free-space bounds `free_space_bank_<bank>_start` and
  `free_space_bank_<bank>_end`, using `bank_any` for relocatable blocks. Number
  multiple ranges as `start_1`/`end_1`, `start_2`/`end_2`, and so on.
