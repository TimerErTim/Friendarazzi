import type { HTMLAttributes } from "react";

import { cn } from "@/lib/cn";

/**
 * A neubrutalist sheet: crisp heavy contour (`border-[2.5px] border-strong`)
 * and hard unblurred offset shadow (`brutal-shadow`).
 */
export function Card({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        "brutal-shadow rounded-md border-[2.5px] border-strong bg-surface",
        className,
      )}
      {...props}
    />
  );
}

export function CardHeader({
  className,
  children,
  ...props
}: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        "space-y-1 border-b-2 border-strong px-6 pt-5 pb-4",
        className,
      )}
      {...props}
    >
      {children}
    </div>
  );
}

/** Bold geometric grotesque display title (`Archivo Black`). */
export function CardTitle({
  className,
  ...props
}: HTMLAttributes<HTMLHeadingElement>) {
  return (
    <h2
      className={cn(
        "font-display text-lg leading-snug tracking-tight text-ink",
        className,
      )}
      {...props}
    />
  );
}

export function CardDescription({
  className,
  ...props
}: HTMLAttributes<HTMLParagraphElement>) {
  return <p className={cn("text-sm text-secondary", className)} {...props} />;
}

export function CardContent({
  className,
  ...props
}: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("px-6 pt-5 pb-6", className)} {...props} />;
}

export function CardFooter({
  className,
  ...props
}: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        "flex items-center gap-2 border-t-2 border-strong bg-surface-muted px-6 py-3",
        className,
      )}
      {...props}
    />
  );
}
