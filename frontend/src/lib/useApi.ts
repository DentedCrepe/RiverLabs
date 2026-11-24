"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { ApiError, api } from "./api";

export interface ApiState<T> {
  data: T | null;
  error: ApiError | null;
  loading: boolean;
  reload: () => void;
}

/**
 * Fetches a GET endpoint on mount. `refreshMs` re-fetches it in the background: a fixed interval,
 * or a function of the latest data (return `null` to stop, e.g. a slower interval when nothing is
 * running). Background refreshes pause while the page is hidden (screen off, app in the
 * background) and run once as soon as it is shown again. `null` path: fetch nothing.
 */
export function useApi<T>(
  path: string | null,
  opts: { refreshMs?: number | ((data: T | null) => number | null) } = {},
): ApiState<T> {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<ApiError | null>(null);
  const [loading, setLoading] = useState(path !== null);
  const [tick, setTick] = useState(0);
  const reload = useCallback(() => setTick((t) => t + 1), []);
  // Interval refreshes are marked as polling so they don't keep an idle RiverLab awake.
  const polling = useRef(false);

  useEffect(() => {
    if (path === null) return;
    let cancelled = false;
    const poll = polling.current;
    polling.current = false;
    api
      .get<T>(path, { poll })
      .then((d) => {
        if (!cancelled) {
          setData(d);
          setError(null);
        }
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(e instanceof ApiError ? e : new ApiError(0, "unknown", String(e)));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [path, tick]);

  const refresh = opts.refreshMs;
  const every = typeof refresh === "function" ? refresh(data) : (refresh ?? null);
  useEffect(() => {
    if (!every || path === null) return;
    const hidden = () => typeof document !== "undefined" && document.visibilityState === "hidden";
    const id = setInterval(() => {
      if (hidden()) return; // nobody is looking: save the battery and the API
      polling.current = true;
      reload();
    }, every);
    const onVisible = () => {
      if (!hidden()) {
        polling.current = true;
        reload();
      }
    };
    document.addEventListener("visibilitychange", onVisible);
    return () => {
      clearInterval(id);
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, [every, reload, path]);

  return { data, error, loading, reload };
}
