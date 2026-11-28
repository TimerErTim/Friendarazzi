"use client";

import { HelpCircle } from "lucide-react";

import { GameStatus, VisibilityMode } from "@/api/friendarazzi-api";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";

function phaseSummary(status: GameStatus): {
  title: string;
  body: string;
} {
  switch (status) {
    case GameStatus.Lobby:
      return {
        title: "Phase 1 · Lobby",
        body: "Teilt den Einladungslink, bis mindestens 3 Mitspieler dabei sind. Der Host legt die Rastergröße und den Sichtbarkeitsmodus fest.",
      };
    case GameStatus.Drafting:
      return {
        title: "Phase 2 · Entwurf (Prompts)",
        body: "Schreibe Aufträge über alle anderen Mitspieler (oder fülle sie per Quick-Start Deck aus). Deine eigenen Aufträge und Aufträge über dich landen niemals auf deinem eigenen Brett!",
      };
    case GameStatus.Active:
      return {
        title: "Phase 3 · Die Foto-Jagd",
        body: "Erwische deine Freunde unauffällig bei den Aufgaben auf deinem Brett und lade ein Beweisfoto hoch. Wer zuerst eine komplette Reihe, Spalte oder Diagonale voll hat, gewinnt Gold!",
      };
    case GameStatus.Completed:
      return {
        title: "Phase 4 · Finale & Rückblick",
        body: "Alle Bretter und versteckten Fotos sind freigeschaltet. Entdecke Auszeichnungen, die 1:1 Zeitachse und alle Beweisfotos.",
      };
  }
}

function visibilityRule(mode: VisibilityMode): string {
  switch (mode) {
    case VisibilityMode.Undercover:
      return "Undercover: Alle Beweisfotos bleiben bis zum Bingo komplett geheim.";
    case VisibilityMode.BlurredTeaser:
      return "Blurred Teaser: Andere sehen nur stark verpixelte Vorschaubilder ohne Ziel und Text.";
    case VisibilityMode.LiveFeed:
      return "Live-Feed: Jedes Beweisfoto erscheint sofort für alle im Aktivitäten-Stream.";
  }
}

export function GameRulesDialog({
  status,
  visibilityMode,
}: {
  status: GameStatus;
  visibilityMode: VisibilityMode;
}) {
  const current = phaseSummary(status);

  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          aria-label="Spielregeln anzeigen"
          title="Spielregeln & Hilfe"
        >
          <HelpCircle className="size-4" aria-hidden="true" />
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>So funktioniert Friendarazzi</DialogTitle>
          <DialogDescription>
            Kurzübersicht zur aktuellen Spielphase und deinen Regeln.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-3 text-sm">
          <div className="rounded-sm border-2 border-strong bg-primary-subtle p-3.5">
            <p className="font-display text-sm text-ink">{current.title}</p>
            <p className="mt-1 font-medium text-ink">{current.body}</p>
          </div>

          <div className="rounded-sm border-2 border-strong bg-cyan-subtle p-3.5">
            <p className="font-display text-sm text-ink">Sichtbarkeitsmodus</p>
            <p className="mt-1 font-medium text-ink">
              {visibilityRule(visibilityMode)}
            </p>
          </div>

          <div className="rounded-sm border-2 border-strong bg-surface p-3.5">
            <p className="font-display text-sm text-ink">
              Faire Brett-Verteilung
            </p>
            <p className="mt-1 font-medium text-secondary">
              Jeder erstellt gleich viele Aufträge pro Mitspieler. Beim Mischen
              erhält niemand eigene Aufträge oder Aufgaben über sich selbst.
            </p>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
