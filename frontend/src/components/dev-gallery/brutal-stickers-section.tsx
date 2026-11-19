"use client";

import { BrandMark, Wordmark } from "@/components/brand/wordmark";
import {
  ComponentSection,
  Example,
} from "@/components/dev-gallery/component-section";
import {
  CalloutNote,
  CodeTag,
  RegisterTag,
  SparkleIcon,
  StickerBand,
} from "@/components/ui/brutal-elements";

export function BrutalStickersSection() {
  return (
    <ComponentSection
      id="brand-stickers"
      index="14"
      title="Marke & Sticker"
      description="Wortmarke, 3×3 Bingo-Linse (farbig & monochrom), Schachbrett-/Schraffenbänder, Code-Tags und Hinweis-Boxen aus semantischen Tokens."
    >
      <Example caption="Wortmarke & Mark">
        <div className="w-full space-y-4">
          <Wordmark size="lg" showMark />
          <div className="flex flex-wrap items-center gap-4">
            <BrandMark size="sm" />
            <BrandMark size="md" />
            <BrandMark size="lg" />
            <BrandMark size="md" variant="mono" />
          </div>
        </div>
      </Example>

      <Example caption="Bänder — Schraffen & Schachbrett" className="block">
        <div className="w-full space-y-3">
          <StickerBand height="sm" />
          <StickerBand checker height="md" />
        </div>
      </Example>

      <Example caption="Code-Tags & Registerzeile">
        <div className="w-full space-y-3">
          <RegisterTag items={["Kategorie", "Alpha", "Entwurf"]} />
          <div className="flex flex-wrap gap-2">
            <CodeTag tone="surface">A7K9P2</CodeTag>
            <CodeTag tone="yellow">014</CodeTag>
            <CodeTag tone="cyan">3×3</CodeTag>
            <CodeTag tone="pink">BINGO!</CodeTag>
            <SparkleIcon tone="primary" />
            <SparkleIcon tone="accent" />
            <SparkleIcon tone="cyan" />
          </div>
        </div>
      </Example>

      <Example
        caption="CalloutNote — Hinweis-Box ohne Alert-Dringlichkeit"
        className="block"
      >
        <CalloutNote label="Hinweis" tone="yellow">
          Diese Box begleitet einen Zustand ohne Fehler- oder
          Erfolgs-Dringlichkeit.
        </CalloutNote>
      </Example>
    </ComponentSection>
  );
}
