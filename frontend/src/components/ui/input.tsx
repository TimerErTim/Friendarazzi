import type { ComponentPropsWithRef } from "react";

import { cn } from "@/lib/cn";

type InputProps = ComponentPropsWithRef<"input">;

/**
 * Neubrutalist input field: 44px height (`h-11`), crisp `border-2 border-control`,
 * hard 2px shadow, and high-contrast focus outline.
 */
export function Input({ className, ...props }: InputProps) {
  return (
    <input
      className={cn(
        "brutal-shadow-sm h-11 w-full rounded-sm border-2 border-control bg-elevated px-3 text-sm font-medium text-ink",
        "transition-colors placeholder:text-muted",
        "hover:bg-surface-muted/40",
        "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus",
        "disabled:cursor-not-allowed disabled:bg-surface-muted disabled:text-muted disabled:shadow-none",
        "aria-invalid:border-impact aria-invalid:focus-visible:outline-impact",
        className,
      )}
      {...props}
    />
  );
}
