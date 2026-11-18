"use client";

import { Eye, EyeOff } from "lucide-react";
import { type InputHTMLAttributes, useState } from "react";

import { cn } from "@/lib/cn";
import { Input } from "./input";

type PasswordInputProps = Omit<InputHTMLAttributes<HTMLInputElement>, "type">;

export function PasswordInput({
  className,
  disabled,
  ...props
}: PasswordInputProps) {
  const [visible, setVisible] = useState(false);

  return (
    <div className="relative">
      <Input
        {...props}
        className={cn("pr-12", className)}
        disabled={disabled}
        type={visible ? "text" : "password"}
      />
      <button
        type="button"
        aria-label={visible ? "Passwort verbergen" : "Passwort anzeigen"}
        aria-pressed={visible}
        disabled={disabled}
        onClick={() => setVisible((current) => !current)}
        className={cn(
          "absolute inset-y-0 right-0 flex w-11 cursor-pointer items-center justify-center rounded-sm",
          "text-ink transition-colors hover:bg-primary/30",
          "focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-focus",
          "disabled:cursor-not-allowed disabled:text-muted",
        )}
      >
        {visible ? (
          <EyeOff aria-hidden="true" className="size-4" />
        ) : (
          <Eye aria-hidden="true" className="size-4" />
        )}
      </button>
    </div>
  );
}
