# Guest mode

Guest mode lets someone else train on your RiverLab (on your phone, say) without changing your
statistics. Turn it on with **Guest mode** in the top bar; a banner stays at the top of every
page until you press **End guest mode**.

The switch is per device and browser (stored in `localStorage`). While it is on, the web app
sends `X-RiverLab-Guest: 1` with every API request (`src/lib/api.ts`, forwarded by
`src/lib/proxy.ts`), and the backend (`riverlab_api/api/guest.py`) behaves as follows.

| Area | In guest mode |
|---|---|
| Academy questions | Graded and explained as usual; the attempt is **not stored** (no revision schedule, no study count, no streak day). |
| Academy progress | The lesson's place, level and completion are returned but **not saved**. |
| Bookmarks | Locked (403 `guest_mode`). |
| Push/fold and postflop drills | Graded as usual; the answer is **not stored** (no Analytics, no study count). |
| Play against bots | Games are played and stored with `play_sessions.tracked = false`, so a guest can replay their hands and ask for a solver review of one. They never count towards the hands goal, streak, Analytics, weaknesses, batch reviews or solver leaks. |
| Game lists (Play history, Hand Analysis) | Show guest games only; your games show only with guest mode off. |
| Deleting games | A guest can delete only guest games. |
| Batch solver reviews | Locked: they look for leaks in your hands. |
| Daily goals | Shown, but the targets cannot be changed. |
| Solver, Library, equity tools | Unchanged: these are not statistics. A solve a guest runs appears in your Library. |

Pages that read your progress (Dashboard, Academy, Analytics) still show **your** numbers in
guest mode; only what the guest does is left out of them.

This is a courtesy switch, not access control: RiverLab has no accounts, and anyone holding the
device can turn guest mode off.
