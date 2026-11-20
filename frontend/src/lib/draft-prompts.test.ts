import { describe, expect, it } from "vitest";

import {
  autofillEmptySlots,
  fillPromptTemplate,
  normalizePromptText,
  rerollSingleSlot,
} from "./draft-prompts.ts";

type TestSlot = {
  key: string;
  targetName: string;
  text: string;
  source: "custom" | "deck";
};

function withDeckText(slot: TestSlot, text: string): TestSlot {
  return { ...slot, text, source: "deck" };
}

describe("draft-prompts", () => {
  it("fillPromptTemplate substitutes {name} and trims surrounding whitespace", () => {
    expect(
      fillPromptTemplate("  {name} bestellt noch ein Getränk  ", "Mara"),
    ).toBe("Mara bestellt noch ein Getränk");
  });

  it("autofillEmptySlots fills empty slots uniquely and preserves custom prompts", () => {
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
    const templates = ["Custom Prompt", "{name} lacht", "{name} tanzt"];

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

  it("autofillEmptySlots detects collisions after {name} substitution and duplicate templates", () => {
    const slots: TestSlot[] = [
      { key: "a-0", targetName: "Mara", text: "Mara lacht", source: "custom" },
      { key: "a-1", targetName: "Mara", text: "", source: "custom" },
      { key: "b-0", targetName: "Luca", text: "", source: "custom" },
    ];
    const templates = [
      "{name} lacht",
      "mara lacht",
      "{name} winkt",
      "{name} winkt",
    ];

    const result = autofillEmptySlots(slots, templates, withDeckText, () => 0);

    expect(result.filledCount).toBe(2);
    expect(result.remainingEmptyCount).toBe(0);
    expect(result.slots[1]?.text).toBe("Mara winkt");
    expect(result.slots[2]?.text).toBe("Luca winkt");
  });

  it("autofillEmptySlots leaves remaining slots empty when deck runs out of unique prompts", () => {
    const slots: TestSlot[] = [
      { key: "a-0", targetName: "Mara", text: "", source: "custom" },
      { key: "a-1", targetName: "Mara", text: "", source: "custom" },
      { key: "a-2", targetName: "Mara", text: "", source: "custom" },
    ];
    const templates = ["{name} lacht", "{name} tanzt", "{name} lacht"];

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
      { key: "a-0", targetName: "Mara", text: "Mara lacht", source: "deck" },
      { key: "a-1", targetName: "Mara", text: "Mara tanzt", source: "custom" },
    ];
    const templates = ["{name} lacht", "{name} tanzt", "{name} winkt"];

    const result = rerollSingleSlot(
      slots,
      "a-0",
      templates,
      withDeckText,
      () => 0,
    );

    expect(result.changed).toBe(true);
    expect(result.slots[0]?.text).toBe("Mara winkt");
    expect(result.slots[0]?.source).toBe("deck");
    expect(result.slots[1]?.text).toBe("Mara tanzt");
  });

  it("rerollSingleSlot keeps slot unchanged when no unique alternative exists", () => {
    const slots: TestSlot[] = [
      { key: "a-0", targetName: "Mara", text: "Mara lacht", source: "deck" },
      { key: "a-1", targetName: "Mara", text: "Mara tanzt", source: "custom" },
    ];
    const templates = ["{name} lacht", "mara tanzt"];

    const result = rerollSingleSlot(
      slots,
      "a-0",
      templates,
      withDeckText,
      () => 0,
    );

    expect(result.changed).toBe(false);
    expect(result.slots[0]?.text).toBe("Mara lacht");
    expect(result.slots[1]?.text).toBe("Mara tanzt");
  });
});
