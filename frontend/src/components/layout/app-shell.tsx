"use client";

import {
  Award,
  CircleAlert,
  LogIn,
  LogOut,
  Menu,
  Settings2,
  UserRound,
} from "lucide-react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useAuth } from "@/api/auth-provider";
import { useAuthMutations, useBenutzerQuery } from "@/api/hooks";
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
        className="sr-only rounded-sm px-4 py-2 text-sm font-medium text-ink focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:bg-surface focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
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
        className="pointer-events-none fixed inset-0 z-40 bg-black/50 opacity-0 transition-opacity peer-checked:pointer-events-auto peer-checked:opacity-100 lg:hidden"
      />

      <div className="relative flex min-w-0 flex-1 flex-col overflow-hidden">
        <header className="sticky top-0 z-30 border-b border-subtle bg-surface">
          <div className="flex h-14 items-center gap-3 px-4">
            <label
              htmlFor="main-drawer"
              className="inline-flex size-11 cursor-pointer items-center justify-center rounded-md text-ink hover:bg-surface-muted lg:hidden"
            >
              <Menu className="size-5" />
              <span className="sr-only">Navigation öffnen</span>
            </label>

            <nav
              aria-label="Breadcrumb"
              className="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto text-sm"
            >
              {breadcrumbs.map((crumb, index) => (
                <span
                  key={`${crumb.label}-${index}`}
                  className="flex items-center"
                >
                  {index > 0 ? (
                    <span className="mx-1 text-secondary">/</span>
                  ) : null}
                  {crumb.href ? (
                    <Link
                      href={crumb.href}
                      className="rounded-sm px-2 py-1 font-medium text-ink hover:bg-surface-muted"
                    >
                      {crumb.label}
                    </Link>
                  ) : (
                    <span className="px-2 py-1 font-semibold text-ink">
                      {crumb.label}
                    </span>
                  )}
                </span>
              ))}
            </nav>

            <div className="flex shrink-0 items-center gap-2">
              {phase.status === "authenticated" && benutzer.data ? (
                <>
                  <Link
                    href="/profil"
                    className="hidden items-center gap-1.5 rounded-sm border border-warning/50 bg-warning/15 px-2.5 py-1 text-xs font-semibold text-ink hover:bg-warning/25 sm:inline-flex"
                    title="Goldmedaillen"
                  >
                    <Award className="size-3.5 text-warning" />
                    <span className="tabular-nums">
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
                        <span className="block truncate text-sm font-medium text-ink">
                          {benutzer.data.displayName}
                        </span>
                        <span className="block truncate text-xs text-secondary">
                          @{benutzer.data.username} ·{" "}
                          {benutzer.data.effectiveTitle}
                        </span>
                      </DropdownMenuLabel>
                      <DropdownMenuSeparator />
                      <DropdownMenuItem asChild>
                        <Link href="/profil">
                          <UserRound aria-hidden="true" /> Mein Profil
                        </Link>
                      </DropdownMenuItem>
                      <DropdownMenuItem asChild>
                        <Link href="/optionen">
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
                <Link href="/anmeldung">
                  <Button
                    size="sm"
                    variant={onAuthRoute ? "secondary" : "primary"}
                  >
                    <LogIn />
                    Anmelden
                  </Button>
                </Link>
              ) : null}
            </div>
          </div>
        </header>

        <main id="main" tabIndex={-1} className="flex-1 overflow-y-auto">
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
