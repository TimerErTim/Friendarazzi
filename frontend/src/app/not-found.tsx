"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";

import { CodeTag } from "@/components/ui/brutal-elements";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  gameHref,
  joinHref,
  paths,
  stripBasePath,
  userProfileHref,
} from "@/lib/navigation";

export default function NotFoundPage() {
  const router = useRouter();
  const [redirecting, setRedirecting] = useState(false);

  useEffect(() => {
    const path = stripBasePath(window.location.pathname).replace(/\/+$/, "");
    const joinMatch = path.match(/^\/join\/([^/]+)$/);
    if (joinMatch?.[1]) {
      setRedirecting(true);
      router.replace(joinHref(decodeURIComponent(joinMatch[1])));
      return;
    }
    const gameMatch = path.match(/^\/games\/([^/]+)$/);
    if (gameMatch?.[1]) {
      setRedirecting(true);
      router.replace(gameHref(decodeURIComponent(gameMatch[1])));
      return;
    }
    const userMatch = path.match(/^\/users\/([^/]+)$/);
    if (userMatch?.[1]) {
      setRedirecting(true);
      router.replace(userProfileHref(decodeURIComponent(userMatch[1])));
    }
  }, [router]);

  if (redirecting) {
    return (
      <div className="mx-auto w-full max-w-xl p-6 font-mono text-sm font-bold text-secondary">
        Weiterleitung …
      </div>
    );
  }

  return (
    <div className="mx-auto flex w-full max-w-lg flex-col justify-center px-4 py-16">
      <Card>
        <CardContent className="space-y-4 pt-6 text-center">
          <CodeTag tone="pink">HTTP 404</CodeTag>
          <h1 className="font-display text-3xl tracking-tight text-ink">
            Seite nicht gefunden
          </h1>
          <p className="text-sm font-medium text-secondary">
            Diese Seite existiert nicht oder wurde verschoben.
          </p>
          <div className="pt-2">
            <Button size="lg" asChild>
              <Link href={paths.home}>Zur Startseite</Link>
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
