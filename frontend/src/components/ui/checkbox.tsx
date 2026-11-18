import type { InputHTMLAttributes } from "react";

import { cn } from "@/lib/cn";

type CheckboxProps = Omit<InputHTMLAttributes<HTMLInputElement>, "type"> & {
  label: string;
};

export function Checkbox({ className, label, ...props }: CheckboxProps) {
  return (
    <label
      className={cn(
        "relative flex min-h-11 w-full cursor-pointer items-center gap-3 text-sm font-medium text-ink",
        className,
      )}
    >
      <input
        type="checkbox"
        className={cn(
          "checkbox-tick brutal-shadow-sm size-5 shrink-0 cursor-pointer appearance-none rounded-sm border-2 border-control bg-surface",
          "transition-colors hover:bg-surface-muted",
          "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus",
          "disabled:cursor-not-allowed disabled:opacity-50",
        )}
        {...props}
      />
      {label}
    </label>
  );
}
