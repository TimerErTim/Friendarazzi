"use client";

import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { createContext, type ReactNode, useContext, useMemo } from "react";

import { config } from "@/lib/config";
import type { AuthSession, FriendarazziApi } from "./friendarazzi-api";
import { createFriendarazziApi } from "./index";

type FriendarazziApiContextValue = {
  api: FriendarazziApi;
  authSession: AuthSession;
  apiBaseUrl: string;
  dev: boolean;
};

const FriendarazziApiContext =
  createContext<FriendarazziApiContextValue | null>(null);

/**
 * Creates the API instance once per client and exposes it. Configuration is static;
 * authentication bootstrap is managed by `AuthProvider` above the app shell.
 */
export function FriendarazziApiProvider({ children }: { children: ReactNode }) {
  // One transport instance per client; `config` never changes at runtime.
  const value = useMemo<FriendarazziApiContextValue>(
    () => ({
      ...createFriendarazziApi(),
      apiBaseUrl: config.apiBaseUrl,
      dev: config.dev,
    }),
    [],
  );

  return (
    <FriendarazziApiContext.Provider value={value}>
      {children}
      {value.dev ? <ReactQueryDevtools initialIsOpen={false} /> : null}
    </FriendarazziApiContext.Provider>
  );
}

export function useFriendarazziApi(): FriendarazziApiContextValue {
  const context = useContext(FriendarazziApiContext);
  if (!context) {
    throw new Error(
      "useFriendarazziApi must be used within FriendarazziApiProvider",
    );
  }
  return context;
}
