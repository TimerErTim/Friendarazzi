"use client";

import * as SliderPrimitive from "@radix-ui/react-slider";
import { cn } from "@/lib/cn";

export type Checkpoint = {
  id: string;
  wert: number;
  farbe: string;
  label: string;
  anzeige?: string;
};

type CheckpointSliderProps = {
  checkpoints: Checkpoint[];
  value: number;
  onValueChange: (next: number) => void;
  onValueCommit?: (next: number) => void;
  min: number;
  max: number;
  step?: number;
  label: string;
  einheit?: string;
  disabled?: boolean;
  thumbHidden?: boolean;
  className?: string;
};

export function CheckpointSlider({
  checkpoints,
  value,
  onValueChange,
  onValueCommit,
  min,
  max,
  step = 1,
  label,
  einheit,
  disabled = false,
  thumbHidden = false,
  className,
}: CheckpointSliderProps) {
  const span = max - min;
  const sorted = [...checkpoints].sort((a, b) => a.wert - b.wert);
  const position = (wert: number) =>
    span === 0 ? 0 : Math.min(100, Math.max(0, ((wert - min) / span) * 100));

  const farbe =
    sorted.length === 0
      ? "var(--bg-surface-muted)"
      : `linear-gradient(to right, ${[
          `${sorted[0]?.farbe} 0%`,
          ...sorted.map(
            (eintrag) => `${eintrag.farbe} ${position(eintrag.wert)}%`,
          ),
          `${sorted[sorted.length - 1]?.farbe} 100%`,
        ].join(", ")})`;

  const nearest = sorted.reduce<Checkpoint | null>((best, entry) => {
    if (!best) return entry;
    return Math.abs(entry.wert - value) < Math.abs(best.wert - value)
      ? entry
      : best;
  }, null);

  const unitText = einheit ? ` ${einheit}` : "";
  const valueText = `${value}${unitText}`;
  const valuetext = nearest
    ? `${valueText}, nächster Punkt ${nearest.label}`
    : valueText;

  return (
    <div className={cn("w-full", className)}>
      <div className="relative h-5">
        {sorted.map((entry) =>
          entry.anzeige ? (
            <span
              key={entry.id}
              aria-hidden="true"
              className="absolute -translate-x-1/2 font-mono text-xs whitespace-nowrap text-secondary tabular-nums"
              style={{ left: `${position(entry.wert)}%` }}
            >
              {entry.anzeige}
            </span>
          ) : null,
        )}
      </div>
      <SliderPrimitive.Root
        className={cn(
          "relative flex h-11 w-full touch-none items-center select-none data-[disabled]:opacity-50",
          thumbHidden && "cursor-pointer",
        )}
        disabled={disabled}
        value={[value]}
        onValueChange={([next]) => {
          if (typeof next === "number") onValueChange(next);
        }}
        onValueCommit={([next]) => {
          if (typeof next === "number") onValueCommit?.(next);
        }}
        min={min}
        max={max}
        step={step}
      >
        <SliderPrimitive.Track className="relative h-3.5 w-full grow overflow-hidden rounded-sm border-2 border-control">
          <span
            aria-hidden="true"
            className="absolute inset-0"
            style={{ background: farbe }}
          />
          <span aria-hidden="true" className="absolute inset-0">
            {sorted.map((entry) => (
              <span
                key={entry.id}
                className="absolute top-0 h-full w-0.5 bg-strong"
                style={{ left: `${position(entry.wert)}%` }}
              />
            ))}
          </span>
        </SliderPrimitive.Track>
        <SliderPrimitive.Thumb
          aria-label={label}
          aria-valuetext={valuetext}
          tabIndex={thumbHidden ? -1 : undefined}
          className={cn(
            thumbHidden && "invisible",
            "brutal-shadow-sm relative block size-5 cursor-grab rounded-full border-2 border-control bg-surface",
            "after:absolute after:inset-0 after:m-auto after:size-2 after:rounded-full after:bg-ink after:content-['']",
            "before:absolute before:-inset-[16px] before:content-['']",
            "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus",
            "active:cursor-grabbing disabled:pointer-events-none disabled:cursor-not-allowed",
          )}
        />
      </SliderPrimitive.Root>
    </div>
  );
}
