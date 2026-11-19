# Study and Calendar feature matrix

| Feature | Delivery |
| --- | --- |
| Calendar | Day, week, and month with selected-day agenda. Create, edit, move by date, duplicate and delete tasks. Subject, priority, estimate, optional clock time and links. Daily/weekly occurrence completion and skip exceptions. Subtasks have per-occurrence checkboxes independent of the parent task. An overdue list links to the original date for incomplete one-off tasks. |
| Focus | Full-page and optional Notes timer share the native vault. Presets, custom duration, stopwatch, manual breaks, pause, resume, finish and links. Optional automatic focus and break rounds share the same session engine. The active segment uses monotonic time while mounted, with a UTC wall-time baseline after reopening. Advancing after suspension caps the elapsed phase and starts the next when observed. |
| Analytics | Completed intervals split on device-local date boundaries. Weekly daily and subject totals, break totals, and weekly goal. Finishing a session leaves task completion unchanged. |
| Exam and progress | Main, Main+Advanced, NEET, target year, preserved switching, personal unit/topic groups, five progress states, weak flag, revision task and Focus linkage. |
| Official seed and countdown | Official source retrieval returned HTTP 403. No official topics or dates are seeded. Personal exam dates are explicitly labeled **Custom date**, use India civil-day countdowns, and appear in the active track’s Calendar. See [source audit](OFFICIAL_SOURCES.md). |
| Follow-on | Complex recurrence rule editing, native reminders, manual time correction/audit, idle detection, Focus wallpaper and sound, exam crosswalk and officially sourced version migration. |
