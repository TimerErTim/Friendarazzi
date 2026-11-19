"use client";

import * as SwitchPrimitive from "@radix-ui/react-switch";
import type { ComponentProps } from "react";

import { cn } from "@/lib/cn";

export function Switch({
  className,
  ...props
}: ComponentProps<typeof SwitchPrimitive.Root>) {
  return (
    <SwitchPrimitive.Root
      className={cn(
        "peer brutal-shadow-sm relative inline-flex h-7 w-12 shrink-0 cursor-pointer items-center rounded-full border-2 border-control",
        "before:absolute before:inset-x-0 before:-inset-y-[11px] before:content-['']",
        "transition-colors",
        "data-[state=checked]:bg-primary data-[state=unchecked]:bg-surface-muted",
        "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus",
        "disabled:cursor-not-allowed disabled:opacity-50",
        className,
      )}
      {...props}
    >
      <SwitchPrimitive.Thumb
        className={cn(
          "pointer-events-none block size-4 rounded-full border-2 border-control bg-surface",
          "transition-transform",
          "data-[state=checked]:translate-x-6 data-[state=unchecked]:translate-x-0.5",
        )}
      />
    </SwitchPrimitive.Root>
  );
}
