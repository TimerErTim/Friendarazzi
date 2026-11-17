"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";

import { PageHeader } from "@/components/layout/page-header";
import { Button } from "@/components/ui/button";
import { gameHref, joinHref, paths, userProfileHref } from "@/lib/navigation";

/**
 * Static export fallback: supports pretty share URLs (`/join/<code>`, `/games/<id>`,
 * `/users/<id>`) by redirecting to the query-param static pages, and serves a real
 * 404 page for everything else.
 */
export default function NotFoundPage() {
  const router = useRouter();
  const [redirecting, setRedirecting] = useState(false);

  useEffect(() => {
    const path = window.location.pathname.replace(/\/+$/, "");
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
      <div className="mx-auto w-full max-w-xl p-6 text-sm text-secondary">
        Weiterleitung …
      </div>
    );
  }

  return (
    <div className="mx-auto w-full max-w-xl space-y-6 p-6">
      <PageHeader
        register={["Friendarazzi"]}
        title="Seite nicht gefunden"
        description="Diese Seite existiert nicht oder wurde verschoben."
      />
      <Button asChild>
        <Link href={paths.home}>Zur Startseite</Link>
      </Button>
    </div>
  );
}
