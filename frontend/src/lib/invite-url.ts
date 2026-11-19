export function normalizeBasePath(raw: string | undefined): string {
  const trimmed = (raw ?? "").trim().replace(/\/+$/, "");
  if (!trimmed || trimmed === "/") return "";
  return trimmed.startsWith("/") ? trimmed : `/${trimmed}`;
}

export const BASE_PATH = normalizeBasePath(
  process.env.NEXT_PUBLIC_FRIENDARAZZI_BASE_PATH,
);

export function withBasePath(path: string, basePath = BASE_PATH): string {
  const normalizedBase = normalizeBasePath(basePath);
  const normalizedPath = path.startsWith("/") ? path : `/${path}`;
  if (
    !normalizedBase ||
    normalizedPath === normalizedBase ||
    normalizedPath.startsWith(`${normalizedBase}/`)
  ) {
    return normalizedPath;
  }
  return `${normalizedBase}${normalizedPath}`;
}

export function stripBasePath(pathname: string, basePath = BASE_PATH): string {
  const normalizedBase = normalizeBasePath(basePath);
  if (normalizedBase && pathname.startsWith(normalizedBase)) {
    const stripped = pathname.slice(normalizedBase.length);
    return stripped.startsWith("/") ? stripped : `/${stripped}`;
  }
  return pathname;
}

/**
 * Build a shareable absolute invite URL for a lobby code.
 * Uses a trailing slash on `/join/` because `next.config.ts` exports static
 * pages with `trailingSlash: true`, avoiding static-host 301 redirects.
 */
export function inviteUrl(
  inviteCode: string,
  options?: { origin?: string; basePath?: string },
): string {
  const normalizedCode = inviteCode.trim().toUpperCase();
  const basePath =
    options?.basePath !== undefined
      ? normalizeBasePath(options.basePath)
      : BASE_PATH;
  const routePath = withBasePath(
    `/join/?code=${encodeURIComponent(normalizedCode)}`,
    basePath,
  );
  const origin =
    options?.origin !== undefined
      ? options.origin.replace(/\/+$/, "")
      : typeof window !== "undefined"
        ? window.location.origin
        : "";
  return origin ? `${origin}${routePath}` : routePath;
}

/** Accept only relative in-app paths so auth redirects cannot leave the origin. */
export function safeRedirectPath(
  raw: string | null | undefined,
  fallback = "/",
): string {
  if (!raw) return fallback;
  const trimmed = raw.trim();
  if (
    !trimmed.startsWith("/") ||
    trimmed.startsWith("//") ||
    trimmed.startsWith("/\\") ||
    /[\r\n\0]/.test(trimmed)
  ) {
    return fallback;
  }
  return trimmed;
}

export function authHrefWithRedirect(
  route: "/anmeldung" | "/registrierung",
  redirectPath: string | null | undefined,
): string {
  const target = safeRedirectPath(redirectPath, "");
  if (!target || target === "/") return route;
  return `${route}?redirect=${encodeURIComponent(target)}`;
}
