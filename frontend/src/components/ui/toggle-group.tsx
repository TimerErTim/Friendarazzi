"use client";

import * as ToggleGroupPrimitive from "@radix-ui/react-toggle-group";
import { cva, type VariantProps } from "class-variance-authority";
import type { ComponentProps } from "react";

import { cn } from "@/lib/cn";

const toggleGroupItemVariants = cva(
  [
    "relative -ml-[2px] inline-flex min-h-11 min-w-11 cursor-pointer items-center justify-center gap-2 px-3.5",
    "border-2 border-control bg-surface text-sm font-bold text-ink",
    "transition-colors first:ml-0 first:rounded-l-sm last:rounded-r-sm",
    "hover:bg-surface-muted",
    "focus-visible:z-10 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus",
    "disabled:pointer-events-none disabled:opacity-50",
    "data-[state=on]:z-10 data-[state=on]:bg-primary data-[state=on]:text-primary-foreground",
  ].join(" "),
);

export function ToggleGroup({
  className,
  ...props
}: ComponentProps<typeof ToggleGroupPrimitive.Root>) {
  return (
    <ToggleGroupPrimitive.Root
      className={cn(
        "brutal-shadow-sm flex w-fit flex-wrap items-center rounded-sm",
        className,
      )}
      {...props}
    />
  );
}

export function ToggleGroupItem({
  className,
  ...props
}: ComponentProps<typeof ToggleGroupPrimitive.Item> &
  VariantProps<typeof toggleGroupItemVariants>) {
  return (
    <ToggleGroupPrimitive.Item
      className={cn(toggleGroupItemVariants(), className)}
      {...props}
    />
  );
}
