"use client";

import { useEffect, useRef, useState } from "react";

import { remainingCountdownSeconds } from "@/lib/game-overview";

export function useGameCountdown(
  autoStartAt: string | null | undefined,
  serverNow: string | null | undefined,
  dataUpdatedAt: number,
  onExpire?: () => void,
): number | null {
  const [nowMs, setNowMs] = useState(() => Date.now());
  const firedForRef = useRef<string | null>(null);
  const onExpireRef = useRef(onExpire);
  onExpireRef.current = onExpire;

  useEffect(() => {
    if (!autoStartAt) {
      firedForRef.current = null;
      return;
    }
    setNowMs(Date.now());
    const timer = setInterval(() => {
      setNowMs(Date.now());
    }, 250);
    return () => clearInterval(timer);
  }, [autoStartAt]);

  const remaining = remainingCountdownSeconds(
    autoStartAt,
    serverNow,
    dataUpdatedAt > 0 ? nowMs - dataUpdatedAt : 0,
  );

  useEffect(() => {
    if (autoStartAt && remaining === 0 && firedForRef.current !== autoStartAt) {
      firedForRef.current = autoStartAt;
      onExpireRef.current?.();
    }
  }, [autoStartAt, remaining]);

  return remaining;
}
