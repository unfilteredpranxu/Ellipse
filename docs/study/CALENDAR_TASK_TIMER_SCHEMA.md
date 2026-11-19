# Calendar and Focus integration decision

Planner fields live in the native Notes Vault snapshot. Calendar, Focus, Study and the Notes mini-timer share the mounted vault. Task and topic IDs are local to the planner. A recurring occurrence is identified by its task ID and civil date. Completion and skip records are specific to occurrences. Each checklist item has a stable task-local ID. The civil date and item ID identify its checked state. Editing a task keeps checks for retained IDs. Duplicating a task starts a fresh checklist. Device-local dates drive calendar views, while session intervals remain UTC epoch milliseconds.

Automatic cycles persist their configured phase lengths and remaining rounds alongside the active session. The shared foreground scheduler records a completed phase at its capped elapsed-time boundary and starts the next phase when the app observes the transition. Periods without an observer produce no additional rounds or break records. Focus, Notes and Calendar read the same logged intervals.

Personal exam targets are keyed by exam and year. The selected track filters the Calendar markers and Study cards. Countdown labels use Asia/Kolkata civil days.

The signed-in website uses separate browser-local planner storage keyed by Firebase UID. Native Notes and media never become accessible to browser accounts. Browser records retain the same storage key. The reader accepts versions 1 and 2 and new writes use version 2. Native Vault schema 3 adds these fields and continues to read schemas 1 and 2. A native vault backup includes planner fields automatically. Existing archives use defaults for missing fields.
