"use client";

import {
  Award,
  Bell,
  CircleAlert,
  LogIn,
  LogOut,
  Settings2,
  Ticket,
  UserRound,
} from "lucide-react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useAuth } from "@/api/auth-provider";
import {
  useAuthMutations,
  useBenutzerQuery,
  useNotificationsQuery,
} from "@/api/hooks";
import { Wordmark } from "@/components/brand/wordmark";
import { PlayerAvatar } from "@/components/friendarazzi/player-avatar";
import { Alert } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/cn";
import { isNavItemActive, paths, primaryNavItems } from "@/lib/navigation";

export function AppShell({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const phase = useAuth();
  const benutzer = useBenutzerQuery();
  const notifications = useNotificationsQuery();
  const auth = useAuthMutations();
  const unreadCount = notifications.data?.unreadCount ?? 0;
  const normalizedPath = pathname.replace(/\/$/, "") || "/";
  const onAuthRoute = ["/anmeldung", "/registrierung", "/passwort"].includes(
    normalizedPath,
  );
  const onLandingPage = normalizedPath === "/";
  const isAuthenticated = phase.status === "authenticated";

  return (
    <div className="flex min-h-screen flex-col bg-canvas text-ink">
      <a
        href="#main"
        className="sr-only rounded-sm border-2 border-strong px-4 py-2 text-sm font-bold text-ink focus:not-sr-only focus:fixed focus:top-2 focus:left-2 focus:z-50 focus:bg-primary focus:text-primary-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
      >
        Zum Inhalt springen
      </a>

      {/* Sticky Top Navbar — no sidebar */}
      <header className="sticky top-0 z-40 border-b-[2.5px] border-strong bg-surface">
        <div className="mx-auto flex h-16 max-w-6xl items-center justify-between gap-4 px-4 sm:px-6">
          <Link
            href={paths.home}
            className="inline-flex min-h-11 items-center rounded-sm px-1 py-1 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
            aria-label="Friendarazzi — Startseite"
          >
            <Wordmark size="sm" showMark />
          </Link>

          {/* Center desktop navigation */}
          {isAuthenticated ? (
            <nav
              aria-label="Hauptnavigation"
              className="hidden items-center gap-1.5 md:flex"
            >
              {primaryNavItems.map((item) => {
                const Icon = item.icon;
                const active = isNavItemActive(
                  pathname,
                  item.href,
                  item.recursive,
                );
                return (
                  <Link
                    key={item.id}
                    href={item.href}
                    aria-current={active ? "page" : undefined}
                    className={cn(
                      "inline-flex min-h-11 items-center gap-2 rounded-sm border-2 px-3.5 py-1.5 text-sm font-bold transition-colors",
                      "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus",
                      active
                        ? "brutal-shadow-sm border-strong bg-primary text-primary-foreground"
                        : "border-transparent text-ink hover:border-strong hover:bg-surface-muted",
                    )}
                  >
                    <Icon className="size-4" aria-hidden="true" />
                    <span>{item.label}</span>
                  </Link>
                );
              })}
            </nav>
          ) : onLandingPage ? (
            <nav
              aria-label="Bereiche"
              className="hidden items-center gap-2 md:flex"
            >
              <a
                href="#so-funktionierts"
                className="inline-flex min-h-11 items-center rounded-sm px-3 text-sm font-bold text-ink hover:bg-primary-subtle focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
              >
                So läuft&apos;s
              </a>
              <a
                href="#modi"
                className="inline-flex min-h-11 items-center rounded-sm px-3 text-sm font-bold text-ink hover:bg-cyan-subtle focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
              >
                Modi
              </a>
              <a
                href="#loslegen"
                className="inline-flex min-h-11 items-center rounded-sm px-3 text-sm font-bold text-ink hover:bg-pink-subtle focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
              >
                Loslegen
              </a>
            </nav>
          ) : null}

          {/* Right action cluster */}
          <div className="flex shrink-0 items-center gap-2">
            {isAuthenticated && benutzer.data ? (
              <>
                <Link
                  href={paths.benachrichtigungen}
                  aria-label={
                    unreadCount > 0
                      ? `Benachrichtigungen (${unreadCount} ungelesen)`
                      : "Benachrichtigungen"
                  }
                  title="Benachrichtigungen"
                  className={cn(
                    "brutal-shadow-sm relative inline-flex min-h-11 min-w-11 items-center justify-center gap-1.5 rounded-sm border-2 border-strong px-2.5 py-1 text-xs font-bold transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus",
                    isNavItemActive(pathname, paths.benachrichtigungen, true)
                      ? "bg-primary text-primary-foreground"
                      : "bg-surface text-ink hover:bg-surface-muted",
                  )}
                >
                  <Bell className="size-4" aria-hidden="true" />
                  {unreadCount > 0 ? (
                    <span className="inline-flex min-w-5 items-center justify-center rounded-sm border border-strong bg-pink px-1 py-0.5 font-mono text-[10px] leading-none font-bold text-pink-foreground tabular-nums">
                      {unreadCount > 99 ? "99+" : unreadCount}
                    </span>
                  ) : null}
                </Link>
                <Link
                  href={paths.profil}
                  className="brutal-shadow-sm hidden min-h-11 items-center gap-1.5 rounded-sm border-2 border-strong bg-primary px-3 py-1 text-xs font-bold text-primary-foreground transition-colors hover:bg-primary-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus sm:inline-flex"
                  title="Goldmedaillen"
                >
                  <Award className="size-4" aria-hidden="true" />
                  <span className="font-mono tabular-nums">
                    {benutzer.data.goldMedalCount}
                  </span>
                </Link>
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button
                      size="md"
                      variant="secondary"
                      aria-label="Profilmenü öffnen"
                      disabled={
                        auth.logout.isPending || auth.deleteAccount.isPending
                      }
                      className="gap-2 pl-1.5"
                    >
                      <PlayerAvatar
                        user={benutzer.data}
                        size="sm"
                        linkToProfile={false}
                        showMedalBadge={false}
                      />
                      <span className="hidden max-w-28 truncate sm:inline">
                        {benutzer.data.displayName}
                      </span>
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end" className="w-64">
                    <DropdownMenuLabel className="normal-case">
                      <span className="block truncate text-sm font-bold text-ink">
                        {benutzer.data.displayName}
                      </span>
                      <span className="block truncate font-mono text-xs text-secondary">
                        {benutzer.data.effectiveTitle}
                      </span>
                    </DropdownMenuLabel>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem asChild>
                      <Link href={paths.profil}>
                        <UserRound aria-hidden="true" /> Mein Profil
                      </Link>
                    </DropdownMenuItem>
                    <DropdownMenuItem asChild>
                      <Link href={paths.benachrichtigungen}>
                        <Bell aria-hidden="true" /> Benachrichtigungen
                        {unreadCount > 0 ? ` (${unreadCount})` : ""}
                      </Link>
                    </DropdownMenuItem>
                    <DropdownMenuItem asChild>
                      <Link href={paths.optionen}>
                        <Settings2 aria-hidden="true" /> Optionen
                      </Link>
                    </DropdownMenuItem>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem
                      onSelect={() =>
                        auth.logout.mutate(undefined, {
                          onSuccess: () => window.location.assign("/"),
                        })
                      }
                    >
                      <LogOut aria-hidden="true" /> Abmelden
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </>
            ) : !onAuthRoute ? (
              <div className="flex items-center gap-2">
                <Button
                  size="sm"
                  variant="ghost"
                  asChild
                  className="hidden sm:inline-flex"
                >
                  <Link href={paths.join}>
                    <Ticket aria-hidden="true" />
                    Code
                  </Link>
                </Button>
                <Button size="sm" variant="secondary" asChild>
                  <Link href="/anmeldung">
                    <LogIn aria-hidden="true" />
                    Anmelden
                  </Link>
                </Button>
                <Button size="sm" asChild className="hidden sm:inline-flex">
                  <Link href="/registrierung">Starten</Link>
                </Button>
              </div>
            ) : null}
          </div>
        </div>
      </header>

      <main
        id="main"
        tabIndex={-1}
        className={cn(
          "brutal-grid flex-1",
          isAuthenticated ? "pb-20 md:pb-8" : "",
        )}
      >
        {phase.status === "unavailable" ? (
          <div className="mx-auto w-full max-w-6xl px-4 pt-6 sm:px-6">
            <Alert variant="warning" icon={<CircleAlert aria-hidden="true" />}>
              <p>
                Deine Sitzung konnte nicht überprüft werden. Bitte lade die
                Seite erneut oder melde dich an.
              </p>
            </Alert>
          </div>
        ) : null}
        {children}
      </main>

      {/* Mobile Bottom Navigation Bar when authenticated */}
      {isAuthenticated ? (
        <nav
          aria-label="Mobile Navigation"
          className="fixed inset-x-0 bottom-0 z-40 grid grid-cols-4 border-t-[2.5px] border-strong bg-surface md:hidden"
        >
          {primaryNavItems.map((item) => {
            const Icon = item.icon;
            const active = isNavItemActive(pathname, item.href, item.recursive);
            return (
              <Link
                key={item.id}
                href={item.href}
                aria-current={active ? "page" : undefined}
                className={cn(
                  "flex min-h-14 flex-col items-center justify-center gap-0.5 border-r-2 border-strong px-1 py-1.5 text-[11px] font-bold last:border-r-0",
                  "focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-focus",
                  active
                    ? "bg-primary text-primary-foreground"
                    : "bg-surface text-ink hover:bg-surface-muted",
                )}
              >
                <Icon className="size-4" aria-hidden="true" />
                <span>{item.label}</span>
              </Link>
            );
          })}
        </nav>
      ) : null}
    </div>
  );
}
