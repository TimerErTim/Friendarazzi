import { cva, type VariantProps } from "class-variance-authority";
import type { HTMLAttributes } from "react";

import { cn } from "@/lib/cn";

/**
 * Neubrutalist sticker badges: crisp `border-2 border-strong`, bold tracked caps,
 * and high-contrast ink labels on saturated brand/semantic tints.
 */
const badgeVariants = cva(
  "inline-flex items-center gap-1 rounded-sm border-2 border-strong px-2 py-0.5 text-[11px] font-bold tracking-[0.06em] uppercase",
  {
    variants: {
      variant: {
        default: "bg-surface-muted text-ink",
        success: "border-success bg-success/15 text-ink",
        error: "border-impact bg-impact/15 text-impact",
        warning: "bg-primary-subtle text-ink",
        primary: "bg-primary text-primary-foreground",
        info: "bg-cyan text-cyan-foreground",
        outline: "bg-surface text-ink",
      },
    },
    defaultVariants: {
      variant: "default",
    },
  },
);

type BadgeProps = HTMLAttributes<HTMLSpanElement> &
  VariantProps<typeof badgeVariants>;

export function Badge({
  className,
  variant = "default",
  ...props
}: BadgeProps) {
  return (
    <span className={cn(badgeVariants({ variant }), className)} {...props} />
  );
}

export { badgeVariants };
