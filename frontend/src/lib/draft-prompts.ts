export const PROMPT_MAX_CHARS = 80;
export const PROMPT_WARN_CHARS = 60;
export const ALL_DECKS_SLUG = "__all__";

export type DraftPromptSlot = {
  key: string;
  targetName?: string;
  text: string;
};

export function fillPromptTemplate(template: string, targetName = ""): string {
  return template.replaceAll("{name}", targetName).trim();
}

export function normalizePromptText(text: string): string {
  return text.trim().toLocaleLowerCase();
}

function shuffledTemplates(
  templates: readonly string[],
  random: () => number,
): string[] {
  const pool = [...templates];
  for (let i = pool.length - 1; i > 0; i -= 1) {
    const j = Math.floor(random() * (i + 1));
    const current = pool[i];
    const swap = pool[j];
    if (current !== undefined && swap !== undefined) {
      pool[i] = swap;
      pool[j] = current;
    }
  }
  return pool;
}

export function autofillEmptySlots<T extends DraftPromptSlot>(
  slots: readonly T[],
  templates: readonly string[],
  onFill: (slot: T, text: string) => T,
  random: () => number = Math.random,
): {
  slots: T[];
  filledCount: number;
  remainingEmptyCount: number;
} {
  if (templates.length === 0) {
    const remainingEmptyCount = slots.filter(
      (slot) => slot.text.trim().length === 0,
    ).length;
    return { slots: [...slots], filledCount: 0, remainingEmptyCount };
  }

  const used = new Set(
    slots
      .map((slot) => normalizePromptText(slot.text))
      .filter((text) => text.length > 0),
  );
  const pool = shuffledTemplates(templates, random);
  let filledCount = 0;
  let remainingEmptyCount = 0;

  const nextSlots = slots.map((slot) => {
    if (slot.text.trim().length > 0) {
      return slot;
    }
    for (const template of pool) {
      const filled = fillPromptTemplate(template, slot.targetName ?? "");
      const normalized = normalizePromptText(filled);
      if (normalized.length > 0 && !used.has(normalized)) {
        used.add(normalized);
        filledCount += 1;
        return onFill(slot, filled);
      }
    }
    remainingEmptyCount += 1;
    return slot;
  });

  return { slots: nextSlots, filledCount, remainingEmptyCount };
}

export function rerollSingleSlot<T extends DraftPromptSlot>(
  slots: readonly T[],
  key: string,
  templates: readonly string[],
  onFill: (slot: T, text: string) => T,
  random: () => number = Math.random,
): {
  slots: T[];
  changed: boolean;
} {
  const target = slots.find((slot) => slot.key === key);
  if (!target || templates.length === 0) {
    return { slots: [...slots], changed: false };
  }

  // Exclude both the target slot's current value and every other occupied slot
  // so a reroll always produces a genuine alternative within the draft.
  const used = new Set(
    slots
      .map((slot) => normalizePromptText(slot.text))
      .filter((text) => text.length > 0),
  );

  const candidates: string[] = [];
  const candidateKeys = new Set<string>();
  for (const template of templates) {
    const filled = fillPromptTemplate(template, target.targetName ?? "");
    const normalized = normalizePromptText(filled);
    if (
      normalized.length > 0 &&
      !used.has(normalized) &&
      !candidateKeys.has(normalized)
    ) {
      candidateKeys.add(normalized);
      candidates.push(filled);
    }
  }

  if (candidates.length === 0) {
    return { slots: [...slots], changed: false };
  }

  const chosen =
    candidates[Math.floor(random() * candidates.length)] ?? candidates[0];
  if (!chosen) {
    return { slots: [...slots], changed: false };
  }

  const nextSlots = slots.map((slot) =>
    slot.key === key ? onFill(slot, chosen) : slot,
  );
  return { slots: nextSlots, changed: true };
}

export function mergeDeckTemplates(
  decks: readonly { prompts: readonly string[] }[],
): string[] {
  const seen = new Set<string>();
  const merged: string[] = [];
  for (const deck of decks) {
    for (const prompt of deck.prompts) {
      const normalized = normalizePromptText(prompt);
      if (normalized.length > 0 && !seen.has(normalized)) {
        seen.add(normalized);
        merged.push(prompt.trim());
      }
    }
  }
  return merged;
}

export function nextPromptSuggestions(
  templates: readonly string[],
  slots: readonly DraftPromptSlot[],
  limit = 4,
  random: () => number = Math.random,
): string[] {
  if (limit <= 0 || templates.length === 0) return [];
  const used = new Set(
    slots
      .map((slot) => normalizePromptText(slot.text))
      .filter((text) => text.length > 0),
  );
  const available: string[] = [];
  const seen = new Set<string>();
  for (const template of templates) {
    const filled = fillPromptTemplate(template);
    const normalized = normalizePromptText(filled);
    if (
      normalized.length > 0 &&
      !used.has(normalized) &&
      !seen.has(normalized)
    ) {
      seen.add(normalized);
      available.push(filled);
    }
  }
  return shuffledTemplates(available, random).slice(0, limit);
}
