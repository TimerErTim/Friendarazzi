import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import type { ComponentPropsWithRef } from "react";

import { cn } from "@/lib/cn";

/**
 * Neubrutalist tactile buttons: heavy contour, hard offset shadow, and active
 * press depression (`translate(2px, 2px)`).
 *
 * `primary` uses Electric Yellow with stark ink text (15.3:1 contrast) and is the
 * one dominant CTA per screen. `secondary` is crisp surface paper with a bold
 * contour. `ghost` stays flat until hovered. `destructive` is outlined in impact red.
 */
const buttonVariants = cva(
  [
    "relative inline-flex shrink-0 cursor-pointer items-center justify-center gap-2 rounded-sm",
    "text-sm font-bold tracking-tight whitespace-nowrap",
    "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus",
    "disabled:cursor-not-allowed disabled:opacity-50",
    "[&_svg]:pointer-events-none [&_svg]:size-4 [&_svg]:shrink-0",
  ].join(" "),
  {
    variants: {
      variant: {
        primary:
          "brutal-press border-2 border-control bg-primary text-primary-foreground hover:bg-primary-hover",
        secondary:
          "brutal-press border-2 border-control bg-surface text-ink hover:bg-surface-muted",
        ghost:
          "border-2 border-transparent text-ink transition-colors hover:border-control hover:bg-surface-muted",
        destructive:
          "brutal-press border-2 border-impact bg-surface text-impact hover:bg-impact/10",
      },
      size: {
        sm: "h-8 px-3 text-xs before:absolute before:inset-x-0 before:-inset-y-[9px] before:content-['']",
        md: "h-10 px-4 before:absolute before:inset-x-0 before:-inset-y-[5px] before:content-['']",
        lg: "h-12 px-6 text-base",
        field: "h-11 px-4",
        icon: "size-10 before:absolute before:-inset-[5px] before:content-['']",
      },
    },
    defaultVariants: {
      variant: "primary",
      size: "md",
    },
  },
);

type ButtonProps = ComponentPropsWithRef<"button"> &
  VariantProps<typeof buttonVariants> & {
    /** Render the variant classes on a child element, for links. */
    asChild?: boolean;
  };

export function Button({
  className,
  variant = "primary",
  size = "md",
  type = "button",
  asChild = false,
  ...props
}: ButtonProps) {
  const Component = asChild ? Slot : "button";

  return (
    <Component
      {...(asChild ? {} : { type })}
      className={cn(buttonVariants({ variant, size }), className)}
      {...props}
    />
  );
}

export { buttonVariants };
