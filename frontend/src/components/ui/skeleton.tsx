import type { HTMLAttributes } from "react";

import { cn } from "@/lib/cn";

export function Skeleton({
  className,
  ...props
}: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      aria-hidden="true"
      className={cn(
        "animate-pulse rounded-sm border-2 border-subtle bg-surface-muted",
        className,
      )}
      {...props}
    />
  );
}
