"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { ApiError, api } from "./api";

export interface ApiState<T> {
  data: T | null;
  error: ApiError | null;
  loading: boolean;
  reload: () => void;
}

/** Fetches a GET endpoint on mount (and optionally on an interval). `null`: fetch nothing. */
export function useApi<T>(path: string | null, opts: { refreshMs?: number } = {}): ApiState<T> {
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

  useEffect(() => {
    if (!opts.refreshMs || path === null) return;
    const id = setInterval(() => {
      polling.current = true;
      reload();
    }, opts.refreshMs);
    return () => clearInterval(id);
  }, [opts.refreshMs, reload, path]);

  return { data, error, loading, reload };
}
