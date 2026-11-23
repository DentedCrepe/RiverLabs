# RiverLab Academy

An integrated course in no-limit hold'em and game theory, from the rules to solver theory.
This page covers the architecture, the content format, and how lessons are written, checked,
reviewed and published. The future AI tutor is designed in
[`academy-ai-tutor.md`](academy-ai-tutor.md).

## Principles
1. **Accuracy over quantity.** Every number shown to a learner comes from the engine:
   - widgets call `/theory/*`, `/ranges/analyze` and `/equity`;
   - numeric answers with a `compute` table are recomputed at every sync, and the sync refuses
     answers that disagree.
2. **Say what kind of claim it is.** Lessons mark text as:
   - an **established fact** (true for the stated model or by definition);
   - a **heuristic** (a rule of thumb);
   - an **approximation** (with its limits);
   - a **solver-specific result** (true for one abstracted game).

   A simplified model is never presented as a poker equilibrium.
3. **Only real references.** Further reading lists real works with authors and year. When in
   doubt, leave it out.
4. **Content is data.** Lessons live in `content/academy/`, outside the application code. Fixing
   or adding a lesson needs no frontend change.

## Architecture
```
content/academy/
  curriculum.toml       six levels -> topics (slug, title, optional lesson)
  components.json       interactive components a lesson may embed (name -> props)
  lessons/<slug>.md     TOML front matter + Markdown body
engine/poker-theory     pot odds, bet math (alpha, MDF, bluff ratio), EV, outs, regret matching
backend
  services/academy_content.py   parse + validate content (format below)
  services/academy.py           sync to the database, progress, grading, revision, dashboard
  api/v1/academy.py             /academy/* endpoints;  api/v1/theory.py  /theory/{kind}
  tables (migration 0005)       academy_lessons, academy_lesson_versions, academy_questions,
                                academy_attempts, academy_progress, academy_bookmarks,
                                academy_activity
frontend
  app/academy                   dashboard, /academy/[slug] lesson, /academy/revision
  components/academy            Markdown (+KaTeX), LessonView, QuestionCard, widgets, registry
```

**Sync.** The API reads `RIVERLAB_ACADEMY_CONTENT_DIR` at startup. Authors re-run it with
`POST /api/v1/academy/sync`, which returns every error and warning. Valid lessons are upserted;
invalid ones are skipped and reported. The previous content of a skipped lesson stays live.

**Learner data.** RiverLab has one local learner (no accounts yet), so progress tables have no
user column. When accounts arrive (roadmap Milestone 8) they gain a `user_id`.

## Lesson file format
```toml
+++
slug = "pot-odds"                 # = file name; lowercase-with-dashes
title = "Pot odds and break-even equity"
level = 1                         # 1–6
order = 5                         # position within the level
version = 1                       # bump on every published change
status = "published"              # draft | in_review | published
summary = "One sentence."
estimated_minutes = 15
objectives = ["…"]
prerequisites = ["equity"]        # curriculum topic slugs
tags = ["math"]

[review]
status = "pending"                # pending | reviewed (reviewed needs reviewer + reviewed_on)
reviewer = ""
reviewed_on = ""
notes = "How the content was produced and checked."

[[tools]]                         # links into RiverLab
href = "/postflop"
label = "Equity calculator"
why = "…"

[[reading]]                       # real works only
title = "The Mathematics of Poker"
authors = "Bill Chen and Jerrod Ankenman"
year = 2006

[[questions]]
id = "half-pot"
type = "numeric"                  # numeric | multiple_choice | decision
prompt = "…"
answer = 25.0
tolerance = 0.5
unit = "%"
explanation = "…"
concepts = ["pot-odds"]           # topic slugs, used for revision and misconceptions
compute = { kind = "pot_odds", params = { pot_mbb = 30000, to_call_mbb = 10000 }, field = "required_equity", scale = 100.0 }
+++
```

Choice questions use `options = [...]` and `answer = <index>`. The answer key never leaves
the server until the learner has answered.

**`compute`** recomputes a numeric answer. `kind` is one of:
- a `/theory` calculation: `pot_odds`, `bet_math`, `ev_call`, `ev_bluff`, `outs`,
  `regret_matching`;
- `range`, which uses the range engine with `notation` and `dead`.

`field` is a dotted path into the result (list indexes allowed, e.g. `0.current.0`). `scale`
multiplies it. The stated `answer` must agree within `tolerance`. Use `compute` for every
numeric answer the engine can produce.

### Body
~~~markdown
## Section title {#section-id}

Text shown at every explanation level.

::: simple
…for beginners…
:::
::: intermediate
…
:::
::: advanced
…
:::
::: math
$$ e^* = \frac{B}{P + 2B} $$
:::

::: fact          (or heuristic | approximation | solver)
A labelled callout.
:::

```riverlab:pot-odds
{"pot_bb": "10", "bet_bb": "5"}
```
~~~
- **Sections** must have ids. They are the anchors for "continue where you left off",
  bookmarks, and keeping your place when you switch levels.
- **Explanation levels:** a learner picks one of the four. A section shows its blocks for that
  level. If it has none, it shows the nearest lower level, else the nearest higher one, so no
  section is ever empty. Untagged text and callouts always show. Not every section needs all
  four levels; write the ones that add something.
- **Maths** is KaTeX: `$inline$` and `$$display$$`.
- **Components** must be listed in `components.json`. The frontend registry
  (`components/academy/registry.tsx`) must match it; a test checks this. `question` embeds one
  of the lesson's questions inline: `{"id": "half-pot"}`.

## Writing, checking and publishing a lesson
1. **Draft.** Create `lessons/<slug>.md` with `status = "draft"` and link it from a topic in
   `curriculum.toml`. Drafts are hidden from learners. Preview them at
   `/api/v1/academy/lessons/<slug>?include_drafts=true`.
2. **Check.**
   - Run `cd backend && uv run pytest tests/unit/test_academy_content.py`. It validates every
     lesson in the repository: structure, prerequisites, components, and every `compute`
     answer against the engine.
   - Then `POST /api/v1/academy/sync`.
3. **Review.** Set `status = "in_review"`. A reviewer checks:
   - every claim is correctly labelled (fact, heuristic, approximation, solver result);
   - derivations are correct and their assumptions are stated;
   - examples and numbers match the engine;
   - references exist and say what the lesson cites them for;
   - questions have one defensible answer;
   - nothing presents a simplified model as a poker equilibrium.
4. **Publish.** Set `status = "published"`. Once a person has reviewed it, set
   `review.status = "reviewed"` with the reviewer and date; until then the lesson shows "Not yet
   reviewed by a person". Any later change to a published lesson must bump `version`. The sync
   refuses silent edits and keeps every version in `academy_lesson_versions`.

The five demonstration lessons were drafted with AI assistance. They are published with
`review.status = "pending"` and are labelled that way in the app until a person reviews them.

## Assessment and revision
- **Question types:** numeric (with tolerance), multiple choice, and decision scenarios
  (multiple choice about a hand). Planned: range identification (picking hands on the 13×13
  grid), strategy-frequency questions graded against stored solver results, and practical
  solver exercises.
- **Recorded per attempt:** the answer, whether it was correct, confidence (1–5, optional) and
  time spent.
- **Revision schedule:** a wrong answer is due at once. After 1, 2, 3, 4 or 5+ correct answers
  in a row, the question is due again in 1, 3, 7, 21 or 60 days.
- **Misconceptions:** concepts answered wrongly at least twice. They are shown as suggested
  revision topics.
- **Retention:** of the questions asked more than once, the share whose latest answer is correct.
- **Recommended lesson:** the first published lesson, in level order, that is not completed and
  whose prerequisite topics are covered by completed lessons. A prerequisite topic with no lesson
  yet never blocks.

## Integration with RiverLab tools
Lessons link to tools through `[[tools]]`, and widgets call the same engine as the tools:

| Concept | Tool |
|---|---|
| Equity, outs | Equity calculator (`/postflop`) |
| Combinatorics, blockers | Range editor (`/preflop`) |
| Mixed strategies, MDF, regret matching | Solver (`/solver`), with exploitability per result |
| Pot odds, bluffing | Play (`/play`): the session report grades river calls against pot odds |
| Board analysis, solver diagnostics, hand-analysis EV | Planned tools; lessons will link to them when they exist |
