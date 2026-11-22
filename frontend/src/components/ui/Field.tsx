import type { InputHTMLAttributes, ReactNode, SelectHTMLAttributes } from "react";
import { useId } from "react";
import { cn } from "./cn";

const CONTROL =
  "h-8 w-full rounded-md border border-line bg-canvas px-2.5 text-sm text-fg placeholder:text-subtle " +
  "hover:border-line-strong focus:border-accent focus:outline-none disabled:opacity-50";

export function Field({
  label,
  hint,
  error,
  children,
}: {
  label: string;
  hint?: ReactNode;
  error?: string | null;
  children: (id: string, describedBy: string | undefined) => ReactNode;
}) {
  const id = useId();
  const hintId = `${id}-hint`;
  return (
    <div className="flex flex-col gap-1">
      <label htmlFor={id} className="label-caps">
        {label}
      </label>
      {children(id, hint || error ? hintId : undefined)}
      {(hint || error) && (
        <p id={hintId} className={cn("text-xs", error ? "text-danger" : "text-subtle")}>
          {error ?? hint}
        </p>
      )}
    </div>
  );
}

export function TextInput({ className, ...rest }: InputHTMLAttributes<HTMLInputElement>) {
  return <input className={cn(CONTROL, "font-mono tabular", className)} {...rest} />;
}

export function Select({ className, children, ...rest }: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <select className={cn(CONTROL, "pr-7", className)} {...rest}>
      {children}
    </select>
  );
}
