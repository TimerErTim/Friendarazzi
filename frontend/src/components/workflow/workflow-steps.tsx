import { cn } from "@/lib/cn";

type Step = {
  id: string;
  label: string;
  done: boolean;
  optional?: boolean;
};

type WorkflowStepsProps = {
  steps: Step[];
  currentStepId: string;
  optionalLabel?: string;
};

export function WorkflowSteps({
  steps,
  currentStepId,
  optionalLabel = "optional",
}: WorkflowStepsProps) {
  const currentIdx = steps.findIndex((step) => step.id === currentStepId);

  return (
    <ol
      aria-label={
        currentIdx >= 0
          ? `Schritt ${currentIdx + 1} von ${steps.length}: ${steps[currentIdx]?.label}`
          : undefined
      }
      className="flex flex-wrap items-center gap-x-2 gap-y-3"
    >
      {steps.map((step, index) => {
        const isCurrent = step.id === currentStepId;
        const isDone = step.done;
        const isLast = index === steps.length - 1;

        return (
          <li
            key={step.id}
            aria-current={isCurrent ? "step" : undefined}
            className="flex items-center gap-2"
          >
            <span
              className={cn(
                "brutal-shadow-sm flex size-7 items-center justify-center rounded-sm border-2 border-strong font-mono text-xs font-bold tabular-nums",
                isCurrent
                  ? "bg-primary text-primary-foreground"
                  : isDone
                    ? "bg-cyan text-cyan-foreground"
                    : "bg-surface text-secondary",
              )}
            >
              {index + 1}
            </span>
            <span
              className={cn(
                "text-sm",
                isCurrent
                  ? "font-bold text-ink"
                  : isDone
                    ? "font-semibold text-ink"
                    : "font-medium text-secondary",
              )}
            >
              {step.label}
              {isCurrent ? (
                <span className="ml-1.5 font-mono text-xs text-secondary">
                  ({index + 1}/{steps.length})
                </span>
              ) : null}
            </span>
            {step.optional ? (
              <span className="font-mono text-xs text-secondary">
                ({optionalLabel})
              </span>
            ) : null}
            {isLast ? null : (
              <span aria-hidden="true" className="h-0.5 w-6 bg-strong" />
            )}
          </li>
        );
      })}
    </ol>
  );
}
