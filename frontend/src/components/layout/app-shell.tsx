"use client";

import {
  Award,
  CircleAlert,
  LogIn,
  LogOut,
  Menu,
  Settings2,
  Ticket,
  UserRound,
} from "lucide-react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useAuth } from "@/api/auth-provider";
import { useAuthMutations, useBenutzerQuery } from "@/api/hooks";
import { Wordmark } from "@/components/brand/wordmark";
import { usePageChrome } from "@/components/layout/page-chrome";
import { Sidebar } from "@/components/layout/sidebar";
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
import {
  breadcrumbsForPathname,
  homeNavItem,
  paths,
  sidebarCategories,
} from "@/lib/navigation";

export function AppShell({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const { breadcrumbLabel } = usePageChrome();
  const breadcrumbs = breadcrumbsForPathname(pathname, breadcrumbLabel);
  const phase = useAuth();
  const benutzer = useBenutzerQuery();
  const auth = useAuthMutations();
  const onAuthRoute = ["/anmeldung", "/registrierung", "/passwort"].includes(
    pathname.replace(/\/$/, ""),
  );

  return (
    <div className="flex h-screen overflow-hidden bg-canvas">
      <a
        href="#main"
        className="sr-only rounded-sm border-2 border-strong px-4 py-2 text-sm font-bold text-ink focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:bg-primary focus:text-primary-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
      >
        Zum Inhalt springen
      </a>

      <input id="main-drawer" type="checkbox" className="peer hidden" />

      <Sidebar
        categories={sidebarCategories}
        homeItem={homeNavItem}
        pathname={pathname}
      />

      <label
        htmlFor="main-drawer"
        aria-label="Navigation schließen"
        className="pointer-events-none fixed inset-0 z-40 bg-ink/50 opacity-0 transition-opacity peer-checked:pointer-events-auto peer-checked:opacity-100 lg:hidden"
      />

      <div className="relative flex min-w-0 flex-1 flex-col overflow-hidden">
        <header className="sticky top-0 z-30 border-b-[2.5px] border-strong bg-surface">
          <div className="flex h-16 items-center gap-3 px-4">
            <label
              htmlFor="main-drawer"
              className="brutal-press inline-flex size-11 cursor-pointer items-center justify-center rounded-sm border-2 border-control bg-primary text-primary-foreground lg:hidden"
            >
              <Menu className="size-5" />
              <span className="sr-only">Navigation öffnen</span>
            </label>

            <div className="lg:hidden">
              <Link
                href={paths.home}
                className="inline-flex min-h-11 items-center rounded-sm px-1 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
                aria-label="Friendarazzi"
              >
                <Wordmark size="sm" />
              </Link>
            </div>

            <nav
              aria-label="Breadcrumb"
              className="hidden min-w-0 flex-1 items-center gap-1 overflow-x-auto text-sm lg:flex"
            >
              {breadcrumbs.map((crumb, index) => (
                <span
                  key={`${crumb.label}-${index}`}
                  className="flex items-center"
                >
                  {index > 0 ? (
                    <span className="mx-1.5 font-mono text-secondary">/</span>
                  ) : null}
                  {crumb.href ? (
                    <Link
                      href={crumb.href}
                      className="rounded-sm px-2 py-1 font-bold text-ink transition-colors hover:bg-primary-subtle focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
                    >
                      {crumb.label}
                    </Link>
                  ) : (
                    <span className="rounded-sm border-2 border-strong bg-surface-muted px-2 py-0.5 font-mono text-xs font-bold text-ink uppercase">
                      {crumb.label}
                    </span>
                  )}
                </span>
              ))}
            </nav>

            <div className="ml-auto flex shrink-0 items-center gap-2">
              {phase.status === "authenticated" && benutzer.data ? (
                <>
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
                        size="icon"
                        variant="secondary"
                        aria-label="Profilmenü öffnen"
                        disabled={
                          auth.logout.isPending || auth.deleteAccount.isPending
                        }
                      >
                        <UserRound />
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
                    variant="secondary"
                    asChild
                    className="hidden sm:inline-flex"
                  >
                    <Link href={paths.join}>
                      <Ticket aria-hidden="true" />
                      Beitreten
                    </Link>
                  </Button>
                  <Button size="sm" asChild>
                    <Link href="/anmeldung">
                      <LogIn aria-hidden="true" />
                      Anmelden
                    </Link>
                  </Button>
                </div>
              ) : null}
            </div>
          </div>
        </header>

        <main
          id="main"
          tabIndex={-1}
          className="brutal-grid flex-1 overflow-y-auto"
        >
          {phase.status === "unavailable" ? (
            <div className="mx-auto w-full max-w-5xl px-6 pt-6">
              <Alert
                variant="warning"
                icon={<CircleAlert aria-hidden="true" />}
              >
                <p>
                  Ihre Sitzung konnte nicht überprüft werden. Bitte laden Sie
                  die Seite erneut oder melden Sie sich an.
                </p>
              </Alert>
            </div>
          ) : null}
          {children}
        </main>
      </div>
    </div>
  );
}
