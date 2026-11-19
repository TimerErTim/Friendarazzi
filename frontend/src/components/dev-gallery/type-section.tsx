"use client";

import { Example } from "@/components/dev-gallery/component-section";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";

export function TypeSection() {
  return (
    <section
      id="type"
      aria-labelledby="type-heading"
      className="scroll-mt-6 space-y-4"
    >
      <header className="flex items-baseline gap-4 border-b-2 border-strong pb-3">
        <span
          aria-hidden="true"
          className="rounded-sm border-2 border-strong bg-primary px-1.5 py-0.5 font-mono text-xs font-bold text-primary-foreground tabular-nums"
        >
          02
        </span>
        <div className="min-w-0 flex-1 space-y-1">
          <h2
            id="type-heading"
            className="font-display text-lg tracking-tight text-ink"
          >
            Schrift
          </h2>
          <p className="max-w-prose text-sm font-medium text-secondary">
            Archivo Black trägt Display-Überschriften und die Wortmarke, Space
            Grotesk den Fließtext und alle Steuerelemente, JetBrains Mono Codes
            und Zeitstempel.
          </p>
        </div>
      </header>

      <div className="grid gap-4 lg:grid-cols-2">
        <div className="light brutal-shadow space-y-5 rounded-md border-[2.5px] border-strong bg-canvas p-4 text-ink">
          <TypeSpecimen />
        </div>
        <div className="dark brutal-shadow space-y-5 rounded-md border-[2.5px] border-strong bg-canvas p-4 text-ink">
          <TypeSpecimen />
        </div>
      </div>
    </section>
  );
}

function TypeSpecimen() {
  return (
    <>
      <div className="flex flex-wrap items-center gap-2">
        <Badge variant="primary">Archivo Black · Display</Badge>
        <Badge variant="info">Space Grotesk · UI</Badge>
        <Badge variant="outline">JetBrains Mono · Werte</Badge>
      </div>

      <Example caption="Überschrift und Fließtext" className="block">
        <div className="w-full space-y-2">
          <p className="font-display text-2xl leading-tight tracking-tight">
            Neuer Eintrag — Bereich Süd
          </p>
          <p className="text-sm font-medium">
            Jeden Eintrag einzeln erfassen. Der erste Wert und der Folgewert
            werden daraus berechnet.
          </p>
          <p className="text-sm text-secondary">
            Nebentext: 32 Einträge, 4 Kategorien, Abstand 42 Schritte.
          </p>
        </div>
      </Example>

      <Example caption="Tabelle und Bedienelemente" className="block">
        <div className="w-full space-y-3">
          <table className="w-full border-collapse text-sm">
            <thead className="border-b-2 border-strong text-left font-mono text-[10px] tracking-[0.08em] text-secondary uppercase">
              <tr>
                <th scope="col" className="py-1.5 font-bold">
                  Nr.
                </th>
                <th scope="col" className="py-1.5 font-bold">
                  Status
                </th>
                <th scope="col" className="py-1.5 text-right font-bold">
                  Wert
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-subtle">
              {SAMPLE_ROWS.map((row) => (
                <tr key={row.nr}>
                  <td className="py-1.5 font-mono text-xs font-bold tabular-nums">
                    {row.nr}
                  </td>
                  <td className="py-1.5 font-medium">{row.status}</td>
                  <td className="py-1.5 text-right font-mono font-bold tabular-nums">
                    {row.wert}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          <div className="flex flex-wrap items-center gap-2">
            <Button size="sm">Eintrag erfassen</Button>
            <Button size="sm" variant="secondary">
              Abbrechen
            </Button>
            <span className="font-mono text-xs font-bold text-secondary tabular-nums">
              0712 · 284 / 320
            </span>
          </div>
        </div>
      </Example>

      <Example caption="Zeichen" className="block">
        <p className="w-full text-sm font-medium">
          ÄÖÜäöüß · 0123456789 · € · → · ✓ · — »Kategorie«
        </p>
      </Example>
    </>
  );
}

const SAMPLE_ROWS = [
  { nr: "1", status: "Aktiv", wert: "1" },
  { nr: "2", status: "Prüfung", wert: "5" },
  { nr: "3", status: "Leer", wert: "0" },
];
