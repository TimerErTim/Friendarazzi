"use client";

import { Camera, Image as ImageIcon } from "lucide-react";
import { type ChangeEvent, type DragEvent, useState } from "react";

import { cn } from "@/lib/cn";
import { useSupportsMobileCameraCapture } from "@/lib/media-capture";

/**
 * Tile proof photo selector: renders a dedicated live-camera capture control
 * (`capture="environment"`) plus a secondary gallery picker on mobile browsers,
 * and a drag-and-drop file picker on desktop.
 */
export function CaptureUploadControl({
  onFileSelect,
}: {
  onFileSelect: (file: File) => void;
}) {
  const isMobileCapture = useSupportsMobileCameraCapture();
  const [isDragging, setIsDragging] = useState(false);

  function handleFileChange(e: ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0] ?? null;
    e.currentTarget.value = "";
    if (file) {
      onFileSelect(file);
    }
  }

  function handleDragOver(e: DragEvent<HTMLLabelElement>) {
    e.preventDefault();
    setIsDragging(true);
  }

  function handleDragLeave(e: DragEvent<HTMLLabelElement>) {
    if (!e.currentTarget.contains(e.relatedTarget as Node | null)) {
      setIsDragging(false);
    }
  }

  function handleDrop(e: DragEvent<HTMLLabelElement>) {
    e.preventDefault();
    setIsDragging(false);
    const file = Array.from(e.dataTransfer.files).find((candidate) =>
      candidate.type.startsWith("image/"),
    );
    if (file) {
      onFileSelect(file);
    }
  }

  if (isMobileCapture) {
    return (
      <div className="space-y-2.5">
        <label className="brutal-press flex min-h-11 w-full cursor-pointer flex-col items-center justify-center gap-1.5 rounded-md border-2 border-control bg-primary-subtle p-4 text-center hover:bg-primary/25 focus-within:outline-2 focus-within:outline-offset-2 focus-within:outline-focus">
          <Camera className="size-8 text-ink" aria-hidden="true" />
          <span className="text-sm font-bold text-ink">Foto aufnehmen</span>
          <input
            type="file"
            accept="image/*"
            capture="environment"
            className="sr-only"
            onChange={handleFileChange}
          />
        </label>

        <label className="brutal-press flex min-h-11 w-full cursor-pointer items-center justify-center gap-2 rounded-sm border-2 border-control bg-surface px-4 py-2 text-sm font-bold text-ink hover:bg-surface-muted focus-within:outline-2 focus-within:outline-offset-2 focus-within:outline-focus">
          <ImageIcon className="size-4 shrink-0 text-ink" aria-hidden="true" />
          <span>Aus Galerie auswählen</span>
          <input
            type="file"
            accept="image/*"
            className="sr-only"
            onChange={handleFileChange}
          />
        </label>

        <p className="text-center font-mono text-xs text-secondary">
          JPEG, PNG oder WebP
        </p>
      </div>
    );
  }

  return (
    <label
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      className={cn(
        "brutal-press flex min-h-11 cursor-pointer flex-col items-center justify-center gap-2 rounded-md border-2 border-dashed border-control bg-primary-subtle p-6 text-center hover:bg-primary/25 focus-within:outline-2 focus-within:outline-offset-2 focus-within:outline-focus",
        isDragging && "border-strong bg-primary/30",
      )}
    >
      <Camera className="size-7 text-ink" aria-hidden="true" />
      <span className="text-sm font-bold text-ink">
        Foto auswählen oder hierher ziehen
      </span>
      <span className="font-mono text-xs text-secondary">
        JPEG, PNG oder WebP
      </span>
      <input
        type="file"
        accept="image/*"
        className="sr-only"
        onChange={handleFileChange}
      />
    </label>
  );
}
