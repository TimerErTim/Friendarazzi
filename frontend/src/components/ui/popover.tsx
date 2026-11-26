"use client";

import * as PopoverPrimitive from "@radix-ui/react-popover";
import type { ComponentProps } from "react";

import { cn } from "@/lib/cn";

export const Popover = PopoverPrimitive.Root;
export const PopoverTrigger = PopoverPrimitive.Trigger;
export const PopoverAnchor = PopoverPrimitive.Anchor;

export function PopoverContent({
  className,
  align = "end",
  sideOffset = 6,
  ...props
}: ComponentProps<typeof PopoverPrimitive.Content>) {
  return (
    <PopoverPrimitive.Portal>
      <PopoverPrimitive.Content
        align={align}
        sideOffset={sideOffset}
        className={cn(
          "brutal-shadow z-50 w-72 rounded-md border-2 border-strong bg-elevated p-3 text-ink",
          "focus-visible:outline-none",
          className,
        )}
        {...props}
      />
    </PopoverPrimitive.Portal>
  );
}
