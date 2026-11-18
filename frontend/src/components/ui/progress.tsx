"use client";

import * as ProgressPrimitive from "@radix-ui/react-progress";
import type { ComponentProps } from "react";

import { cn } from "@/lib/cn";

export function Progress({
  className,
  value = 0,
  ...props
}: ComponentProps<typeof ProgressPrimitive.Root>) {
  const clamped = Math.min(100, Math.max(0, value ?? 0));

  return (
    <ProgressPrimitive.Root
      value={clamped}
      className={cn(
        "relative h-3 w-full overflow-hidden rounded-sm border-2 border-strong bg-surface-muted",
        className,
      )}
      {...props}
    >
      <ProgressPrimitive.Indicator
        className="h-full bg-primary transition-transform"
        style={{ transform: `translateX(-${100 - clamped}%)` }}
      />
      <span
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 flex justify-between"
      >
        {[0, 1, 2, 3, 4].map((tick) => (
          <span key={`tick-${tick}`} className="w-0.5 bg-strong/30" />
        ))}
      </span>
    </ProgressPrimitive.Root>
  );
}
