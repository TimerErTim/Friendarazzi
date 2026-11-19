import { type ComponentPropsWithRef, useEffect, useRef } from "react";

import { cn } from "@/lib/cn";

type TextareaProps = ComponentPropsWithRef<"textarea"> & {
  autoGrow?: boolean;
};

export function Textarea({
  className,
  rows,
  autoGrow = true,
  onChange,
  ref,
  value,
  defaultValue,
  ...props
}: TextareaProps) {
  const internalRef = useRef<HTMLTextAreaElement>(null);

  const grow = useRef((element: HTMLTextAreaElement) => {
    element.style.height = "auto";
    element.style.height = `${element.scrollHeight}px`;
  });

  useEffect(() => {
    if (!autoGrow || !internalRef.current) return;
    const nextValue = value ?? defaultValue;
    if (typeof nextValue === "string") internalRef.current.value = nextValue;
    grow.current(internalRef.current);
  }, [autoGrow, defaultValue, value]);

  return (
    <textarea
      ref={(element) => {
        internalRef.current = element;
        if (typeof ref === "function") ref(element);
        else if (ref) ref.current = element;
      }}
      rows={rows ?? 1}
      value={value}
      defaultValue={defaultValue}
      onChange={(event) => {
        onChange?.(event);
        if (autoGrow) grow.current(event.currentTarget);
      }}
      className={cn(
        "brutal-shadow-sm min-h-11 w-full rounded-sm border-2 border-control bg-elevated px-3 py-2 text-sm font-medium text-ink",
        "transition-colors placeholder:text-muted",
        "hover:bg-surface-muted/40",
        "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus",
        "disabled:cursor-not-allowed disabled:bg-surface-muted disabled:text-muted disabled:shadow-none",
        "aria-invalid:border-impact aria-invalid:focus-visible:outline-impact",
        "resize-none",
        className,
      )}
      {...props}
    />
  );
}
