"use client";

import { CameraOff } from "lucide-react";

import { useMediaObjectUrl } from "@/api/hooks";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/cn";

type AuthenticatedImageProps = {
  storageKey: string | null | undefined;
  alt: string;
  className?: string;
  blurred?: boolean;
};

/**
 * Renders an authenticated backend media object via `fetch` + Bearer token.
 */
export function AuthenticatedImage({
  storageKey,
  alt,
  className,
  blurred = false,
}: AuthenticatedImageProps) {
  const { url, isLoading, isError } = useMediaObjectUrl(storageKey);

  if (!storageKey) {
    return (
      <div
        className={cn(
          "flex items-center justify-center bg-surface-muted text-secondary",
          className,
        )}
      >
        <CameraOff className="size-5" aria-hidden="true" />
      </div>
    );
  }

  if (isLoading && !url) {
    return <Skeleton className={cn("h-full w-full", className)} />;
  }

  if (isError || !url) {
    return (
      <div
        className={cn(
          "flex items-center justify-center bg-surface-muted text-secondary",
          className,
        )}
      >
        <CameraOff className="size-5" aria-hidden="true" />
      </div>
    );
  }

  return (
    // biome-ignore lint/performance/noImgElement: authenticated blob URL cannot use next/image
    <img
      src={url}
      alt={alt}
      className={cn(
        "object-cover",
        blurred ? "scale-110 blur-sm" : "",
        className,
      )}
    />
  );
}
