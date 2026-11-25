/**
 * Plain-language definitions for the metrics shown in the app. Each says exactly how RiverLab
 * measures it (see docs/play.md and docs/solver-validation.md).
 */
export const GLOSSARY = {
  hands: "Hands you were dealt in this session.",
  result: "Your total win or loss. Each hand is converted at its own big blind, so sit & go levels compare fairly.",
  bb100:
    "Big blinds won per 100 hands: your result divided by hands played, times 100. It swings a lot over small samples.",
  allInLuck:
    "For hands where the money went in before the river: your expected result is your exact equity at that moment times the pot (after rake), minus what you put in. Luck = actual result minus expected, summed over those hands. Positive means the cards ran better than average.",
  vpip: "Voluntarily Put money In Pot: the share of hands where you called or raised preflop. Posting a blind or checking the big blind doesn't count.",
  pfr: "Preflop Raise: the share of hands where you raised before the flop, all-in raises included (calling an all-in does not count).",
  threeBet: "How often you re-raised when facing exactly one raise preflop, out of the times you had that chance.",
  foldToThreeBet: "After you opened with a raise and someone re-raised, how often you folded.",
  cbet: "Continuation bet: as the last preflop raiser, how often you bet the flop when no one had bet before you.",
  foldToCbet: "When the preflop raiser (a bot) bet the flop first, how often you folded to it.",
  wtsd: "Went To ShowDown: of the hands where you saw the flop, how often you reached showdown without folding.",
  wsd: "Won at ShowDown: of your showdowns, how often you won money.",
  aggression:
    "Aggression factor: your bets and raises after the flop divided by your calls. Above 1 means you bet or raise more often than you call.",
  modelComparison:
    "For preflop spots on the bots' model tree: how often you played or raised, against how often the model would with the same hands in the same spots. The model is approximate.",
  feedback:
    "Each decision is compared with the same references as the session report: the bots' preflop model (approximate) for preflop choices, and pot odds against the bots' actual ranges for calls and folds. Bets and raises after the flop aren't graded.",
  heroEquity:
    "Your share of the pot if all remaining cards were dealt with no more betting, against the hands the bots could still have given how they've played (their tracked ranges). Monte Carlo estimate. Facing a bet, the price shows the equity a call needs to break even when no more money goes in.",
  icm: "ICM (the Independent Chip Model, Malmuth–Harville) turns tournament stacks into shares of the prize pool: your chance of finishing first is your share of the chips, and lower places follow the same way. It is a model: it ignores skill, position and the blinds. Heads-up all-in calls and folds in a sit & go are graded against the equity ICM requires, not chip pot odds. Shown under each seat.",
  reveal:
    "Practice mode: shows the bots' cards during hands. It makes decisions easier, so hands played with it on are flagged in the report.",
  exploitability:
    "How much a perfect opponent could win against this strategy in the solved game, per hand. Zero means an exact equilibrium for the bet sizes allowed.",
  independentCheck:
    "RiverLab recomputes the exploitability with its own best-response code, separate from the solver's, as a cross-check.",
  solverValue:
    "Average result per hand for this player under the solved strategies, net of their half of the starting pot.",
  modelGap:
    "How much players could still gain by changing strategy inside the preflop model. It measures how well the model has converged, not how close the model is to real poker.",
} as const;
