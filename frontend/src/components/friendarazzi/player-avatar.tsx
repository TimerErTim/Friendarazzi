"use client";

import { Award, Crown } from "lucide-react";
import Link from "next/link";
import type { PlayerBadge } from "@/api/friendarazzi-api";
import { useBenutzerQuery } from "@/api/hooks";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/cn";
import { paths, userProfileHref } from "@/lib/navigation";

import { AuthenticatedImage } from "./media-image";

type AvatarSize = "sm" | "md" | "lg";

const SIZE_CLASSES: Record<AvatarSize, string> = {
  sm: "size-8 text-xs",
  md: "size-11 text-sm",
  lg: "size-16 text-lg",
};

function initialsFor(name: string): string {
  const parts = name.trim().split(/\s+/);
  if (parts.length >= 2) {
    return `${parts[0]?.[0] ?? ""}${parts[1]?.[0] ?? ""}`.toUpperCase();
  }
  return name.slice(0, 2).toUpperCase() || "?";
}

export function PlayerAvatar({
  user,
  size = "md",
  linkToProfile = true,
  showMedalBadge = true,
  className,
}: {
  user: Pick<
    PlayerBadge,
    "id" | "displayName" | "avatarStorageKey" | "goldMedalCount"
  >;
  size?: AvatarSize;
  linkToProfile?: boolean;
  showMedalBadge?: boolean;
  className?: string;
}) {
  const me = useBenutzerQuery();
  const href =
    me.data?.id === user.id ? paths.profil : userProfileHref(user.id);
  const hasGold = user.goldMedalCount > 0;

  const avatarBox = (
    <span className={cn("relative inline-flex shrink-0", className)}>
      <span
        className={cn(
          "inline-flex items-center justify-center overflow-hidden rounded-full border-2 bg-surface-muted font-semibold text-ink",
          SIZE_CLASSES[size],
          hasGold ? "border-warning ring-2 ring-warning/30" : "border-subtle",
        )}
      >
        {user.avatarStorageKey ? (
          <AuthenticatedImage
            storageKey={user.avatarStorageKey}
            alt={user.displayName}
            className="h-full w-full"
          />
        ) : (
          <span>{initialsFor(user.displayName)}</span>
        )}
      </span>
      {showMedalBadge && hasGold ? (
        <span
          className="absolute -right-1 -bottom-1 inline-flex items-center gap-0.5 rounded-full border border-warning bg-elevated px-1 py-0.5 text-[10px] leading-none font-bold text-ink tabular-nums"
          title={`${user.goldMedalCount} Goldmedaille(n)`}
        >
          <Award className="size-2.5 text-warning" aria-hidden="true" />
          {user.goldMedalCount}
        </span>
      ) : null}
    </span>
  );

  if (!linkToProfile) {
    return avatarBox;
  }

  return (
    <Link
      href={href}
      className="inline-flex min-h-11 min-w-11 items-center justify-center rounded-full focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
      title={`Profil von ${user.displayName} öffnen`}
    >
      {avatarBox}
    </Link>
  );
}

/**
 * Clickable player row/chip with avatar, display name, Gold Medal badge, and title.
 */
export function PlayerBadgeLink({
  user,
  subtitle,
  compact = false,
}: {
  user: PlayerBadge;
  subtitle?: string;
  compact?: boolean;
}) {
  const me = useBenutzerQuery();
  const href =
    me.data?.id === user.id ? paths.profil : userProfileHref(user.id);

  return (
    <Link
      href={href}
      className={cn(
        "group inline-flex min-h-11 items-center gap-2.5 rounded-sm px-1.5 py-1 transition-colors hover:bg-surface-muted",
        "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary",
      )}
    >
      <PlayerAvatar
        user={user}
        size={compact ? "sm" : "md"}
        linkToProfile={false}
      />
      <span className="min-w-0 flex-1 text-left">
        <span className="flex flex-wrap items-center gap-1.5">
          <span className="truncate text-sm font-semibold text-ink group-hover:underline">
            {user.displayName}
          </span>
          {user.isHost ? (
            <Badge variant="primary" className="gap-1">
              <Crown className="size-3" aria-hidden="true" /> Host
            </Badge>
          ) : null}
          {user.goldMedalCount > 0 ? (
            <Badge variant="warning" className="gap-1">
              <Award className="size-3 text-warning" aria-hidden="true" />
              {user.goldMedalCount}
            </Badge>
          ) : null}
        </span>
        <span className="block truncate text-xs text-secondary">
          {subtitle ?? user.effectiveTitle}
        </span>
      </span>
    </Link>
  );
}
