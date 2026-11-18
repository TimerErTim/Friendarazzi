import type { LucideIcon } from "lucide-react";
import { Camera, Home, Settings2, Ticket, UserRound } from "lucide-react";

export type NavItem = {
  id: string;
  label: string;
  href: string;
  icon: LucideIcon;
  recursive?: boolean;
};

export type NavCategory = {
  label: string;
  items: NavItem[];
};

/** Strip trailing slash so `/foo/` matches `/foo` (Next `trailingSlash: true`). */
export function normalizePath(path: string): string {
  if (path.length > 1 && path.endsWith("/")) {
    return path.slice(0, -1);
  }
  return path;
}

export function isExactPath(pathname: string, href: string): boolean {
  return normalizePath(pathname) === normalizePath(href);
}

export const BASE_PATH = (
  process.env.NEXT_PUBLIC_FRIENDARAZZI_BASE_PATH ?? ""
).replace(/\/+$/, "");

export function withBasePath(path: string): string {
  if (!BASE_PATH || path.startsWith(BASE_PATH)) {
    return path;
  }
  return `${BASE_PATH}${path.startsWith("/") ? path : `/${path}`}`;
}

export function stripBasePath(pathname: string): string {
  if (BASE_PATH && pathname.startsWith(BASE_PATH)) {
    const stripped = pathname.slice(BASE_PATH.length);
    return stripped.startsWith("/") ? stripped : `/${stripped}`;
  }
  return pathname;
}

export function absoluteUrl(appPath: string): string {
  const fullPath = withBasePath(appPath);
  if (typeof window !== "undefined") {
    return `${window.location.origin}${fullPath}`;
  }
  return fullPath;
}

export const paths = {
  home: "/",
  games: "/games",
  join: "/join",
  profil: "/profil",
  users: "/users",
  optionen: "/optionen",
} as const;

export function gameHref(gameId: string): string {
  return `${paths.games}?id=${encodeURIComponent(gameId)}`;
}

export function joinHref(inviteCode: string): string {
  return `${paths.join}?code=${encodeURIComponent(inviteCode)}`;
}

export function userProfileHref(userId: string): string {
  return `${paths.users}?id=${encodeURIComponent(userId)}`;
}

export const sidebarCategories: NavCategory[] = [
  {
    label: "Friendarazzi",
    items: [
      {
        id: "games",
        label: "Aktuelles Spiel",
        href: paths.games,
        icon: Camera,
        recursive: true,
      },
      {
        id: "join",
        label: "Mit Code beitreten",
        href: paths.join,
        icon: Ticket,
        recursive: true,
      },
    ],
  },
  {
    label: "Konto",
    items: [
      {
        id: "profil",
        label: "Mein Profil",
        href: paths.profil,
        icon: UserRound,
        recursive: true,
      },
      {
        id: "optionen",
        label: "Optionen",
        href: paths.optionen,
        icon: Settings2,
        recursive: true,
      },
    ],
  },
];

export const homeNavItem: NavItem = {
  id: "home",
  label: "Spiele",
  href: paths.home,
  icon: Home,
  recursive: false,
};

type BreadcrumbSegment = {
  label: string;
  href?: string;
};

const routeLabels: Record<string, string> = {
  games: "Spiel",
  join: "Beitreten",
  users: "Profil",
  anmeldung: "Anmelden",
  registrierung: "Registrieren",
  passwort: "Passwort wiederherstellen",
  profil: "Mein Profil",
  optionen: "Optionen",
};

export function breadcrumbsForPathname(
  pathname: string,
  dynamicLabel?: string,
): BreadcrumbSegment[] {
  const segments = pathname.split("/").filter(Boolean);
  if (segments.length === 0) {
    return [{ label: "Spiele", href: paths.home }];
  }

  const crumbs: BreadcrumbSegment[] = [];
  let href = "";

  for (let index = 0; index < segments.length; index += 1) {
    const segment = segments[index] ?? "";
    href += `/${segment}`;
    const isLast = index === segments.length - 1;

    let label = routeLabels[segment] ?? segment;
    if (isLast && dynamicLabel) {
      label = dynamicLabel;
    }

    crumbs.push({
      label,
      href: isLast ? undefined : href,
    });
  }

  return crumbs;
}

export function isNavItemActive(
  pathname: string,
  href: string,
  recursive = true,
): boolean {
  const normalizedPath = normalizePath(pathname);
  const normalizedHref = normalizePath(href);

  if (recursive) {
    return (
      normalizedPath === normalizedHref ||
      normalizedPath.startsWith(`${normalizedHref}/`)
    );
  }
  return normalizedPath === normalizedHref;
}
