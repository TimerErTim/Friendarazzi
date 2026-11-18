import type { ComponentProps, HTMLAttributes } from "react";

import { cn } from "@/lib/cn";

export function Table({
  className,
  ...props
}: HTMLAttributes<HTMLTableElement>) {
  return (
    <div className="brutal-shadow overflow-x-auto rounded-md border-[2.5px] border-strong bg-surface">
      <table
        className={cn("w-full border-collapse text-sm", className)}
        {...props}
      />
    </div>
  );
}

export function TableHead({
  className,
  ...props
}: HTMLAttributes<HTMLTableSectionElement>) {
  return (
    <thead
      className={cn(
        "sticky top-0 z-10 border-b-2 border-strong bg-primary text-left",
        "text-[11px] font-bold tracking-[0.07em] text-primary-foreground uppercase",
        className,
      )}
      {...props}
    />
  );
}

export function TableBody({
  className,
  ...props
}: HTMLAttributes<HTMLTableSectionElement>) {
  return (
    <tbody className={cn("divide-y-2 divide-subtle", className)} {...props} />
  );
}

export function TableRow({ className, ...props }: ComponentProps<"tr">) {
  return (
    <tr
      className={cn("transition-colors hover:bg-surface-muted/60", className)}
      {...props}
    />
  );
}

export function TableHeader({ className, ...props }: ComponentProps<"th">) {
  return (
    <th
      scope="col"
      className={cn("px-3 py-2.5 font-bold", className)}
      {...props}
    />
  );
}

export function TableCell({ className, ...props }: ComponentProps<"td">) {
  return (
    <td className={cn("px-3 py-2.5 align-middle", className)} {...props} />
  );
}

export function TableKeyValue({
  labels,
  className,
}: {
  labels: { id: string; label: string; value: string }[];
  className?: string;
}) {
  return (
    <dl className={cn("space-y-2 text-sm", className)}>
      {labels.map((entry) => (
        <div
          key={entry.id}
          className="flex items-baseline justify-between gap-3 border-b border-dashed border-subtle pb-1 last:border-0 last:pb-0"
        >
          <dt className="shrink-0 font-medium text-secondary">{entry.label}</dt>
          <dd className="shrink-0 font-mono font-bold text-ink tabular-nums">
            {entry.value}
          </dd>
        </div>
      ))}
    </dl>
  );
}
