import type { ComponentProps } from "react";

import { cn } from "@/lib/cn";

/**
 * Bold diagonal hatch divider separating major sections.
 */
export function StickerBand({
  className,
  checker = false,
  height = "md",
  ...props
}: ComponentProps<"div"> & {
  checker?: boolean;
  height?: "sm" | "md" | "lg";
}) {
  const heightClass = { sm: "h-3", md: "h-5", lg: "h-8" }[height];

  return (
    <div
      aria-hidden="true"
      className={cn(
        checker ? "brutal-checker" : "brutal-hatch",
        heightClass,
        "w-full border-y-2 border-strong",
        className,
      )}
      {...props}
    />
  );
}

/**
 * Neubrutalist callout block for calm contextual notes (empty states, waiting states).
 */
export function CalloutNote({
  label,
  tone = "neutral",
  children,
  className,
}: {
  label: string;
  tone?: "neutral" | "yellow" | "cyan" | "pink";
  children: React.ReactNode;
  className?: string;
}) {
  const toneClass = {
    neutral: "bg-surface-muted",
    yellow: "bg-primary-subtle",
    cyan: "bg-cyan-subtle",
    pink: "bg-pink-subtle",
  }[tone];

  return (
    <aside
      className={cn(
        "brutal-shadow-sm rounded-md border-2 border-strong p-4",
        toneClass,
        className,
      )}
    >
      <p className="inline-block rounded-sm border-2 border-strong bg-surface px-2 py-0.5 font-mono text-[11px] font-bold tracking-wider text-ink uppercase">
        {label}
      </p>
      <div className="mt-2.5 text-sm font-medium text-ink">{children}</div>
    </aside>
  );
}

/**
 * Monospace tag for invite codes, numeric counters, and plate indices.
 */
export function CodeTag({
  children,
  tone = "surface",
  className,
}: {
  children: React.ReactNode;
  tone?: "surface" | "yellow" | "cyan" | "pink";
  className?: string;
}) {
  const toneClass = {
    surface: "bg-surface text-ink",
    yellow: "bg-primary text-primary-foreground",
    cyan: "bg-cyan text-cyan-foreground",
    pink: "bg-pink text-pink-foreground",
  }[tone];

  return (
    <span
      className={cn(
        "brutal-shadow-sm inline-flex items-center rounded-sm border-2 border-strong px-2 py-0.5",
        "font-mono text-xs font-bold tracking-wider uppercase tabular-nums",
        toneClass,
        className,
      )}
    >
      {children}
    </span>
  );
}

/**
 * Running register pills above page headings.
 */
export function RegisterTag({
  items,
  className,
}: {
  items: readonly string[];
  className?: string;
}) {
  return (
    <p
      className={cn(
        "flex flex-wrap items-center gap-x-2 gap-y-1 font-mono text-[11px] font-bold tracking-[0.08em] text-secondary uppercase",
        className,
      )}
    >
      {items.map((item, index) => (
        <span key={item} className="flex items-center gap-2">
          {index > 0 ? <span aria-hidden="true">/</span> : null}
          {item}
        </span>
      ))}
    </p>
  );
}

/**
 * 4-point comic sparkle / starburst motif from the Friendarazzi banners.
 */
export function SparkleIcon({
  className,
  tone = "primary",
}: {
  className?: string;
  tone?: "primary" | "accent" | "cyan";
}) {
  const fill = {
    primary: "var(--color-primary)",
    accent: "var(--color-pink)",
    cyan: "var(--color-cyan)",
  }[tone];

  return (
    <svg
      viewBox="0 0 32 32"
      fill="none"
      aria-hidden="true"
      className={cn("size-6 shrink-0", className)}
    >
      <path
        d="M16 2L19.8 12.2L30 16L19.8 19.8L16 30L12.2 19.8L2 16L12.2 12.2L16 2Z"
        fill={fill}
        stroke="var(--primary-foreground)"
        strokeWidth="2.5"
        strokeLinejoin="round"
      />
    </svg>
  );
}
