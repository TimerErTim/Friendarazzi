"use client";

import { Check, Copy, QrCode, Share2, Ticket, X } from "lucide-react";
import QRCode from "qrcode";
import { useEffect, useMemo, useRef, useState } from "react";

import { CodeTag } from "@/components/ui/brutal-elements";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { inviteUrl } from "@/lib/navigation";

function buildQrSvgPath(value: string): { size: number; path: string } {
  const qr = QRCode.create(value, { errorCorrectionLevel: "M" });
  const size = qr.modules.size;
  const parts: string[] = [];
  for (let row = 0; row < size; row += 1) {
    for (let col = 0; col < size; col += 1) {
      if (qr.modules.get(row, col)) {
        parts.push(`M${col},${row}h1v1h-1z`);
      }
    }
  }
  return { size, path: parts.join("") };
}

/**
 * Single consolidated invite card: manual room code + one primary share action
 * (Web Share -> Clipboard -> selectable fallback input) + optional QR code panel.
 */
export function InviteShareCard({ inviteCode }: { inviteCode: string }) {
  const shareUrl = useMemo(() => inviteUrl(inviteCode), [inviteCode]);
  const qrData = useMemo(() => buildQrSvgPath(shareUrl), [shareUrl]);

  const [shareStatus, setShareStatus] = useState<
    "idle" | "copied" | "shared" | "manual"
  >("idle");
  const [showQr, setShowQr] = useState(false);

  const manualInputRef = useRef<HTMLInputElement>(null);
  const qrToggleRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (shareStatus !== "copied" && shareStatus !== "shared") return;
    const timer = setTimeout(() => setShareStatus("idle"), 2200);
    return () => clearTimeout(timer);
  }, [shareStatus]);

  useEffect(() => {
    if (shareStatus === "manual") {
      manualInputRef.current?.focus();
      manualInputRef.current?.select();
    }
  }, [shareStatus]);

  async function handleShare() {
    if (
      typeof navigator !== "undefined" &&
      typeof navigator.share === "function"
    ) {
      try {
        await navigator.share({
          title: "Friendarazzi",
          text: `Spiel mit mir Foto-Bingo! Code: ${inviteCode}`,
          url: shareUrl,
        });
        setShareStatus("shared");
        return;
      } catch (err) {
        if (err instanceof DOMException && err.name === "AbortError") {
          return;
        }
        // Other Web Share errors fall through to clipboard / manual fallback.
      }
    }

    if (
      typeof navigator !== "undefined" &&
      navigator.clipboard &&
      typeof navigator.clipboard.writeText === "function"
    ) {
      try {
        await navigator.clipboard.writeText(shareUrl);
        setShareStatus("copied");
        return;
      } catch {
        // Fall through to selectable input fallback.
      }
    }

    setShareStatus("manual");
  }

  function closeQrAndRestoreFocus() {
    setShowQr(false);
    requestAnimationFrame(() => qrToggleRef.current?.focus());
  }

  const canUseNativeShare =
    typeof navigator !== "undefined" && typeof navigator.share === "function";

  return (
    <div className="brutal-shadow-lg overflow-hidden rounded-md border-[2.5px] border-strong bg-surface">
      <div className="flex flex-wrap items-center justify-between gap-4 border-b-2 border-strong bg-primary p-5 text-primary-foreground">
        <div className="space-y-1">
          <p className="inline-flex items-center gap-1.5 font-mono text-xs font-bold tracking-wider uppercase">
            <Ticket className="size-3.5" aria-hidden="true" />
            Freunde einladen
          </p>
          <p className="font-mono text-3xl font-bold tracking-[0.2em]">
            {inviteCode}
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Button
            type="button"
            variant="secondary"
            onClick={() => void handleShare()}
          >
            {shareStatus === "copied" ? (
              <>
                <Check className="size-4 text-success" aria-hidden="true" />
                Link kopiert!
              </>
            ) : shareStatus === "shared" ? (
              <>
                <Check className="size-4 text-success" aria-hidden="true" />
                Geteilt!
              </>
            ) : canUseNativeShare ? (
              <>
                <Share2 className="size-4" aria-hidden="true" />
                Einladung teilen
              </>
            ) : (
              <>
                <Copy className="size-4" aria-hidden="true" />
                Einladung teilen
              </>
            )}
          </Button>

          <Button
            ref={qrToggleRef}
            type="button"
            variant="secondary"
            aria-expanded={showQr}
            onClick={() => setShowQr((prev) => !prev)}
          >
            {showQr ? (
              <>
                <X className="size-4" aria-hidden="true" />
                QR schließen
              </>
            ) : (
              <>
                <QrCode className="size-4" aria-hidden="true" />
                QR-Code anzeigen
              </>
            )}
          </Button>
        </div>
      </div>

      <output aria-live="polite" className="sr-only">
        {shareStatus === "copied"
          ? "Einladungslink wurde in die Zwischenablage kopiert."
          : shareStatus === "shared"
            ? "Einladung wurde geteilt."
            : shareStatus === "manual"
              ? "Automatisches Kopieren nicht verfügbar. Einladungslink ist markiert."
              : ""}
      </output>

      {shareStatus === "manual" ? (
        <div className="space-y-2 border-b-2 border-strong bg-surface-muted p-4">
          <label
            htmlFor="invite-manual-url"
            className="block font-mono text-xs font-bold text-ink"
          >
            Link zum Kopieren (Strg+C / Teilen)
          </label>
          <Input
            ref={manualInputRef}
            id="invite-manual-url"
            readOnly
            value={shareUrl}
            onFocus={(e) => e.currentTarget.select()}
            className="font-mono text-xs"
          />
        </div>
      ) : null}

      {showQr ? (
        <section
          aria-label="Einladungs-QR-Code"
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              e.stopPropagation();
              closeQrAndRestoreFocus();
            }
          }}
          className="flex flex-col items-center gap-3 bg-surface-muted p-5 text-center"
        >
          {/* Opaque light paper & ink modules ensure camera scan reliability in dark mode too. */}
          <div className="brutal-shadow-sm rounded-md border-2 border-strong bg-surface p-2">
            <svg
              role="img"
              aria-label={`QR-Code zum Beitreten in Lobby ${inviteCode}`}
              viewBox={`-4 -4 ${qrData.size + 8} ${qrData.size + 8}`}
              className="size-44 sm:size-48"
              shapeRendering="crispEdges"
            >
              <rect
                x={-4}
                y={-4}
                width={qrData.size + 8}
                height={qrData.size + 8}
                fill="#ffffff"
              />
              <path d={qrData.path} fill="#0d0d0d" />
            </svg>
          </div>
          <div className="space-y-1">
            <p className="text-xs font-bold text-ink">
              Mit Smartphone-Kamera scannen oder Code eingeben:
            </p>
            <CodeTag tone="yellow">{inviteCode}</CodeTag>
          </div>
        </section>
      ) : null}
    </div>
  );
}
