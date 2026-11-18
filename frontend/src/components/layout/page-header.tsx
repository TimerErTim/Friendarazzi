"use client";

import type { ReactNode } from "react";
import { CodeTag, RegisterTag } from "@/components/ui/brutal-elements";
import { cn } from "@/lib/cn";

type PageHeaderProps = {
  register: readonly string[];
  title: ReactNode;
  plate?: string;
  description?: ReactNode;
  actions?: ReactNode;
  className?: string;
};

/**
 * Neubrutalist page header: register trail, heavy display title (`Archivo Black`),
 * optional monospace invite/plate tag, and solid ink divider.
 */
export function PageHeader({
  register,
  title,
  plate,
  description,
  actions,
  className,
}: PageHeaderProps) {
  return (
    <header
      className={cn("space-y-3 border-b-[2.5px] border-strong pb-5", className)}
    >
      <RegisterTag items={register} />

      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap items-center gap-3">
          <h1 className="font-display text-2xl leading-tight tracking-tight text-ink sm:text-3xl">
            {title}
          </h1>
          {plate ? <CodeTag tone="cyan">{plate}</CodeTag> : null}
        </div>

        {actions ? (
          <div className="flex flex-wrap items-center gap-2">{actions}</div>
        ) : null}
      </div>

      {description ? (
        <p className="max-w-2xl text-sm font-medium text-secondary">
          {description}
        </p>
      ) : null}
    </header>
  );
}
