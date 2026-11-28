"use client";

import {
  Crown,
  Flame,
  Ghost,
  LayoutGrid,
  List,
  Target,
  Zap,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

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
import {
  MatchTargetOverview,
  TargetGroupedBoard,
} from "@/components/friendarazzi/target-grouped-board";
import { Badge } from "@/components/ui/badge";
import { CalloutNote } from "@/components/ui/brutal-elements";
import { Card, CardContent } from "@/components/ui/card";
import { SearchInput } from "@/components/ui/search-input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import {
  aggregateMatchByTarget,
  type BoardViewMode,
  loadBoardViewMode,
  saveBoardViewMode,
} from "@/lib/board-grouping";

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

export function CompletedPhase({
  game,
  initialCaptureId = null,
  onConsumeInitialCapture,
}: {
  game: GameDetail;
  initialCaptureId?: string | null;
  onConsumeInitialCapture?: () => void;
}) {
  const [selectedCapture, setSelectedCapture] =
    useState<CaptureFeedItem | null>(null);
  const [boardViewMode, setBoardViewMode] = useState<BoardViewMode>(() =>
    loadBoardViewMode(),
  );
  const [targetSearch, setTargetSearch] = useState("");
  const openedCaptureRef = useRef<string | null>(null);

  const matchTargetOverview = useMemo(
    () => aggregateMatchByTarget(game.allBoards, game.feed, targetSearch),
    [game.allBoards, game.feed, targetSearch],
  );

  function handleBoardViewChange(next: string) {
    if (next !== "grid" && next !== "list") return;
    setBoardViewMode(next);
    saveBoardViewMode(next);
  }

  useEffect(() => {
    if (!initialCaptureId || openedCaptureRef.current === initialCaptureId) {
      return;
    }
    const found = game.feed.find((item) => item.id === initialCaptureId);
    if (found) {
      openedCaptureRef.current = initialCaptureId;
      setSelectedCapture(found);
    }
  }, [initialCaptureId, game.feed]);

  return (
    <div className="space-y-8">
      {/* Winner Podium Banner */}
      {game.winner ? (
        <div className="brutal-shadow-lg rounded-md border-[2.5px] border-strong bg-primary p-5 text-primary-foreground sm:p-6">
          <div className="flex items-center gap-4">
            <PlayerAvatar user={game.winner} size="lg" />
            <div className="space-y-1">
              <Badge variant="outline" className="gap-1">
                <Crown className="size-3.5" aria-hidden="true" />
                BINGO-SIEGER
              </Badge>
              <h2 className="font-display text-2xl tracking-tight sm:text-3xl">
                {game.winner.displayName} hat gewonnen!
              </h2>
              <p className="font-mono text-xs font-bold">
                {game.winner.effectiveTitle} · {game.winner.goldMedalCount}{" "}
                Goldmedaillen (+1)
              </p>
            </div>
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
                <Card key={sup.kind} className="overflow-hidden">
                  <CardContent className="space-y-2 pt-4">
                    <div className="flex items-center gap-2 font-mono text-xs font-bold tracking-wide text-secondary uppercase">
                      <span className="inline-flex size-6 items-center justify-center rounded-sm border border-strong bg-primary-subtle text-ink">
                        <Icon className="size-3.5" aria-hidden="true" />
                      </span>
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

      {/* Tabbed Recap: Gallery, All Unlocked Boards, and Target Overview (#26) */}
      <Tabs defaultValue="gallery">
        <TabsList>
          <TabsTrigger value="gallery">
            Spiel-Galerie ({game.feed.length})
          </TabsTrigger>
          <TabsTrigger value="boards">
            Aufgedeckte Bretter ({game.allBoards.length})
          </TabsTrigger>
          <TabsTrigger value="by-target">
            Nach Ziel ({matchTargetOverview.length})
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

        <TabsContent value="boards" className="space-y-4 pt-2">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <p className="font-mono text-xs font-bold text-secondary">
              Darstellung der Spielbretter wählen:
            </p>
            <ToggleGroup
              type="single"
              value={boardViewMode}
              onValueChange={handleBoardViewChange}
              aria-label="Brett-Darstellung im Rückblick wählen"
            >
              <ToggleGroupItem value="grid">
                <LayoutGrid className="size-4" aria-hidden="true" />
                <span>Raster</span>
              </ToggleGroupItem>
              <ToggleGroupItem value="list">
                <List className="size-4" aria-hidden="true" />
                <span>Nach Ziel</span>
              </ToggleGroupItem>
            </ToggleGroup>
          </div>

          <div
            className={
              boardViewMode === "grid"
                ? "grid gap-6 xl:grid-cols-2"
                : "space-y-6"
            }
          >
            {game.allBoards.map((board) => (
              <div key={board.id} className="space-y-3">
                <div className="brutal-shadow-sm flex items-center justify-between gap-2 rounded-md border-2 border-strong bg-surface px-4 py-2.5">
                  <PlayerBadgeLink user={board.owner} compact />
                  {board.hasBingo ? <Badge variant="info">Bingo!</Badge> : null}
                </div>
                {boardViewMode === "list" ? (
                  <TargetGroupedBoard
                    board={board}
                    feed={game.feed}
                    onViewCapture={(cap) => setSelectedCapture(cap)}
                  />
                ) : (
                  <BingoBoard
                    board={board}
                    gridSize={game.gridSize}
                    feed={game.feed}
                    onViewCapture={(cap) => setSelectedCapture(cap)}
                  />
                )}
              </div>
            ))}
          </div>
        </TabsContent>

        <TabsContent value="by-target" className="space-y-4 pt-2">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <p className="text-sm font-medium text-secondary">
              Alle Aufträge und Treffer dieses Spiels nach Zielperson gruppiert.
            </p>
            <SearchInput
              value={targetSearch}
              onChange={(e) => setTargetSearch(e.target.value)}
              placeholder="Ziel, Auftrag oder Fänger …"
              aria-label="Rückblick nach Zielperson durchsuchen"
            />
          </div>
          <MatchTargetOverview
            aggregates={matchTargetOverview}
            onViewCapture={(cap) => setSelectedCapture(cap)}
          />
        </TabsContent>
      </Tabs>

      <PhotoLightbox
        capture={selectedCapture}
        open={Boolean(selectedCapture)}
        onOpenChange={(open) => {
          if (!open) {
            setSelectedCapture(null);
            if (initialCaptureId) {
              onConsumeInitialCapture?.();
            }
          }
        }}
      />
    </div>
  );
}
