"use client";

import { useState } from "react";

import { DemoPage } from "@/components/dev-gallery/demo-page";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { cn } from "@/lib/cn";

export function ComposedPageSection() {
  const [theme, setTheme] = useState<"light" | "dark">("light");

  return (
    <section
      id="composed"
      aria-labelledby="composed-heading"
      className="scroll-mt-6"
    >
      <header className="flex items-baseline gap-4 border-b-2 border-strong pb-3">
        <span
          aria-hidden="true"
          className="rounded-sm border-2 border-strong bg-primary px-1.5 py-0.5 font-mono text-xs font-bold text-primary-foreground tabular-nums"
        >
          16
        </span>
        <div className="min-w-0 flex-1 space-y-1">
          <h2
            id="composed-heading"
            className="font-display text-lg tracking-tight text-ink"
          >
            Zusammengesetzte Seite
          </h2>
          <p className="max-w-prose text-sm font-medium text-secondary">
            Eine ganze Seite aus denselben neubrutalistischen Bausteinen:
            Registerzeile, Titelblock, Filter, Tabs, Kennzahlen, Grafik,
            Tabelle, Formular und Fußblock.
          </p>
        </div>
      </header>

      <div className="mt-4 mb-3 flex flex-wrap items-center justify-between gap-3">
        <ToggleGroup
          type="single"
          value={theme}
          onValueChange={(next) => {
            if (next) {
              setTheme(next as "light" | "dark");
            }
          }}
          aria-label="Theme der Seite"
        >
          <ToggleGroupItem value="light">Papier</ToggleGroupItem>
          <ToggleGroupItem value="dark">Blitzlicht</ToggleGroupItem>
        </ToggleGroup>
        <code className="font-mono text-xs font-bold text-secondary">
          .{theme} · ganze Seite
        </code>
      </div>

      <div
        className={cn(
          theme,
          "brutal-shadow-lg overflow-hidden rounded-md border-[2.5px] border-strong bg-canvas text-ink",
        )}
      >
        <DemoPage />
      </div>
    </section>
  );
}
