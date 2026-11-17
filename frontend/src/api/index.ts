import { config } from "@/lib/config";
import type { AuthSession, FriendarazziApi } from "./friendarazzi-api";
import { HttpFriendarazziApi } from "./http-client";

/**
 * The one place that picks a transport.
 *
 * `FriendarazziApi` stays an interface on purpose: swapping this single line for an
 * embedded/wasm implementation of the core domain is the intended future move.
 */
export function createFriendarazziApi(): {
  api: FriendarazziApi;
  authSession: AuthSession;
} {
  const transport = new HttpFriendarazziApi(config.apiBaseUrl);
  return { api: transport, authSession: transport };
}
