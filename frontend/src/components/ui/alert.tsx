import { cva, type VariantProps } from "class-variance-authority";
import type { HTMLAttributes, ReactNode } from "react";

import { cn } from "@/lib/cn";

/**
 * Neubrutalist callout box: heavy contour, crisp 2px offset shadow, thick coloured
 * left bar, and high-contrast ink text so every status remains AA-legible.
 */
const alertVariants = cva(
  "brutal-shadow-sm flex items-start gap-3 rounded-sm border-2 border-strong border-l-[6px] bg-surface px-4 py-3 text-sm font-medium text-ink",
  {
    variants: {
      variant: {
        info: "border-l-cyan bg-cyan-subtle",
        success: "border-l-success bg-success/10",
        warning: "border-l-primary bg-primary-subtle",
        error: "border-l-impact bg-impact/10",
      },
    },
    defaultVariants: {
      variant: "info",
    },
  },
);

type AlertProps = HTMLAttributes<HTMLDivElement> &
  VariantProps<typeof alertVariants> & {
    icon?: ReactNode;
  };

const iconColour: Record<NonNullable<AlertProps["variant"]>, string> = {
  info: "text-ink",
  success: "text-success",
  warning: "text-ink",
  error: "text-impact",
};

export function Alert({
  className,
  variant = "info",
  icon,
  children,
  ...props
}: AlertProps) {
  const tone = variant ?? "info";

  return (
    <div
      role={tone === "error" ? "alert" : "status"}
      className={cn(alertVariants({ variant: tone }), className)}
      {...props}
    >
      {icon ? (
        <span className={cn("mt-0.5 shrink-0", iconColour[tone])}>{icon}</span>
      ) : null}
      <div className="min-w-0 flex-1">{children}</div>
    </div>
  );
}

export { alertVariants };
