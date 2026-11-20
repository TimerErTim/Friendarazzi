import assert from "node:assert/strict";
import test from "node:test";

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

test("fillPromptTemplate substitutes {name} and trims surrounding whitespace", () => {
  assert.equal(
    fillPromptTemplate("  {name} bestellt noch ein Getränk  ", "Mara"),
    "Mara bestellt noch ein Getränk",
  );
});

test("autofillEmptySlots fills empty slots uniquely and preserves custom prompts", () => {
  const slots: TestSlot[] = [
    { key: "a-0", targetName: "Mara", text: "Custom Prompt", source: "custom" },
    { key: "a-1", targetName: "Mara", text: "", source: "custom" },
    { key: "b-0", targetName: "Luca", text: "", source: "custom" },
  ];
  const templates = ["Custom Prompt", "{name} lacht", "{name} tanzt"];

  const result = autofillEmptySlots(slots, templates, withDeckText, () => 0.4);

  assert.equal(result.filledCount, 2);
  assert.equal(result.remainingEmptyCount, 0);
  assert.equal(result.slots[0]?.text, "Custom Prompt");
  assert.equal(result.slots[0]?.source, "custom");
  const normalized = result.slots.map((s) => normalizePromptText(s.text));
  assert.equal(new Set(normalized).size, 3);
});

test("autofillEmptySlots detects collisions after {name} substitution and duplicate templates", () => {
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

  assert.equal(result.filledCount, 2);
  assert.equal(result.remainingEmptyCount, 0);
  assert.equal(result.slots[1]?.text, "Mara winkt");
  assert.equal(result.slots[2]?.text, "Luca winkt");
});

test("autofillEmptySlots leaves remaining slots empty when deck runs out of unique prompts", () => {
  const slots: TestSlot[] = [
    { key: "a-0", targetName: "Mara", text: "", source: "custom" },
    { key: "a-1", targetName: "Mara", text: "", source: "custom" },
    { key: "a-2", targetName: "Mara", text: "", source: "custom" },
  ];
  const templates = ["{name} lacht", "{name} tanzt", "{name} lacht"];

  const result = autofillEmptySlots(slots, templates, withDeckText, () => 0.1);

  assert.equal(result.filledCount, 2);
  assert.equal(result.remainingEmptyCount, 1);
  const nonEmpty = result.slots.filter((s) => s.text.length > 0);
  assert.equal(nonEmpty.length, 2);
  assert.equal(new Set(nonEmpty.map((s) => s.text)).size, 2);
  assert.equal(result.slots[2]?.text, "");
});

test("rerollSingleSlot excludes current slot value and other occupied slots", () => {
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

  assert.equal(result.changed, true);
  assert.equal(result.slots[0]?.text, "Mara winkt");
  assert.equal(result.slots[0]?.source, "deck");
  assert.equal(result.slots[1]?.text, "Mara tanzt");
});

test("rerollSingleSlot keeps slot unchanged when no unique alternative exists", () => {
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

  assert.equal(result.changed, false);
  assert.equal(result.slots[0]?.text, "Mara lacht");
  assert.equal(result.slots[1]?.text, "Mara tanzt");
});
