# Academy AI tutor: design (not implemented)

Status: **design only.** Nothing here is built yet. The Academy is built so the tutor can be
added without changing lesson content.

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

## Build steps (when the tutor is scheduled)
1. Read-only `solver_result` endpoint with provenance.
2. Tutor service with tool calling, plus the post-check.
3. Chat panel in `LessonView` with citations.
4. Evaluation suite in CI.
