"use client";

import { useEffect, useRef, useState } from "react";
import {
  ComponentSection,
  Example,
} from "@/components/dev-gallery/component-section";
import {
  AA_NORMAL_TEXT,
  contrastRatio,
} from "@/components/dev-gallery/contrast";
import { Badge } from "@/components/ui/badge";

const COLOUR_TOKENS = [
  "bg-canvas",
  "bg-surface",
  "bg-elevated",
  "bg-surface-muted",
  "border-subtle",
  "border-strong",
  "text-primary",
  "text-secondary",
  "text-muted",
  "color-primary",
  "color-primary-hover",
  "color-primary-subtle",
  "primary-foreground",
  "color-pink",
  "color-pink-subtle",
  "pink-foreground",
  "color-cyan",
  "color-cyan-subtle",
  "cyan-foreground",
  "color-focus",
  "shadow-ink",
  "semantic-impact",
  "semantic-warning",
  "semantic-success",
  "semantic-info",
] as const;

const CONTRAST_PAIRS = [
  { fg: "text-primary", bg: "bg-canvas", label: "Ink / Canvas" },
  { fg: "text-primary", bg: "bg-surface", label: "Ink / Surface" },
  { fg: "text-secondary", bg: "bg-surface", label: "Secondary / Surface" },
  {
    fg: "text-secondary",
    bg: "bg-surface-muted",
    label: "Secondary / Muted",
  },
  {
    fg: "primary-foreground",
    bg: "color-primary",
    label: "Ink / Electric Yellow",
  },
  {
    fg: "pink-foreground",
    bg: "color-pink",
    label: "Ink / Hot Pink",
  },
  {
    fg: "cyan-foreground",
    bg: "color-cyan",
    label: "Ink / Electric Cyan",
  },
  { fg: "semantic-impact", bg: "bg-surface", label: "Impact / Surface" },
  { fg: "semantic-success", bg: "bg-surface", label: "Success / Surface" },
  { fg: "semantic-info", bg: "bg-surface", label: "Info / Surface" },
  { fg: "semantic-warning", bg: "bg-surface", label: "Warning / Surface" },
] as const;

type Measurement = {
  label: string;
  fg: string;
  bg: string;
  ratio: number | null;
};

function readVar(element: Element, token: string): string {
  return getComputedStyle(element).getPropertyValue(`--${token}`).trim();
}

export function TokenSwatches() {
  return (
    <ComponentSection
      id="tokens"
      index="01"
      title="Tokens"
      description="Alle Farbwerte stammen aus globals.css. Die Kontrastwerte werden live aus den berechneten CSS-Variablen gemessen (WCAG 2.2 AA, normaler Text 4.5:1)."
    >
      <SwatchGrid />
    </ComponentSection>
  );
}

function SwatchGrid() {
  const probeRef = useRef<HTMLDivElement>(null);
  const [measurements, setMeasurements] = useState<Measurement[]>([]);

  useEffect(() => {
    const probe = probeRef.current;
    if (!probe) return;

    const next = CONTRAST_PAIRS.map(({ fg, bg, label }) => {
      const foreground = readVar(probe, fg);
      const background = readVar(probe, bg);
      return {
        label,
        fg: foreground,
        bg: background,
        ratio: contrastRatio(foreground, background),
      };
    });

    setMeasurements(next);
  }, []);

  return (
    <div ref={probeRef} className="space-y-4">
      <Example caption="Palette">
        {COLOUR_TOKENS.map((token) => (
          <div key={token} className="w-28 space-y-1">
            <div
              className="brutal-shadow-sm h-11 rounded-sm border-2 border-strong"
              style={{ background: `var(--${token})` }}
            />
            <code className="block font-mono text-[10px] break-all text-secondary">
              --{token}
            </code>
          </div>
        ))}
      </Example>

      <div className="space-y-1.5">
        <p className="font-mono text-[11px] font-bold text-secondary uppercase">
          Kontrast (WCAG 2.2 AA)
        </p>
        <ul className="space-y-1.5">
          {measurements.map((entry) => {
            const passes =
              entry.ratio !== null && entry.ratio >= AA_NORMAL_TEXT;
            return (
              <li
                key={entry.label}
                className="flex items-center justify-between gap-2 text-xs"
              >
                <span className="font-medium">{entry.label}</span>
                <span className="flex items-center gap-2">
                  <code className="font-mono font-semibold text-secondary tabular-nums">
                    {entry.ratio === null ? "–" : `${entry.ratio.toFixed(2)}:1`}
                  </code>
                  <Badge variant={passes ? "success" : "error"}>
                    {entry.ratio === null ? "?" : passes ? "AA" : "unter AA"}
                  </Badge>
                </span>
              </li>
            );
          })}
        </ul>
        {measurements.length === 0 ? (
          <p className="text-xs text-secondary">Wird gemessen…</p>
        ) : null}
      </div>
    </div>
  );
}
