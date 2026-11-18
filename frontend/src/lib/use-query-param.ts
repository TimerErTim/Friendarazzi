"use client";

import { useEffect, useState } from "react";

const LOCATION_CHANGE_EVENT = "friendarazzi:locationchange";

let historyHooked = false;

function ensureHistoryPatched(): void {
  if (historyHooked || typeof window === "undefined") {
    return;
  }
  historyHooked = true;

  for (const method of ["pushState", "replaceState"] as const) {
    const original = window.history[method];
    window.history[method] = function patchedHistory(...args) {
      const result = original.apply(this, args);
      queueMicrotask(() => {
        window.dispatchEvent(new Event(LOCATION_CHANGE_EVENT));
      });
      return result;
    };
  }
}

/**
 * Reads URL search parameter `name` safely under static export.
 * Returns `null` before client mount, `""` when absent, or the param value.
 */
export function useQueryParam(name: string): string | null {
  const [value, setValue] = useState<string | null>(null);

  useEffect(() => {
    ensureHistoryPatched();

    const read = () =>
      setValue(new URLSearchParams(window.location.search).get(name) ?? "");

    read();
    window.addEventListener("popstate", read);
    window.addEventListener(LOCATION_CHANGE_EVENT, read);
    return () => {
      window.removeEventListener("popstate", read);
      window.removeEventListener(LOCATION_CHANGE_EVENT, read);
    };
  }, [name]);

  return value;
}
