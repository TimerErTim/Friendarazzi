"use client";

import { Ticket, Users } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";

import { useAuth } from "@/api/auth-provider";
import { apiErrorMessage } from "@/api/errors";
import { useFriendarazziMutations, useGameByCodeQuery } from "@/api/hooks";
import { PageHeader } from "@/components/layout/page-header";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Field } from "@/components/ui/field";
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
    <div className="mx-auto w-full max-w-lg space-y-6 p-4 sm:p-6">
      <PageHeader
        register={["Friendarazzi", "Einladung"]}
        title="Spiel beitreten"
        description="Gib einen Einladungscode ein oder bestätige die Einladung deiner Freunde."
      />

      {phase.status === "anonymous" ? (
        <Card>
          <CardHeader className="bg-primary-subtle">
            <CardTitle>Anmeldung erforderlich</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4 pt-5">
            <p className="text-sm font-medium text-secondary">
              Bitte melde dich an oder erstelle ein Konto, um dieser Lobby
              beizutreten.
            </p>
            <div className="flex flex-wrap gap-3">
              <Button asChild>
                <Link href="/anmeldung">Anmelden</Link>
              </Button>
              <Button variant="secondary" asChild>
                <Link href="/registrierung">Registrieren</Link>
              </Button>
            </div>
          </CardContent>
        </Card>
      ) : (
        <Card>
          <CardContent className="space-y-4 pt-6">
            <Field label="Einladungscode">
              <Input
                value={manualCode}
                onChange={(e) => setManualCode(e.target.value.toUpperCase())}
                placeholder="z.B. A7K9P2"
                className="font-mono text-base font-bold tracking-widest uppercase"
                maxLength={16}
              />
            </Field>

            {preview.isLoading ? (
              <Skeleton className="h-28 w-full" />
            ) : preview.isError && activeCode ? (
              <Alert variant="error">
                {apiErrorMessage(preview.error, {
                  fallback: "Keine Lobby mit diesem Code gefunden.",
                })}
              </Alert>
            ) : preview.data ? (
              <div className="brutal-shadow-sm space-y-4 rounded-md border-2 border-strong bg-cyan-subtle p-4">
                <div className="flex items-start justify-between gap-2">
                  <div>
                    <h2 className="font-display text-lg text-ink">
                      {preview.data.title}
                    </h2>
                    <p className="font-mono text-xs font-semibold text-secondary">
                      {preview.data.gridSize}×{preview.data.gridSize} Bingo ·{" "}
                      <span className="inline-flex items-center gap-1">
                        <Users className="size-3.5" aria-hidden="true" />
                        {preview.data.playerCount} Spieler
                      </span>
                    </p>
                  </div>
                  <Badge variant="primary">{preview.data.status}</Badge>
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
                    : "Jetzt beitreten"}
                </Button>
              </div>
            ) : null}
          </CardContent>
        </Card>
      )}
    </div>
  );
}
