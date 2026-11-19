"use client";

import { useId } from "react";

import { cn } from "@/lib/cn";

export function ComponentSection({
  title,
  description,
  id,
  index,
  children,
}: {
  title: string;
  description?: string;
  id?: string;
  index?: string;
  children: React.ReactNode;
}) {
  const headingId = useId();

  return (
    <section id={id} aria-labelledby={headingId} className="scroll-mt-6">
      <header className="flex items-baseline gap-4 border-b-2 border-strong pb-3">
        {index ? (
          <span
            aria-hidden="true"
            className="rounded-sm border-2 border-strong bg-primary px-1.5 py-0.5 font-mono text-xs font-bold text-primary-foreground tabular-nums"
          >
            {index}
          </span>
        ) : null}
        <div className="min-w-0 flex-1 space-y-1">
          <h2
            id={headingId}
            className="font-display text-lg tracking-tight text-ink"
          >
            {title}
          </h2>
          {description ? (
            <p className="max-w-prose text-sm font-medium text-secondary">
              {description}
            </p>
          ) : null}
        </div>
      </header>

      <div className="mt-4 grid gap-4 lg:grid-cols-2">
        <ThemePlate theme="light">{children}</ThemePlate>
        <ThemePlate theme="dark">{children}</ThemePlate>
      </div>
    </section>
  );
}

function ThemePlate({
  theme,
  children,
}: {
  theme: "light" | "dark";
  children: React.ReactNode;
}) {
  return (
    <div
      className={cn(
        theme,
        "brutal-shadow overflow-hidden rounded-md border-[2.5px] border-strong bg-canvas text-ink",
      )}
    >
      <div className="flex items-center justify-between border-b-2 border-strong bg-surface px-4 py-2">
        <span className="font-mono text-[11px] font-bold tracking-[0.08em] text-ink uppercase">
          {theme === "light" ? "Papier (Light)" : "Blitzlicht (Dark)"}
        </span>
        <code className="font-mono text-[11px] text-secondary">.{theme}</code>
      </div>
      <div className="space-y-5 p-4">{children}</div>
    </div>
  );
}

export function Example({
  caption,
  children,
  className,
}: {
  caption?: string;
  children: React.ReactNode;
  className?: string;
}) {
  const captionId = useId();

  return (
    <div className="space-y-2">
      {caption ? (
        <p
          id={captionId}
          className="font-mono text-[11px] font-bold tracking-[0.07em] text-secondary uppercase"
        >
          {caption}
        </p>
      ) : null}
      <div className={cn("flex flex-wrap items-center gap-2", className)}>
        {children}
      </div>
    </div>
  );
}

export const touchTargetOutline = [
  "[&_a]:outline-2 [&_a]:outline-offset-2 [&_a]:outline-dashed [&_a]:outline-warning",
  "[&_button]:outline-2 [&_button]:outline-offset-2 [&_button]:outline-dashed [&_button]:outline-warning",
  "[&_input]:outline-2 [&_input]:outline-offset-2 [&_input]:outline-dashed [&_input]:outline-warning",
  "[&_select]:outline-2 [&_select]:outline-offset-2 [&_select]:outline-dashed [&_select]:outline-warning",
  "[&_textarea]:outline-2 [&_textarea]:outline-offset-2 [&_textarea]:outline-dashed [&_textarea]:outline-warning",
].join(" ");
