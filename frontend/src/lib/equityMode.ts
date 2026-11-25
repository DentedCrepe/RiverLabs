/**
 * How your equity is shown at the Play table (a per-device setting): "request" (a button shows
 * it for the current decision), "always" (shown at every decision) or "off".
 */
import { useSyncExternalStore } from "react";

export type EquityMode = "request" | "always" | "off";
const KEY = "riverlab.equityMode";
const EVENT = "riverlab-equity-mode";

export function equityMode(): EquityMode {
  try {
    const v = typeof window !== "undefined" ? window.localStorage.getItem(KEY) : null;
    return v === "always" || v === "off" ? v : "request";
  } catch {
    return "request";
  }
}

export function setEquityMode(mode: EquityMode) {
  try {
    window.localStorage.setItem(KEY, mode);
  } catch {
    // not remembered: the choice still applies until the page reloads
  }
  window.dispatchEvent(new Event(EVENT));
}

function subscribe(cb: () => void): () => void {
  window.addEventListener(EVENT, cb);
  window.addEventListener("storage", cb);
  return () => {
    window.removeEventListener(EVENT, cb);
    window.removeEventListener("storage", cb);
  };
}

export function useEquityMode(): EquityMode {
  return useSyncExternalStore(subscribe, equityMode, () => "request");
}
