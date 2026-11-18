"use client";

import * as SliderPrimitive from "@radix-ui/react-slider";
import type { ComponentProps } from "react";

import { cn } from "@/lib/cn";

export function Slider({
  className,
  ...props
}: ComponentProps<typeof SliderPrimitive.Root>) {
  return (
    <SliderPrimitive.Root
      className={cn(
        "relative flex w-full touch-none items-center select-none data-[disabled]:opacity-50",
        className,
      )}
      {...props}
    >
      <SliderPrimitive.Track className="relative h-2.5 w-full grow overflow-hidden rounded-sm border-2 border-control bg-surface-muted">
        <SliderPrimitive.Range className="absolute h-full bg-primary" />
      </SliderPrimitive.Track>
      <SliderPrimitive.Thumb
        className={cn(
          "brutal-shadow-sm relative block size-5 cursor-grab rounded-full border-2 border-control bg-primary",
          "after:absolute after:inset-0 after:m-auto after:size-1.5 after:rounded-full after:bg-primary-foreground after:content-['']",
          "before:absolute before:-inset-[16px] before:content-['']",
          "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus",
          "active:cursor-grabbing disabled:pointer-events-none disabled:cursor-not-allowed",
        )}
      />
    </SliderPrimitive.Root>
  );
}
