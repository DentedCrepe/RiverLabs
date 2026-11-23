/** Colours by action type; bets/raises get progressively warmer by order of size. */
const AGGRESSIVE = ["#d6a43a", "#e0864f", "#e0574f", "#b83b5e", "#8f2d56"];

export function actionColors(actions: string[]): string[] {
  let k = 0;
  return actions.map((label) => {
    const a = label.toLowerCase();
    if (a.startsWith("fold")) return "#5b8def";
    if (a.startsWith("check") || a.startsWith("call") || a.startsWith("limp")) return "#3fae7a";
    return AGGRESSIVE[Math.min(k++, AGGRESSIVE.length - 1)];
  });
}
