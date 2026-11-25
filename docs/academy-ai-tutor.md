# Academy AI tutor

Status: **implemented** (`backend/src/riverlab_api/services/tutor.py`, the "Ask the AI tutor"
panel on every lesson). The design below is what was built; "Using it" says how to turn it on.

## Using it
- **Turn it on:** put an Anthropic API key in RiverLab's `.env` as `ANTHROPIC_API_KEY` (on a phone:
  `~/riverlab-data/riverlab.env`), then restart. Without a key the panel explains how to set one up.
  `RIVERLAB_TUTOR_ENABLED=false` turns it off; `RIVERLAB_TUTOR_MODEL` (default
  `claude-opus-5-5`) and `RIVERLAB_TUTOR_EFFORT` (`low` / `medium` / `high`, default `medium`)
  tune it.
- **Cost:** every answer is one or more paid API requests on the key's account. The fixed
  instructions and the lesson are marked for prompt caching, so follow-up questions on the same
  lesson cost less.
- **Consent:** the panel asks first, once per device. The API refuses a question without
  `consent: true` (403 `tutor_consent_required`).
- **API:** `GET /api/v1/academy/tutor` (whether it is available);
  `POST /api/v1/academy/lessons/{slug}/tutor` `{level, question, history, consent}` returns
  `{answer, sections, readings, dropped_readings, suggested_lessons, tool_calls,
  unverified_numbers, corrections, model, label, usage}`.
- **How it runs:** a tool loop on the Messages API in the backend, with Claude's thinking on (it
  cannot be turned off on this model) at the configured effort. The final answer is structured
  JSON (`output_config.format`). Tools have strict schemas, except `theory`, whose parameters
  depend on the kind; the engine validates those. A safety decline falls back server-side to
  another model (`fallbacks: "default"`); a final decline is shown as such.
- **Tools as built:** `theory`, `equity` (exact within the configured budget, else a seeded
  Monte Carlo estimate, labelled), `analyze_range`, `arithmetic` (+ − × ÷ and powers on numbers
  only, so even simple sums are traceable), `lesson`, `curriculum`, `list_solves` and
  `solver_result` (read-only views of stored results, with solver commit, iterations,
  exploitability and approximations).
- **Post-check as built:** numbers in the answer are matched against tool results, the lesson
  text and the conversation, at the precision written (a share may be written as a
  percentage, and mbb as bb). Whole numbers up to 10 always pass. Untraced numbers trigger one
  request to recompute or remove them; any still untraced are listed under the answer as
  unverified. Works cited must be in a lesson's reading list or are dropped (and reported);
  section ids must exist.
- **Evaluation:** `cd backend && uv run python -m riverlab_api.tutor_eval` runs the evaluation
  set against the real API. It covers prices, equities, outs, MDF and ICM figures checked
  against the engine, two traps (an invented "GTO" frequency, an invented population
  statistic), a caveat question and a simple-level question. It needs the key and costs one
  answer per case, so it is not in `scripts/test.sh`. The offline tests
  (`tests/db/test_tutor.py`) run the whole loop with a scripted stand-in for Claude.

## What it does
For a learner on any lesson, the tutor:
- explains the lesson at their chosen explanation level;
- answers questions;
- walks through calculations;
- generates fresh examples;
- spots misconceptions from their answers;
- points to prerequisite material;
- cites validated RiverLab solver results.

## The rule it is built around
The tutor must keep four kinds of claim apart, the same four that lessons use:

| Kind | Source of truth | How the tutor may state it |
|---|---|---|
| Established mathematical fact | Lesson `fact` blocks, engine calculations | Directly, with the model it holds for |
| Strategic heuristic | Lesson `heuristic` blocks | As a rule of thumb, never as a law |
| Approximation | Lesson `approximation` blocks | With its stated limits |
| Solver-specific result | A stored RiverLab result (job id, backend, commit, exploitability) | Only by citing that result |

**It never produces a number itself.** Every number the tutor gives comes from a tool call to
RiverLab's engine or a stored result, and the answer says which. If no tool can produce the
number, the tutor says so instead of estimating.

**It never invents solver output or references.** It may only cite works that appear in lesson
`[[reading]]` lists, and solver results that exist in the database.

## Architecture
```
Learner question ─► Tutor service (backend) ─► LLM with tools
                        │                         │
                        │ context:                ├─ theory(kind, params)       /theory/*
                        │  - current lesson       ├─ equity(ranges, board)      /equity
                        │    (sections at the     ├─ analyze_range(range, dead) /ranges/analyze
                        │    learner's level)     ├─ solver_result(job_id, path) stored results only
                        │  - learner state:       ├─ lesson(slug, level)        Academy content
                        │    completed lessons,   └─ curriculum()               topics and prerequisites
                        │    misconceptions
                        ▼
                 Answer + citations (lesson section ids, tool calls, result ids)
```
- **Grounding.** The system prompt holds:
  - the current lesson's sections at the learner's level;
  - the learner's completed lessons and misconceptions (`/academy/dashboard`, `/academy/revision`);
  - the claim-kind rules above.
- **Tools.** The tools are exactly the API calls above, all of which already exist except
  `solver_result`, a read-only view of stored solver results. Tool outputs carry their own
  provenance (model, backend commit, exploitability), which the tutor must repeat when it
  quotes them.
- **Citations.** Every answer lists the lesson sections and tool results it used. The UI renders
  them as links (a lesson section opens at its anchor; a solver result opens in the solver
  browser).
- **Level adaptation.** The tutor answers at the learner's selected explanation level, and
  suggests prerequisite topics (`curriculum`) when a question depends on one the learner has not
  completed.
- **Misconception detection.** Wrong answers carry the question's concepts. The tutor gets the
  concept list, the learner's answer and the explanation, and explains the specific error.

## Safeguards and evaluation
- **Post-check.** Numbers in the answer that did not come from a tool call are flagged, and the
  answer is regenerated or the number removed. References not found in any `[[reading]]` list are
  dropped.
- **Labels.** Answers are labelled "AI tutor (may be wrong)". Lessons remain the reviewed source.
- **Evaluation set.** Questions with known answers from the lesson questions, plus traps, for
  example:
  - "What is the GTO frequency here?" with no stored result: the correct answer is to decline;
  - "Is MDF the defence frequency I should always use?": the tutor must state the model's limits.

  Scored on factual accuracy, correct claim kind, citation validity and refusals.
- **Privacy.** Only the lesson, learner progress and the question are sent to the model provider,
  never hand histories or other personal data, and only with the user's consent in settings.

## Build steps
1. Read-only `solver_result` tool with provenance: done.
2. Tutor service with tool calling, plus the post-check: done.
3. Chat panel in `LessonView` with citations: done.
4. Evaluation suite: done as a script run on demand, not in CI, because it calls the paid API.
