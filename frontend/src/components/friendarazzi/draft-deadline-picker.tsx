"use client";

import { Clock } from "lucide-react";

import { Input } from "@/components/ui/input";
import { cn } from "@/lib/cn";

export type DeadlinePresetKey = "none" | "15m" | "1h" | "24h" | "custom";

const DEADLINE_PRESETS: ReadonlyArray<{
  key: Exclude<DeadlinePresetKey, "custom">;
  label: string;
  offsetMinutes: number | null;
}> = [
  { key: "none", label: "Kein Limit", offsetMinutes: null },
  { key: "15m", label: "+15 Min.", offsetMinutes: 15 },
  { key: "1h", label: "+1 Std.", offsetMinutes: 60 },
  { key: "24h", label: "+24 Std.", offsetMinutes: 24 * 60 },
];

export function toLocalDateTimeInputValue(
  iso: string | null | undefined,
): string {
  if (!iso) return "";
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

export function fromLocalDateTimeInputValue(value: string): string | null {
  if (!value.trim()) return null;
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return null;
  return date.toISOString();
}

export function computePresetDeadlineIso(
  offsetMinutes: number,
  nowMs: number = Date.now(),
): string {
  return new Date(nowMs + offsetMinutes * 60_000).toISOString();
}

export function formatDeadlineShort(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return date.toLocaleString("de-DE", {
    day: "2-digit",
    month: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/**
 * Shared preset + custom deadline picker used for draft auto-close deadlines.
 */
export function DraftDeadlinePicker({
  legend = "Entwurfsfrist (optional)",
  description = "Danach werden offene Felder automatisch aus Starter-Decks gefüllt.",
  preset,
  customLocalValue,
  currentIso,
  onSelectPreset,
  onCustomChange,
  disabled = false,
}: {
  legend?: string;
  description?: string;
  preset: DeadlinePresetKey;
  customLocalValue: string;
  currentIso?: string | null;
  onSelectPreset: (preset: DeadlinePresetKey, nextIso: string | null) => void;
  onCustomChange: (localValue: string, nextIso: string | null) => void;
  disabled?: boolean;
}) {
  return (
    <fieldset className="space-y-2">
      <legend className="text-xs font-bold tracking-[0.06em] text-ink uppercase">
        {legend}
      </legend>
      <p className="text-xs text-secondary">{description}</p>
      <div className="flex flex-wrap gap-2">
        {DEADLINE_PRESETS.map((opt) => {
          const selected = preset === opt.key;
          return (
            <button
              key={opt.key}
              type="button"
              disabled={disabled}
              aria-pressed={selected}
              onClick={() => {
                const nextIso =
                  opt.offsetMinutes === null
                    ? null
                    : computePresetDeadlineIso(opt.offsetMinutes);
                onSelectPreset(opt.key, nextIso);
              }}
              className={cn(
                "brutal-press inline-flex min-h-11 cursor-pointer items-center rounded-sm border-2 border-control px-3 py-1.5 font-mono text-xs font-bold focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus disabled:cursor-not-allowed disabled:opacity-50",
                selected
                  ? "bg-cyan text-cyan-foreground"
                  : "bg-surface text-ink hover:bg-surface-muted",
              )}
            >
              {opt.label}
            </button>
          );
        })}
        <button
          type="button"
          disabled={disabled}
          aria-pressed={preset === "custom"}
          onClick={() => {
            const fallbackLocal =
              customLocalValue ||
              toLocalDateTimeInputValue(
                currentIso ?? computePresetDeadlineIso(60),
              );
            onSelectPreset(
              "custom",
              fromLocalDateTimeInputValue(fallbackLocal),
            );
          }}
          className={cn(
            "brutal-press inline-flex min-h-11 cursor-pointer items-center gap-1.5 rounded-sm border-2 border-control px-3 py-1.5 font-mono text-xs font-bold focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus disabled:cursor-not-allowed disabled:opacity-50",
            preset === "custom"
              ? "bg-cyan text-cyan-foreground"
              : "bg-surface text-ink hover:bg-surface-muted",
          )}
        >
          <Clock className="size-3.5" aria-hidden="true" />
          Eigene Zeit
        </button>
      </div>
      {preset === "custom" ? (
        <div className="pt-1">
          <Input
            type="datetime-local"
            aria-label={`${legend} Datum und Uhrzeit`}
            disabled={disabled}
            value={customLocalValue}
            onChange={(e) => {
              const local = e.target.value;
              onCustomChange(local, fromLocalDateTimeInputValue(local));
            }}
          />
        </div>
      ) : null}
      {currentIso && preset !== "none" ? (
        <p className="font-mono text-[11px] font-bold text-secondary">
          Automatischer Abschluss am {formatDeadlineShort(currentIso)}
        </p>
      ) : null}
    </fieldset>
  );
}
