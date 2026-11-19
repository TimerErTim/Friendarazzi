"use client";

import Link from "next/link";
import { useState } from "react";
import { BrutalStickersSection } from "@/components/dev-gallery/brutal-stickers-section";
import { touchTargetOutline } from "@/components/dev-gallery/component-section";
import { ComposedPageSection } from "@/components/dev-gallery/composed-page-section";
import { DataFiguresSection } from "@/components/dev-gallery/data-figures-section";
import { InkedControlsSection } from "@/components/dev-gallery/inked-controls-section";
import {
  AlertSection,
  BadgeSection,
  ButtonSection,
  CardSection,
  DataSection,
  FormSection,
  MiscSection,
  OverlaySection,
} from "@/components/dev-gallery/sections";
import {
  CheckpointSliderSection,
  SortableTableSection,
} from "@/components/dev-gallery/sortable-table-section";
import { TokenSwatches } from "@/components/dev-gallery/token-swatches";
import { TypeSection } from "@/components/dev-gallery/type-section";
import { SetBreadcrumbLabel } from "@/components/layout/page-chrome";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";

const PLATES = [
  { id: "tokens", index: "01", label: "Tokens" },
  { id: "type", index: "02", label: "Schrift" },
  { id: "sortable-table", index: "03", label: "Sortierbare Tabelle" },
  { id: "figures", index: "04", label: "Datenbilder" },
  { id: "checkpoint-slider", index: "05", label: "Bereichs-Slider" },
  { id: "button", index: "06", label: "Button" },
  { id: "badge", index: "07", label: "Badge" },
  { id: "alert", index: "08", label: "Alert" },
  { id: "card", index: "09", label: "Card" },
  { id: "form", index: "10", label: "Formular" },
  { id: "data", index: "11", label: "Tabelle & Maß" },
  { id: "overlay", index: "12", label: "Überlagerung" },
  { id: "misc", index: "13", label: "Kleinteile" },
  { id: "brand-stickers", index: "14", label: "Marke & Sticker" },
  { id: "inked", index: "15", label: "Markierte Bedienelemente" },
  { id: "composed", index: "16", label: "Zusammengesetzte Seite" },
] as const;

export default function DevGalleryPage() {
  const [showTouchTargets, setShowTouchTargets] = useState(false);

  return (
    <div className="min-h-full bg-canvas">
      <SetBreadcrumbLabel label="Design-Galerie" />

      <div className="mx-auto max-w-6xl px-6 py-10">
        <header className="space-y-4">
          <div className="flex flex-wrap items-center gap-2">
            <Badge variant="primary">Neubrutalismus</Badge>
            <Badge variant="info">Nur im Dev-Server</Badge>
            <span className="font-mono text-xs font-bold text-secondary">
              16 Tafeln
            </span>
          </div>

          <div className="space-y-2">
            <h1 className="font-display text-3xl leading-tight tracking-tight text-balance">
              Gestaltungs-Galerie
            </h1>
            <p className="max-w-prose text-sm font-medium text-secondary">
              Das Design-System als Musterbuch. Jede Tafel zeigt dieselben
              Bausteine zweimal: links auf hellem Papier, rechts im dunklen
              Blitzlicht. Alle Token-Werte kommen ausschließlich aus{" "}
              <code className="font-mono text-xs font-bold text-ink">
                globals.css
              </code>
              .
            </p>
          </div>

          <Separator />
        </header>

        <nav aria-label="Tafeln" className="py-6">
          <ol className="grid gap-x-8 gap-y-2 sm:grid-cols-2 lg:grid-cols-3">
            {PLATES.map((plate) => (
              <li key={plate.id}>
                <a
                  href={`#${plate.id}`}
                  className="group flex items-center gap-2 rounded-sm py-1 text-sm font-bold transition-colors hover:underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
                >
                  <span className="rounded-sm border border-strong bg-primary px-1.5 py-0.5 font-mono text-[11px] font-bold text-primary-foreground tabular-nums">
                    {plate.index}
                  </span>
                  <span>{plate.label}</span>
                </a>
              </li>
            ))}
          </ol>
        </nav>

        <Separator />

        <div className="flex flex-wrap items-center justify-between gap-3 py-4">
          <label className="flex min-h-11 cursor-pointer items-center gap-3 text-sm font-bold">
            <input
              type="checkbox"
              checked={showTouchTargets}
              onChange={(event) => setShowTouchTargets(event.target.checked)}
              className="checkbox-tick brutal-shadow-sm size-5 appearance-none rounded-sm border-2 border-control bg-surface"
            />
            44 px Touch-Ziele markieren
          </label>
          <Button variant="secondary" size="sm" asChild>
            <Link href="/">Zur App</Link>
          </Button>
        </div>

        <div className="space-y-14">
          <TokenSwatches />
          <TypeSection />
          <SortableTableSection />
          <DataFiguresSection />
          <CheckpointSliderSection />
          <div className={showTouchTargets ? touchTargetOutline : undefined}>
            <div className="space-y-14">
              <ButtonSection />
              <BadgeSection />
              <AlertSection />
              <CardSection />
              <FormSection />
              <DataSection />
              <OverlaySection />
              <MiscSection />
              <BrutalStickersSection />
              <InkedControlsSection />
            </div>
          </div>
          <ComposedPageSection />
        </div>

        <footer className="mt-16 border-t-2 border-strong pt-4">
          <p className="flex flex-wrap items-center gap-x-3 gap-y-1 font-mono text-xs font-bold text-secondary">
            <span>Archivo Black · Space Grotesk · JetBrains Mono</span>
            <span aria-hidden="true">·</span>
            <span>WCAG 2.2 AA</span>
            <span aria-hidden="true">·</span>
            <span>44 px Touch</span>
          </p>
        </footer>
      </div>
    </div>
  );
}
