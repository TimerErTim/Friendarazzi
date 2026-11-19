"use client";

import { ArrowRight, Camera, Ticket, Users } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";

import { useAuth } from "@/api/auth-provider";
import { apiErrorMessage } from "@/api/errors";
import { useFriendarazziMutations, useGameByCodeQuery } from "@/api/hooks";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { CodeTag } from "@/components/ui/brutal-elements";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { gameHref } from "@/lib/navigation";
import { useQueryParam } from "@/lib/use-query-param";

export default function JoinPage() {
  const phase = useAuth();
  const router = useRouter();
  const codeParam = useQueryParam("code");
  const [manualCode, setManualCode] = useState("");

  useEffect(() => {
    if (codeParam) {
      setManualCode(codeParam.toUpperCase());
    }
  }, [codeParam]);

  const activeCode = (codeParam || manualCode).trim().toUpperCase();
  const preview = useGameByCodeQuery(activeCode || null);
  const mutations = useFriendarazziMutations();

  async function handleJoin() {
    if (!activeCode) return;
    const game = await mutations.joinGame.mutateAsync(activeCode);
    router.replace(gameHref(game.id));
  }

  return (
    <div className="mx-auto flex w-full max-w-xl flex-col justify-center px-4 py-10 sm:px-6 lg:py-16">
      {/* Match Ticket Container */}
      <div className="brutal-shadow-lg overflow-hidden rounded-md border-[2.5px] border-strong bg-surface">
        {/* Ticket Header Stub */}
        <div className="border-b-[2.5px] border-strong bg-primary p-6 text-primary-foreground">
          <div className="flex items-center justify-between gap-2">
            <span className="inline-flex items-center gap-1.5 rounded-sm border-2 border-strong bg-surface px-2.5 py-0.5 font-mono text-xs font-bold text-ink uppercase">
              <Ticket className="size-3.5" aria-hidden="true" />
              Eintrittskarte
            </span>
            <CodeTag tone="surface">Lobby-Join</CodeTag>
          </div>
          <h1 className="mt-3 font-display text-2xl tracking-tight sm:text-3xl">
            Spiel beitreten
          </h1>
          <p className="mt-1 text-sm font-bold">
            Gib deinen Einladungscode ein und spring direkt in die Lobby.
          </p>
        </div>

        {/* Ticket Body */}
        <div className="space-y-5 p-6">
          {phase.status === "anonymous" ? (
            <div className="space-y-4">
              <p className="text-sm font-medium text-secondary">
                Um einer Lobby beizutreten und Beweisfotos zu knipsen, brauchst
                du ein Friendarazzi-Konto.
              </p>
              {activeCode ? (
                <div className="rounded-sm border-2 border-strong bg-cyan-subtle px-4 py-3 font-mono text-sm font-bold text-ink">
                  Dein Code: {activeCode}
                </div>
              ) : null}
              <div className="flex flex-wrap gap-3">
                <Button size="lg" asChild>
                  <Link href="/anmeldung">
                    Anmelden
                    <ArrowRight aria-hidden="true" />
                  </Link>
                </Button>
                <Button variant="secondary" size="lg" asChild>
                  <Link href="/registrierung">Konto erstellen</Link>
                </Button>
              </div>
            </div>
          ) : (
            <>
              <div className="space-y-2">
                <label
                  htmlFor="join-ticket-code"
                  className="block font-mono text-xs font-bold tracking-wider text-secondary uppercase"
                >
                  Einladungscode
                </label>
                <Input
                  id="join-ticket-code"
                  value={manualCode}
                  onChange={(e) => setManualCode(e.target.value.toUpperCase())}
                  placeholder="A7K9P2"
                  className="h-14 text-center font-mono text-2xl font-bold tracking-[0.25em] uppercase"
                  maxLength={16}
                />
              </div>

              {preview.isLoading ? (
                <Skeleton className="h-32 w-full" />
              ) : preview.isError && activeCode ? (
                <Alert variant="error">
                  {apiErrorMessage(preview.error, {
                    fallback: "Keine Lobby mit diesem Code gefunden.",
                  })}
                </Alert>
              ) : preview.data ? (
                <div className="space-y-4 border-t-2 border-dashed border-strong pt-5">
                  <div className="brutal-shadow-sm rounded-md border-2 border-strong bg-cyan-subtle p-4">
                    <div className="flex items-start justify-between gap-3">
                      <div className="space-y-1">
                        <h2 className="font-display text-xl text-ink">
                          {preview.data.title}
                        </h2>
                        <div className="flex flex-wrap items-center gap-3 font-mono text-xs font-bold text-ink">
                          <span className="inline-flex items-center gap-1">
                            <Camera className="size-3.5" aria-hidden="true" />
                            {preview.data.gridSize}×{preview.data.gridSize}
                          </span>
                          <span className="inline-flex items-center gap-1">
                            <Users className="size-3.5" aria-hidden="true" />
                            {preview.data.playerCount} Spieler
                          </span>
                        </div>
                      </div>
                      <Badge variant="primary">{preview.data.status}</Badge>
                    </div>
                  </div>

                  {mutations.joinGame.isError ? (
                    <Alert variant="error">
                      {apiErrorMessage(mutations.joinGame.error, {
                        fallback: "Beitritt fehlgeschlagen.",
                      })}
                    </Alert>
                  ) : null}

                  <Button
                    className="w-full"
                    size="lg"
                    disabled={mutations.joinGame.isPending}
                    onClick={() => void handleJoin()}
                  >
                    <Ticket aria-hidden="true" />
                    {mutations.joinGame.isPending
                      ? "Tritt bei …"
                      : "Jetzt mitspielen!"}
                  </Button>
                </div>
              ) : (
                <p className="text-center font-mono text-xs font-medium text-secondary">
                  Code eingeben, um die Match-Vorschau zu laden.
                </p>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
