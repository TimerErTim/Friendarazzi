"use client";

import { Pin, Upload } from "lucide-react";
import { useEffect, useState } from "react";

import { apiErrorMessage } from "@/api/errors";
import type { BoardTileView } from "@/api/friendarazzi-api";
import { useFriendarazziMutations } from "@/api/hooks";
import { CaptureUploadControl } from "@/components/friendarazzi/capture-upload-control";
import { PlayerAvatar } from "@/components/friendarazzi/player-avatar";
import { Alert } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { cn } from "@/lib/cn";
import { normalizeImageForUpload } from "@/lib/media-capture";

export function CaptureTileDialog({
  gameId,
  tile,
  isPinned,
  onTogglePin,
  onClose,
  onCaptured,
}: {
  gameId: string;
  tile: BoardTileView | null;
  isPinned: boolean;
  onTogglePin: (tileId: string) => void;
  onClose: () => void;
  onCaptured?: (tileId: string) => void;
}) {
  const mutations = useFriendarazziMutations();
  const { reset: resetCaptureMutation } = mutations.submitCapture;
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [isPreparingUpload, setIsPreparingUpload] = useState(false);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);

  const tileId = tile?.id ?? null;

  useEffect(() => {
    setSelectedFile(null);
    if (tileId) {
      resetCaptureMutation();
    }
  }, [tileId, resetCaptureMutation]);

  useEffect(() => {
    if (!selectedFile) {
      setPreviewUrl(null);
      return;
    }
    const url = URL.createObjectURL(selectedFile);
    setPreviewUrl(url);
    return () => URL.revokeObjectURL(url);
  }, [selectedFile]);

  const isUploading = isPreparingUpload || mutations.submitCapture.isPending;

  async function submitTilePhoto() {
    if (!tile || !selectedFile || isUploading) return;
    const submittedTileId = tile.id;
    setIsPreparingUpload(true);
    try {
      const normalized = await normalizeImageForUpload(selectedFile);
      await mutations.submitCapture.mutateAsync({
        gameId,
        tileId: submittedTileId,
        file: normalized,
      });
      setSelectedFile(null);
      onCaptured?.(submittedTileId);
      onClose();
    } finally {
      setIsPreparingUpload(false);
    }
  }

  return (
    <Dialog
      open={Boolean(tile)}
      onOpenChange={(open) => {
        if (!open) {
          setSelectedFile(null);
          onClose();
        }
      }}
    >
      <DialogContent>
        <DialogHeader className="space-y-2.5">
          {tile?.targetUser ? (
            <div className="flex items-center justify-between gap-3 rounded-md border-2 border-strong bg-cyan-subtle p-2.5">
              <div className="flex min-w-0 items-center gap-3">
                <PlayerAvatar
                  user={tile.targetUser}
                  size="sm"
                  linkToProfile={false}
                />
                <div className="min-w-0">
                  <p className="truncate font-display text-sm leading-tight text-ink">
                    {tile.targetUser.displayName}
                  </p>
                  <p className="truncate font-mono text-[11px] font-bold text-secondary">
                    {tile.targetUser.effectiveTitle}
                  </p>
                </div>
              </div>
              <div className="flex shrink-0 items-center gap-1.5">
                <span className="rounded-sm border border-strong bg-surface px-1.5 py-0.5 font-mono text-[10px] font-bold tracking-wider text-ink uppercase">
                  Zielperson
                </span>
                <Button
                  type="button"
                  variant="secondary"
                  size="icon"
                  className={cn(
                    isPinned &&
                      "border-info bg-info/15 text-ink hover:bg-info/25",
                  )}
                  aria-pressed={isPinned}
                  aria-label={
                    isPinned
                      ? `Auftrag „${tile.promptText}“ aus Fokus entfernen`
                      : `Auftrag „${tile.promptText}“ anheften`
                  }
                  title={isPinned ? "Aus Fokus entfernen" : "Im Fokus anheften"}
                  onClick={() => onTogglePin(tile.id)}
                >
                  <Pin
                    className={cn(
                      "size-4 rotate-45",
                      isPinned && "fill-info text-info",
                    )}
                    aria-hidden="true"
                  />
                </Button>
              </div>
            </div>
          ) : null}
          <DialogTitle className="font-display text-2xl leading-tight tracking-tight text-ink">
            {tile?.promptText}
          </DialogTitle>
          <DialogDescription className="sr-only">
            Lade ein Beweisfoto für diesen Auftrag hoch.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-3">
          {previewUrl ? (
            <div className="space-y-2">
              <div className="brutal-shadow-sm overflow-hidden rounded-md border-2 border-strong bg-surface p-1.5">
                {/* biome-ignore lint/performance/noImgElement: local File preview blob URL */}
                <img
                  src={previewUrl}
                  alt="Vorschau"
                  className="max-h-72 w-full rounded-sm object-contain"
                />
              </div>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                disabled={isUploading}
                onClick={() => setSelectedFile(null)}
              >
                Anderes Foto wählen
              </Button>
            </div>
          ) : (
            <CaptureUploadControl
              onFileSelect={(file) => setSelectedFile(file)}
            />
          )}

          {mutations.submitCapture.isError ? (
            <Alert variant="error">
              {apiErrorMessage(mutations.submitCapture.error, {
                fallback: "Das Foto konnte nicht hochgeladen werden.",
              })}
            </Alert>
          ) : null}
        </div>

        <DialogFooter>
          <Button
            type="button"
            variant="secondary"
            onClick={() => {
              setSelectedFile(null);
              onClose();
            }}
            disabled={isUploading}
          >
            Abbrechen
          </Button>
          <Button
            type="button"
            disabled={!selectedFile || isUploading}
            onClick={() => void submitTilePhoto()}
          >
            <Upload className="size-4" aria-hidden="true" />
            {isUploading ? "Wird hochgeladen …" : "Beweis einreichen"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
