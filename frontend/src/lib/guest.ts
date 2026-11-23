/**
 * Guest mode: someone else trains on this RiverLab without changing the owner's statistics.
 * A per-device switch kept in localStorage; while it is on, every API request carries
 * `X-RiverLab-Guest: 1` and the backend grades answers and plays games but records nothing
 * towards progress, streak, goals, analytics or leaks (docs/guest-mode.md).
 */
import { useSyncExternalStore } from "react";

const KEY = "riverlab.guest";
const EVENT = "riverlab-guest-change";

export const GUEST_HEADER = "X-RiverLab-Guest";

export function isGuest(): boolean {
  try {
    return typeof window !== "undefined" && window.localStorage.getItem(KEY) === "1";
  } catch {
    return false; // storage blocked: guest mode cannot be on
  }
}

/** Turns guest mode on or off. Returns false if the browser would not store the choice. */
export function setGuest(on: boolean): boolean {
  try {
    if (on) window.localStorage.setItem(KEY, "1");
    else window.localStorage.removeItem(KEY);
  } catch {
    return false;
  }
  window.dispatchEvent(new Event(EVENT));
  return isGuest() === on;
}

function subscribe(cb: () => void): () => void {
  window.addEventListener(EVENT, cb);
  window.addEventListener("storage", cb);
  return () => {
    window.removeEventListener(EVENT, cb);
    window.removeEventListener("storage", cb);
  };
}

export function useGuest(): boolean {
  return useSyncExternalStore(subscribe, isGuest, () => false);
}
