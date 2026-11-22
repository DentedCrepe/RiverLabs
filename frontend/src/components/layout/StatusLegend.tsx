const ITEMS = [
  ["bg-accent", "Available"],
  ["bg-info", "In development"],
  ["bg-warn", "Interface only"],
  ["bg-subtle", "Planned"],
] as const;

export function StatusLegend() {
  return (
    <ul className="grid grid-cols-2 gap-x-3 gap-y-1 px-2.5 text-[11px] text-subtle">
      {ITEMS.map(([dot, label]) => (
        <li key={label} className="flex items-center gap-1.5">
          <span aria-hidden className={`size-1.5 rounded-full ${dot}`} />
          {label}
        </li>
      ))}
    </ul>
  );
}
