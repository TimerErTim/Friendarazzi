import {
  Children,
  cloneElement,
  type HTMLAttributes,
  isValidElement,
  type ReactElement,
  type ReactNode,
  useId,
} from "react";

import { cn } from "@/lib/cn";
import { Label } from "./label";

type FieldProps = HTMLAttributes<HTMLDivElement> & {
  label: string;
  hint?: string;
  error?: string;
  optional?: boolean;
  labelHidden?: boolean;
  hintAlignment?: "start" | "end";
};

type LabelledProps = {
  id?: string;
  label?: string;
  "aria-label"?: string;
  "aria-labelledby"?: string;
};

export function Field({
  label,
  hint,
  error,
  optional = false,
  labelHidden = false,
  hintAlignment = "start",
  className,
  children,
  ...props
}: FieldProps) {
  const generatedId = useId();
  const single: ReactElement<LabelledProps> | null =
    Children.count(children) === 1 && isValidElement<LabelledProps>(children)
      ? children
      : null;

  const selfLabelled = Boolean(
    single?.props.label ??
      single?.props["aria-label"] ??
      single?.props["aria-labelledby"],
  );
  const controlId = selfLabelled
    ? undefined
    : (single?.props.id ?? generatedId);

  const control =
    single && controlId && !single.props.id
      ? cloneElement(single, { id: controlId })
      : (children as ReactNode);

  return (
    <div className={cn("space-y-1.5", className)} {...props}>
      <span
        className={cn(
          "flex min-h-[18px] items-center justify-between gap-2",
          labelHidden && "sr-only",
        )}
      >
        <Label htmlFor={controlId}>{label}</Label>
        {optional ? (
          <span className="font-mono text-[11px] font-medium text-secondary">
            optional
          </span>
        ) : null}
      </span>
      {control}
      {error ? (
        <p role="alert" className="text-xs font-semibold text-impact">
          {error}
        </p>
      ) : hint ? (
        <p
          className={cn(
            "text-xs text-secondary",
            hintAlignment === "end" && "text-right",
          )}
        >
          {hint}
        </p>
      ) : null}
    </div>
  );
}

type FormRowProps = HTMLAttributes<HTMLDivElement> & {
  actions?: ReactNode;
};

export function FormRow({
  actions,
  className,
  children,
  ...props
}: FormRowProps) {
  return (
    <div className={cn("grid items-start gap-4", className)} {...props}>
      {children}
      {actions ? <FormActions>{actions}</FormActions> : null}
    </div>
  );
}

export function FormActions({
  className,
  children,
  ...props
}: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn("flex items-center gap-2 self-start pt-6", className)}
      {...props}
    >
      {children}
    </div>
  );
}
