import { Search } from "lucide-react";
import type { InputHTMLAttributes } from "react";

import { cn } from "@/lib/cn";

type SearchInputProps = InputHTMLAttributes<HTMLInputElement>;

export function SearchInput({ className, id, ...props }: SearchInputProps) {
  return (
    <div className="relative w-full max-w-xs">
      <Search
        aria-hidden="true"
        className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-ink"
      />
      <input
        id={id}
        type="search"
        className={cn(
          "brutal-shadow-sm h-11 w-full rounded-sm border-2 border-control bg-elevated pr-3 pl-9 text-sm font-medium text-ink",
          "transition-colors placeholder:text-muted",
          "hover:bg-surface-muted/40",
          "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus",
          "disabled:cursor-not-allowed disabled:bg-surface-muted disabled:text-muted disabled:shadow-none",
          className,
        )}
        {...props}
      />
    </div>
  );
}
