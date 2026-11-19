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
  fit?: "cover" | "contain-blurred";
};

/**
 * Renders an authenticated backend media object via `fetch` + Bearer token.
 * In `contain-blurred` mode the full photo fits inside the frame while a
 * blurred `cover` layer fills letterboxed edges.
 */
export function AuthenticatedImage({
  storageKey,
  alt,
  className,
  blurred = false,
  fit = "cover",
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

  if (fit === "contain-blurred") {
    return (
      <div
        className={cn("relative overflow-hidden bg-surface-muted", className)}
      >
        {/* biome-ignore lint/performance/noImgElement: authenticated blob URL cannot use next/image */}
        <img
          src={url}
          alt=""
          aria-hidden="true"
          className="pointer-events-none absolute inset-0 h-full w-full scale-115 object-cover opacity-80 blur-md select-none"
        />
        {/* biome-ignore lint/performance/noImgElement: authenticated blob URL cannot use next/image */}
        <img
          src={url}
          alt={alt}
          className={cn(
            "relative z-10 h-full w-full object-contain",
            blurred ? "blur-sm" : "",
          )}
        />
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
