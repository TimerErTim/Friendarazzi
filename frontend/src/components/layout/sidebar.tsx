"use client";

import Link from "next/link";
import { Wordmark } from "@/components/brand/wordmark";
import { SparkleIcon } from "@/components/ui/brutal-elements";
import { cn } from "@/lib/cn";
import type { NavCategory, NavItem } from "@/lib/navigation";
import { isNavItemActive } from "@/lib/navigation";

type SidebarProps = {
  categories: NavCategory[];
  homeItem: NavItem;
  pathname: string;
};

export function Sidebar({ categories, homeItem, pathname }: SidebarProps) {
  const HomeIcon = homeItem.icon;
  const homeActive = isNavItemActive(
    pathname,
    homeItem.href,
    homeItem.recursive,
  );

  return (
    <aside
      className={cn(
        "fixed inset-y-0 left-0 z-50 flex w-68 flex-col border-r-[2.5px] border-strong bg-surface transition-transform",
        "-translate-x-full lg:static lg:translate-x-0",
        "peer-checked:translate-x-0",
      )}
    >
      <div className="flex h-full flex-col overflow-y-auto px-4 py-5">
        <div className="mb-6 border-b-2 border-strong pb-4">
          <Link
            href={homeItem.href}
            className="inline-flex min-h-11 items-center rounded-sm px-1 py-1 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
            aria-label="Friendarazzi — Startseite"
          >
            <Wordmark size="sm" showMark />
          </Link>
        </div>

        <div className="mb-6">
          <Link
            href={homeItem.href}
            aria-current={homeActive ? "page" : undefined}
            className={cn(
              "flex min-h-11 w-full items-center gap-3 rounded-sm border-2 px-3 py-2 text-sm font-bold transition-colors",
              "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus",
              homeActive
                ? "brutal-shadow-sm border-strong bg-primary text-primary-foreground"
                : "border-transparent text-ink hover:border-strong hover:bg-surface-muted",
            )}
          >
            <HomeIcon className="size-5 shrink-0" aria-hidden="true" />
            <span>{homeItem.label}</span>
          </Link>
        </div>

        {categories.length > 0 ? (
          <nav aria-label="Hauptnavigation" className="flex-1 space-y-6">
            {categories.map((category) => (
              <div key={category.label} className="space-y-2">
                <h3 className="px-2 font-mono text-[11px] font-bold tracking-[0.08em] text-secondary uppercase">
                  {category.label}
                </h3>
                <div className="space-y-1.5">
                  {category.items.map((item) => (
                    <SidebarLink
                      key={item.id}
                      item={item}
                      pathname={pathname}
                    />
                  ))}
                </div>
              </div>
            ))}
          </nav>
        ) : null}

        <div className="mt-6 rounded-md border-2 border-strong bg-cyan-subtle p-3">
          <div className="flex items-center gap-2">
            <SparkleIcon tone="accent" className="size-5" />
            <span className="font-display text-xs tracking-tight text-ink uppercase">
              Foto-Jagd-Bingo
            </span>
          </div>
          <p className="mt-1 text-xs font-medium text-secondary">
            Schnappschüsse sammeln, Reihen füllen, Freunde erwischen.
          </p>
        </div>
      </div>
    </aside>
  );
}

function SidebarLink({ item, pathname }: { item: NavItem; pathname: string }) {
  const Icon = item.icon;
  const active = isNavItemActive(pathname, item.href, item.recursive);

  return (
    <Link
      href={item.href}
      aria-current={active ? "page" : undefined}
      className={cn(
        "flex min-h-11 w-full items-center gap-3 rounded-sm border-2 px-3 py-2 text-sm font-bold transition-colors",
        "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus",
        active
          ? "brutal-shadow-sm border-strong bg-primary text-primary-foreground"
          : "border-transparent text-ink hover:border-strong hover:bg-surface-muted",
      )}
    >
      <Icon className="size-5 shrink-0" aria-hidden="true" />
      <span>{item.label}</span>
    </Link>
  );
}
