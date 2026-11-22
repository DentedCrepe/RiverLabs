import Link from "next/link";

export function Brand() {
  return (
    <Link href="/" className="flex items-center gap-2.5 rounded-md px-1 py-1" aria-label="RiverLab home">
      <svg aria-hidden viewBox="0 0 24 24" className="size-6">
        <rect x="1" y="1" width="22" height="22" rx="5" fill="var(--color-accent-dim)" />
        <path
          d="M4 15c2.5 0 2.5-3 5-3s2.5 3 5 3 2.5-3 5-3M4 10c2.5 0 2.5-3 5-3s2.5 3 5 3 2.5-3 5-3"
          stroke="var(--color-accent-strong)"
          strokeWidth="1.6"
          fill="none"
          strokeLinecap="round"
        />
      </svg>
      <span className="text-[15px] font-semibold tracking-tight text-fg">
        River<span className="text-accent">Lab</span>
      </span>
    </Link>
  );
}
