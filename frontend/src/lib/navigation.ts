import type { LucideIcon } from "lucide-react";
import { Camera, Home, Settings2, Trophy, UserRound } from "lucide-react";

import { BASE_PATH, withBasePath } from "./invite-url";

export {
  authHrefWithRedirect,
  BASE_PATH,
  inviteUrl,
  safeRedirectPath,
  stripBasePath,
  withBasePath,
} from "./invite-url";

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

export function absoluteUrl(appPath: string): string {
  const fullPath = withBasePath(appPath, BASE_PATH);
  if (typeof window !== "undefined") {
    return `${window.location.origin}${fullPath}`;
  }
  return fullPath;
}

export const paths = {
  home: "/",
  games: "/games",
  collections: "/collections",
  join: "/join",
  profil: "/profil",
  users: "/users",
  optionen: "/optionen",
  benachrichtigungen: "/benachrichtigungen",
  abbestellen: "/abbestellen",
} as const;

export function gameHref(gameId: string): string {
  return `${paths.games}?id=${encodeURIComponent(gameId)}`;
}

export function gameTileHref(gameId: string, tileId: string): string {
  return `${paths.games}?id=${encodeURIComponent(gameId)}&tile=${encodeURIComponent(tileId)}`;
}

export function captureHref(gameId: string, captureId: string): string {
  return `${paths.games}?id=${encodeURIComponent(gameId)}&capture=${encodeURIComponent(captureId)}`;
}

export function joinHref(inviteCode: string): string {
  return `${paths.join}?code=${encodeURIComponent(inviteCode)}`;
}

export function userProfileHref(userId: string): string {
  return `${paths.users}?id=${encodeURIComponent(userId)}`;
}

export const primaryNavItems: NavItem[] = [
  {
    id: "home",
    label: "Home",
    href: paths.home,
    icon: Home,
    recursive: false,
  },
  {
    id: "games",
    label: "Lobbys",
    href: paths.games,
    icon: Camera,
    recursive: true,
  },
  {
    id: "collections",
    label: "Collections",
    href: paths.collections,
    icon: Trophy,
    recursive: true,
  },
];

export const sidebarCategories: NavCategory[] = [
  {
    label: "Friendarazzi",
    items: [
      {
        id: "games",
        label: "Lobbys",
        href: paths.games,
        icon: Camera,
        recursive: true,
      },
      {
        id: "collections",
        label: "Collections",
        href: paths.collections,
        icon: Trophy,
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
  label: "Home",
  href: paths.home,
  icon: Home,
  recursive: false,
};

type BreadcrumbSegment = {
  label: string;
  href?: string;
};

const routeLabels: Record<string, string> = {
  games: "Lobbys",
  collections: "Collections",
  join: "Beitreten",
  users: "Profil",
  anmeldung: "Anmelden",
  registrierung: "Registrieren",
  passwort: "Passwort wiederherstellen",
  profil: "Mein Profil",
  optionen: "Optionen",
  benachrichtigungen: "Benachrichtigungen",
  abbestellen: "E-Mail abbestellen",
};

export function breadcrumbsForPathname(
  pathname: string,
  dynamicLabel?: string,
): BreadcrumbSegment[] {
  const segments = pathname.split("/").filter(Boolean);
  if (segments.length === 0) {
    return [{ label: "Home", href: paths.home }];
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
