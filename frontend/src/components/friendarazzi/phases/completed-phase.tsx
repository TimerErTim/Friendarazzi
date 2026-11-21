"use client";

import { Crown, Flame, Ghost, Target, Timer, Zap } from "lucide-react";
import { useState } from "react";

import {
  type CaptureFeedItem,
  type GameDetail,
  SuperlativeKind,
} from "@/api/friendarazzi-api";
import { BingoBoard } from "@/components/friendarazzi/bingo-board";
import { CaptureGalleryCard } from "@/components/friendarazzi/capture-gallery-card";
import { GameTimeline } from "@/components/friendarazzi/game-timeline";
import { PhotoLightbox } from "@/components/friendarazzi/photo-lightbox";
import {
  PlayerAvatar,
  PlayerBadgeLink,
} from "@/components/friendarazzi/player-avatar";
import { Badge } from "@/components/ui/badge";
import { CalloutNote } from "@/components/ui/brutal-elements";
import { Card, CardContent } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

function superlativeMeta(kind: SuperlativeKind, metric: number) {
  switch (kind) {
    case SuperlativeKind.MostWanted:
      return {
        label: "Meistgesucht (Most Wanted)",
        detail: `${metric}× erwischt`,
        icon: Target,
      };
    case SuperlativeKind.Ghost:
      return {
        label: "Phantom (Ghost)",
        detail: `${metric}× erwischt`,
        icon: Ghost,
      };
    case SuperlativeKind.QuickestDraw:
      return {
        label: "Schnellster Finger (Quickest Draw)",
        detail: `nach ${metric} Sek.`,
        icon: Zap,
      };
    case SuperlativeKind.Sniper:
      return {
        label: "Scharfschütze (Sniper)",
        detail: `${metric} Treffer in 1 Std.`,
        icon: Flame,
      };
  }
}

export function CompletedPhase({ game }: { game: GameDetail }) {
  const [selectedCapture, setSelectedCapture] =
    useState<CaptureFeedItem | null>(null);

  return (
    <div className="space-y-8">
      {/* Winner Podium Banner */}
      {game.winner ? (
        <div className="brutal-shadow-lg rounded-md border-[2.5px] border-strong bg-primary p-6 text-primary-foreground">
          <div className="flex flex-wrap items-center justify-between gap-4">
            <div className="flex items-center gap-4">
              <PlayerAvatar user={game.winner} size="lg" />
              <div className="space-y-1">
                <Badge variant="outline" className="gap-1">
                  <Crown className="size-3.5" aria-hidden="true" />
                  BINGO-SIEGER
                </Badge>
                <h2 className="font-display text-2xl tracking-tight sm:text-3xl">
                  {game.winner.displayName} gewinnt „{game.title}“!
                </h2>
                <p className="font-mono text-xs font-bold">
                  {game.winner.effectiveTitle} · {game.winner.goldMedalCount}{" "}
                  Goldmedaillen (+1)
                </p>
              </div>
            </div>
            {game.startedAt && game.endedAt ? (
              <div className="inline-flex items-center gap-1.5 rounded-sm border-2 border-strong bg-surface px-3 py-1 font-mono text-xs font-bold text-ink tabular-nums">
                <Timer className="size-4" aria-hidden="true" />
                {new Date(game.endedAt).toLocaleString("de-AT")}
              </div>
            ) : null}
          </div>
        </div>
      ) : null}

      {/* Automated Post-Game Superlatives */}
      {game.superlatives.length > 0 ? (
        <section className="space-y-3">
          <h2 className="font-display text-lg tracking-tight text-ink">
            Auszeichnungen
          </h2>
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
            {game.superlatives.map((sup) => {
              const meta = superlativeMeta(sup.kind, sup.metricValue);
              const Icon = meta.icon;
              return (
                <Card key={sup.kind}>
                  <CardContent className="space-y-2 pt-4">
                    <div className="flex items-center gap-2 font-mono text-xs font-bold tracking-wide text-secondary uppercase">
                      <Icon className="size-4 text-ink" aria-hidden="true" />
                      <span>{meta.label}</span>
                    </div>
                    <PlayerBadgeLink
                      user={sup.user}
                      compact
                      subtitle={meta.detail}
                    />
                  </CardContent>
                </Card>
              );
            })}
          </div>
        </section>
      ) : null}

      {/* 1:1 Proportional Timeline */}
      <GameTimeline
        startedAt={game.startedAt}
        endedAt={game.endedAt}
        captures={game.feed}
      />

      {/* Tabbed Recap: Gallery vs All Unlocked Boards */}
      <Tabs defaultValue="gallery">
        <TabsList>
          <TabsTrigger value="gallery">
            Spiel-Galerie ({game.feed.length})
          </TabsTrigger>
          <TabsTrigger value="boards">
            Aufgedeckte Bretter ({game.allBoards.length})
          </TabsTrigger>
        </TabsList>

        <TabsContent value="gallery" className="pt-2">
          {game.feed.length === 0 ? (
            <CalloutNote label="Keine Fotos">
              In diesem Spiel wurden keine Fotos eingereicht.
            </CalloutNote>
          ) : (
            <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
              {game.feed.map((cap) => (
                <CaptureGalleryCard
                  key={cap.id}
                  capture={cap}
                  onSelect={setSelectedCapture}
                />
              ))}
            </div>
          )}
        </TabsContent>

        <TabsContent value="boards" className="pt-2">
          <div className="grid gap-6 xl:grid-cols-2">
            {game.allBoards.map((board) => (
              <div key={board.id} className="space-y-3">
                <div className="brutal-shadow-sm flex items-center justify-between gap-2 rounded-md border-2 border-strong bg-surface px-4 py-2.5">
                  <PlayerBadgeLink user={board.owner} compact />
                  {board.hasBingo ? (
                    <Badge variant="primary">Bingo!</Badge>
                  ) : null}
                </div>
                <BingoBoard
                  board={board}
                  gridSize={game.gridSize}
                  feed={game.feed}
                  onViewCapture={(cap) => setSelectedCapture(cap)}
                />
              </div>
            ))}
          </div>
        </TabsContent>
      </Tabs>

      <PhotoLightbox
        capture={selectedCapture}
        open={Boolean(selectedCapture)}
        onOpenChange={(open) => {
          if (!open) setSelectedCapture(null);
        }}
      />
    </div>
  );
}
