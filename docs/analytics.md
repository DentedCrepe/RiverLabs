# Analytics

Analytics summarises only your own data. With no hands or answers it shows empty states, never
sample numbers. Imported hand histories (Hand Analysis) are planned and not included.

## Games against the bots
`GET /api/v1/analytics/play?scope=cash|sit_and_go` runs the engine's session report
(`play_engine::report`, see `docs/play.md`) over every stored hand in that scope, as if it were
one long session: cash and fast-fold together (amounts in bb), or sit & go (tournament chips). It
also lists the sessions the hands came from, with your net result in each. Findings name hands
by their number within their own session.

## Push/fold trainer
Every graded answer on the Trainer page is stored in `push_fold_answers` (migration 0006) with the
engine's full grade. `GET /api/v1/analytics/trainer` reports:
- totals: answers, correct / close / mistakes (the verdicts in `docs/trainer.md`) and EV lost;
- the same by seat, and by stack band (2–5.5, 6–10, 10.5–15, 15.5–20 and 20.5–25bb);
- the ten hand class and seat pairs that cost you the most EV, and your latest answers.

EV lost is always the engine's: the best action's EV minus yours, against the exact equilibrium.
Analytics only adds these numbers up and averages them.

## Postflop drills
`GET /api/v1/analytics/postflop` adds up your postflop drill answers (`postflop_answers`,
migration 0008): verdicts and EV lost overall, by street, by position (in or out of position), and
the ten hand class and street pairs that cost you the most. EV losses are the engine's, from the
saved solves the drills came from.
