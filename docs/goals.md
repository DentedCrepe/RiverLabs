# Daily goals and streak

The Dashboard's **Today** card sets two daily targets and keeps a streak
(`services/daily.py`, `GET /api/v1/goals/today`, `PUT /api/v1/goals`).

- **Hands:** hands finished against the bots that day (any format). Default 30.
- **Study:** study items that day. Each Academy question answered, push/fold drill and postflop
  drill counts as one. Default 10.

Both targets can be changed on the card (1 to 1000) and are stored in `daily_goals` (migration
0009, one row).

**Days** follow this device's local time. A day is **met** when both targets are reached. The
**streak** is the number of consecutive met days up to today; until today is met, it counts up to
yesterday, so it only breaks once a whole day passes without meeting the goals. The card also
shows the best streak and the last 14 days (met, some activity, or none).

## Recommended today
Built only from your own data; with none, only a general drill suggestion appears.

1. **Your most frequent weakness at the tables:** the session report (`docs/play.md`) is run
   over your last 300 cash and fast-fold hands. The first finding seen at least twice is mapped to
   where to study it:

   | Finding | Study |
   |---|---|
   | Played hands the model folds / folded hands it plays / called its raises | Preflop page (the model's ranges) |
   | Called without enough equity (river or all-in) | Pot odds lesson |
   | Called without enough equity (flop or turn) | Implied odds lesson |
   | Folded with enough equity (river or all-in) | Pot odds lesson |
   | Folded with enough equity (flop or turn) | Equity lesson |

   It links to Analytics, where you can flick through the hands behind each weakness.
2. **Academy:** your due revision questions if there are any; otherwise the next lesson whose
   prerequisites you have covered.
3. **Trainer:** your weakest postflop street (at least 5 answers, losing more than 1% of the pot on
   average), else your weaker push/fold seat (at least 5 answers), else push/fold drills.

## Guest mode

What a guest does in guest mode never counts here; see [guest-mode.md](guest-mode.md).
