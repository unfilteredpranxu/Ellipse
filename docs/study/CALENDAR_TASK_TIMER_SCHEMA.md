# Calendar and Focus integration decision

Planner fields live in the native Notes Vault snapshot. Calendar, Focus, Study and the Notes mini-timer share the mounted vault. Task and topic IDs are local to the planner. A recurring occurrence is identified by its task ID and civil date. Completion and skip records are specific to occurrences. Device-local dates drive calendar views, while session intervals remain UTC epoch milliseconds.

The signed-in website uses separate browser-local planner storage keyed by Firebase UID. Native Notes and media never become accessible to browser accounts. A native vault backup includes planner fields automatically. Existing archives use default fields when opening in the current app.
