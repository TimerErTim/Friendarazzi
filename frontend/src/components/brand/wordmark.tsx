import { cn } from "@/lib/cn";

/**
 * Alternating letter colour pattern matching `assets/banner_landscape.*` and
 * `assets/banner_panorama.*`, strictly mapped to semantic brand tokens.
 */
const LETTERS = [
  { char: "F", color: "text-primary" },
  { char: "r", color: "text-cyan" },
  { char: "i", color: "text-cyan" },
  { char: "e", color: "text-pink" },
  { char: "n", color: "text-pink" },
  { char: "d", color: "text-primary" },
  { char: "a", color: "text-primary" },
  { char: "r", color: "text-cyan" },
  { char: "a", color: "text-pink" },
  { char: "z", color: "text-primary" },
  { char: "z", color: "text-cyan" },
  { char: "i", color: "text-cyan" },
] as const;

type BrandMarkProps = {
  size?: "sm" | "md" | "lg";
  variant?: "color" | "mono";
  className?: string;
};

const MARK_SIZE = {
  sm: "size-7",
  md: "size-9",
  lg: "size-12",
} as const;

/**
 * Single-source geometric 3x3 Bingo-Lens mark in semantic CSS tokens.
 * Mirrors `assets/favicon.svg` cleanly at every UI scale.
 */
export function BrandMark({
  size = "md",
  variant = "color",
  className,
}: BrandMarkProps) {
  if (variant === "mono") {
    return (
      <svg
        viewBox="0 0 64 64"
        fill="none"
        aria-hidden="true"
        className={cn(MARK_SIZE[size], "shrink-0", className)}
      >
        <rect
          x="4"
          y="4"
          width="56"
          height="56"
          rx="4"
          stroke="currentColor"
          strokeWidth="5"
        />
        <path
          d="M22.667 4V60M41.333 4V60M4 22.667H60M4 41.333H60"
          stroke="currentColor"
          strokeWidth="4.5"
        />
        <circle cx="32" cy="32" r="8" stroke="currentColor" strokeWidth="4" />
        <circle cx="32" cy="32" r="3.5" fill="currentColor" />
      </svg>
    );
  }

  return (
    <svg
      viewBox="0 0 64 64"
      fill="none"
      aria-hidden="true"
      className={cn(
        MARK_SIZE[size],
        "brutal-shadow-sm shrink-0 rounded-sm border-2 border-strong",
        className,
      )}
    >
      {/* 3x3 grid blocks matching the favicon */}
      <rect
        x="0"
        y="0"
        width="21.3"
        height="21.3"
        fill="var(--color-primary)"
      />
      <rect
        x="21.3"
        y="0"
        width="21.4"
        height="21.3"
        fill="var(--color-pink)"
      />
      <rect
        x="42.7"
        y="0"
        width="21.3"
        height="21.3"
        fill="var(--color-cyan)"
      />

      <rect
        x="0"
        y="21.3"
        width="21.3"
        height="21.4"
        fill="var(--color-pink)"
      />
      <rect
        x="21.3"
        y="21.3"
        width="21.4"
        height="21.4"
        fill="var(--color-primary)"
      />
      <rect
        x="42.7"
        y="21.3"
        width="21.3"
        height="21.4"
        fill="var(--color-pink)"
      />

      <rect
        x="0"
        y="42.7"
        width="21.3"
        height="21.3"
        fill="var(--color-cyan)"
      />
      <rect
        x="21.3"
        y="42.7"
        width="21.4"
        height="21.3"
        fill="var(--color-primary)"
      />
      <rect
        x="42.7"
        y="42.7"
        width="21.3"
        height="21.3"
        fill="var(--color-cyan)"
      />

      {/* Grid lines */}
      <path
        d="M21.3 0V64M42.7 0V64M0 21.3H64M0 42.7H64"
        stroke="var(--primary-foreground)"
        strokeWidth="3.5"
      />
      {/* Corner shapes */}
      <rect
        x="5"
        y="5"
        width="7"
        height="7"
        fill="var(--color-pink)"
        stroke="var(--primary-foreground)"
        strokeWidth="2.5"
      />
      <circle
        cx="53.3"
        cy="10.7"
        r="5.5"
        fill="var(--color-pink)"
        stroke="var(--primary-foreground)"
        strokeWidth="2.5"
      />
      <circle
        cx="10.7"
        cy="53.3"
        r="5.5"
        fill="var(--color-pink)"
        stroke="var(--primary-foreground)"
        strokeWidth="2.5"
      />
      {/* Center camera lens */}
      <circle
        cx="32"
        cy="32"
        r="7.5"
        fill="var(--color-cyan)"
        stroke="var(--primary-foreground)"
        strokeWidth="2.8"
      />
      <circle cx="32" cy="32" r="4" fill="var(--primary-foreground)" />
    </svg>
  );
}

type WordmarkProps = {
  size?: "sm" | "md" | "lg" | "hero";
  showMark?: boolean;
  className?: string;
};

const WORDMARK_SIZE = {
  sm: "text-xl",
  md: "text-2xl",
  lg: "text-3xl sm:text-4xl",
  hero: "text-4xl sm:text-6xl lg:text-7xl",
} as const;

/**
 * Recreated Friendarazzi wordmark in `Archivo Black` with alternating Electric Yellow,
 * Cyan, and Hot Pink letter fills, stark ink outline, and hard neubrutalist offset shadow.
 */
export function Wordmark({
  size = "md",
  showMark = false,
  className,
}: WordmarkProps) {
  const isCompact = size === "sm" || size === "md";

  return (
    <span className={cn("inline-flex items-center gap-2.5", className)}>
      {showMark ? (
        <BrandMark
          size={size === "sm" ? "sm" : size === "hero" ? "lg" : "md"}
        />
      ) : null}
      <span className="sr-only">Friendarazzi</span>
      <span
        aria-hidden="true"
        className={cn(
          "inline-flex select-none font-display leading-none tracking-tight",
          WORDMARK_SIZE[size],
          isCompact ? "wordmark-glyph-sm" : "wordmark-glyph",
        )}
      >
        {LETTERS.map((entry, idx) => (
          <span key={`${entry.char}-${idx}`} className={entry.color}>
            {entry.char}
          </span>
        ))}
      </span>
    </span>
  );
}
