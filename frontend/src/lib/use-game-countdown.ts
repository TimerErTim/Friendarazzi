"use client";

import { useEffect, useRef, useState } from "react";

import { remainingCountdownSeconds } from "@/lib/game-overview";

export function useGameCountdown(
  draftClosesAt: string | null | undefined,
  serverNow: string | null | undefined,
  dataUpdatedAt: number,
  onExpire?: () => void,
): number | null {
  const [nowMs, setNowMs] = useState(() => Date.now());
  const firedForRef = useRef<string | null>(null);
  const onExpireRef = useRef(onExpire);
  onExpireRef.current = onExpire;

  useEffect(() => {
    if (!draftClosesAt) {
      firedForRef.current = null;
      return;
    }
    setNowMs(Date.now());
    const timer = setInterval(() => {
      setNowMs(Date.now());
    }, 250);
    return () => clearInterval(timer);
  }, [draftClosesAt]);

  const remaining = remainingCountdownSeconds(
    draftClosesAt,
    serverNow,
    dataUpdatedAt > 0 ? nowMs - dataUpdatedAt : 0,
  );

  useEffect(() => {
    if (
      draftClosesAt &&
      remaining === 0 &&
      firedForRef.current !== draftClosesAt
    ) {
      firedForRef.current = draftClosesAt;
      onExpireRef.current?.();
    }
  }, [draftClosesAt, remaining]);

  return remaining;
}
