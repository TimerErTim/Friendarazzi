import { describe, expect, it } from "vitest";

import {
  ALL_DECKS_SLUG,
  autofillEmptySlots,
  fillPromptTemplate,
  mergeDeckTemplates,
  nextPromptSuggestions,
  normalizePromptText,
  rerollSingleSlot,
} from "./draft-prompts.ts";

type TestSlot = {
  key: string;
  targetName?: string;
  text: string;
  source: "custom" | "deck";
};

function withDeckText(slot: TestSlot, text: string): TestSlot {
  return { ...slot, text, source: "deck" };
}

describe("draft-prompts", () => {
  it("fillPromptTemplate trims target-agnostic templates and strips legacy {name} if present", () => {
    expect(fillPromptTemplate("  Bestellt noch ein Getränk  ")).toBe(
      "Bestellt noch ein Getränk",
    );
    expect(
      fillPromptTemplate("  {name} bestellt noch ein Getränk  ", "Mara"),
    ).toBe("Mara bestellt noch ein Getränk");
  });

  it("autofillEmptySlots fills empty slots uniquely across targets and preserves custom prompts", () => {
    const slots: TestSlot[] = [
      {
        key: "a-0",
        targetName: "Mara",
        text: "Custom Prompt",
        source: "custom",
      },
      { key: "a-1", targetName: "Mara", text: "", source: "custom" },
      { key: "b-0", targetName: "Luca", text: "", source: "custom" },
    ];
    const templates = [
      "Custom Prompt",
      "Lacht über den eigenen Witz",
      "Tanzt kurz alleine im Stehen",
    ];

    const result = autofillEmptySlots(
      slots,
      templates,
      withDeckText,
      () => 0.4,
    );

    expect(result.filledCount).toBe(2);
    expect(result.remainingEmptyCount).toBe(0);
    expect(result.slots[0]?.text).toBe("Custom Prompt");
    expect(result.slots[0]?.source).toBe("custom");
    const normalized = result.slots.map((s) => normalizePromptText(s.text));
    expect(new Set(normalized).size).toBe(3);
  });

  it("autofillEmptySlots detects case-insensitive collisions and duplicate templates", () => {
    const slots: TestSlot[] = [
      {
        key: "a-0",
        targetName: "Mara",
        text: "Lacht über den eigenen Witz",
        source: "custom",
      },
      { key: "a-1", targetName: "Mara", text: "", source: "custom" },
      { key: "b-0", targetName: "Luca", text: "", source: "custom" },
    ];
    const templates = [
      "Lacht über den eigenen Witz",
      "lacht über den eigenen witz",
      "Winkt jemandem aus der Ferne zu",
      "Winkt jemandem aus der Ferne zu",
      "Bestellt noch eine Runde",
    ];

    const result = autofillEmptySlots(slots, templates, withDeckText, () => 0);

    expect(result.filledCount).toBe(2);
    expect(result.remainingEmptyCount).toBe(0);
    expect(result.slots[1]?.text).toBe("Winkt jemandem aus der Ferne zu");
    expect(result.slots[2]?.text).toBe("Bestellt noch eine Runde");
  });

  it("autofillEmptySlots leaves remaining slots empty when deck runs out of unique prompts", () => {
    const slots: TestSlot[] = [
      { key: "a-0", targetName: "Mara", text: "", source: "custom" },
      { key: "a-1", targetName: "Mara", text: "", source: "custom" },
      { key: "a-2", targetName: "Mara", text: "", source: "custom" },
    ];
    const templates = [
      "Lacht über den eigenen Witz",
      "Tanzt kurz alleine im Stehen",
      "Lacht über den eigenen Witz",
    ];

    const result = autofillEmptySlots(
      slots,
      templates,
      withDeckText,
      () => 0.1,
    );

    expect(result.filledCount).toBe(2);
    expect(result.remainingEmptyCount).toBe(1);
    const nonEmpty = result.slots.filter((s) => s.text.length > 0);
    expect(nonEmpty.length).toBe(2);
    expect(new Set(nonEmpty.map((s) => s.text)).size).toBe(2);
    expect(result.slots[2]?.text).toBe("");
  });

  it("rerollSingleSlot excludes current slot value and other occupied slots", () => {
    const slots: TestSlot[] = [
      {
        key: "a-0",
        targetName: "Mara",
        text: "Lacht über den eigenen Witz",
        source: "deck",
      },
      {
        key: "a-1",
        targetName: "Mara",
        text: "Tanzt kurz alleine im Stehen",
        source: "custom",
      },
    ];
    const templates = [
      "Lacht über den eigenen Witz",
      "Tanzt kurz alleine im Stehen",
      "Winkt jemandem aus der Ferne zu",
    ];

    const result = rerollSingleSlot(
      slots,
      "a-0",
      templates,
      withDeckText,
      () => 0,
    );

    expect(result.changed).toBe(true);
    expect(result.slots[0]?.text).toBe("Winkt jemandem aus der Ferne zu");
    expect(result.slots[0]?.source).toBe("deck");
    expect(result.slots[1]?.text).toBe("Tanzt kurz alleine im Stehen");
  });

  it("rerollSingleSlot keeps slot unchanged when no unique alternative exists", () => {
    const slots: TestSlot[] = [
      {
        key: "a-0",
        targetName: "Mara",
        text: "Lacht über den eigenen Witz",
        source: "deck",
      },
      {
        key: "a-1",
        targetName: "Mara",
        text: "Tanzt kurz alleine im Stehen",
        source: "custom",
      },
    ];
    const templates = [
      "Lacht über den eigenen Witz",
      "tanzt kurz alleine im stehen",
    ];

    const result = rerollSingleSlot(
      slots,
      "a-0",
      templates,
      withDeckText,
      () => 0,
    );

    expect(result.changed).toBe(false);
    expect(result.slots[0]?.text).toBe("Lacht über den eigenen Witz");
    expect(result.slots[1]?.text).toBe("Tanzt kurz alleine im Stehen");
  });

  it("mergeDeckTemplates deduplicates across starter decks", () => {
    expect(ALL_DECKS_SLUG).toBe("__all__");
    const merged = mergeDeckTemplates([
      { prompts: ["Trinkt Wasser", "Bestellt ein Getränk"] },
      { prompts: ["trinkt wasser", "Zeigt ein altes Foto"] },
    ]);
    expect(merged).toEqual([
      "Trinkt Wasser",
      "Bestellt ein Getränk",
      "Zeigt ein altes Foto",
    ]);
  });

  it("nextPromptSuggestions filters out prompts already in slots", () => {
    const slots: TestSlot[] = [
      {
        key: "a-0",
        targetName: "Mara",
        text: "Trinkt Wasser",
        source: "custom",
      },
      { key: "a-1", targetName: "Mara", text: "", source: "custom" },
    ];
    const suggestions = nextPromptSuggestions(
      ["trinkt wasser", "Macht ein Selfie", "Bringt Snacks"],
      slots,
      4,
      () => 0,
    );
    expect(suggestions).toHaveLength(2);
    expect(suggestions).toContain("Macht ein Selfie");
    expect(suggestions).toContain("Bringt Snacks");
  });
});
