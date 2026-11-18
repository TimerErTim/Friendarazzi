import type { SelectHTMLAttributes } from "react";

import { cn } from "@/lib/cn";

type SelectProps = SelectHTMLAttributes<HTMLSelectElement>;

/** Native select styled to match `Input` with a crisp `currentColor` chevron. */
export function Select({ className, ...props }: SelectProps) {
  return (
    <select
      className={cn(
        "select-chevron brutal-shadow-sm h-11 w-full appearance-none rounded-sm border-2 border-control bg-elevated py-0 pr-9 pl-3 text-sm font-medium text-ink",
        "transition-colors hover:bg-surface-muted/40",
        "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus",
        "disabled:cursor-not-allowed disabled:bg-surface-muted disabled:text-muted disabled:shadow-none",
        className,
      )}
      {...props}
    />
  );
}
