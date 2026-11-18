"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";

import { PageHeader } from "@/components/layout/page-header";
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
    <div className="mx-auto w-full max-w-xl space-y-6 p-4 sm:p-6">
      <PageHeader
        register={["Friendarazzi", "Fehler"]}
        title="Seite nicht gefunden"
        plate="404"
        description="Diese Seite existiert nicht oder wurde verschoben."
      />
      <Card>
        <CardContent className="flex flex-wrap items-center justify-between gap-4 pt-6">
          <CodeTag tone="pink">HTTP 404</CodeTag>
          <Button asChild>
            <Link href={paths.home}>Zur Startseite</Link>
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
