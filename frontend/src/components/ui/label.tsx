import type { ComponentProps } from "react";

import { cn } from "@/lib/cn";

export function Label({ className, ...props }: ComponentProps<"label">) {
  return (
    // biome-ignore lint/a11y/noLabelWithoutControl: `htmlFor` is supplied by `Field`.
    <label
      className={cn(
        "text-xs font-bold tracking-[0.06em] text-ink uppercase",
        className,
      )}
      {...props}
    />
  );
}
