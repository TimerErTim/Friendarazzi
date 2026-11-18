import createClient from "openapi-fetch";

import { ApiError } from "./errors";
import type {
  AuthenticatedBenutzer,
  AuthPhase,
  AuthSession,
  CaptureOutcome,
  CreateGameInput,
  DraftPromptRequestItem,
  FriendarazziApi,
  GameDetail,
  GameSummary,
  StarterDeck,
  UpdateGameSettingsInput,
  UpdateProfileInput,
  UserGallery,
  UserProfile,
} from "./friendarazzi-api";
import type { paths } from "./schema";

type JsonClient = ReturnType<typeof createClient<paths>>;

async function unwrap<T>(result: {
  data?: T;
  error?: unknown;
  response: Response;
}): Promise<T> {
  if (result.error !== undefined) {
    throw ApiError.fromUnknown(result.error, result.response.status);
  }
  return result.data as T;
}

/** HTTP implementation of the transport-neutral API. */
export class HttpFriendarazziApi implements FriendarazziApi, AuthSession {
  private readonly client: JsonClient;
  private renewal: Promise<void> | null = null;
  private readonly baseUrl: string;
  private accessToken: string | null = null;
  private authRevision = 0;
  private phase: AuthPhase = Object.freeze({ status: "checking" });
  private readonly listeners = new Set<() => void>();

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl.replace(/\/$/, "");
    this.client = createClient<paths>({
      baseUrl: this.baseUrl,
      fetch: (request) =>
        fetch(new Request(request, { credentials: "include" })),
    });
  }

  private setAccessToken(token: string | null): void {
    this.authRevision += 1;
    this.accessToken = token;
    this.setPhase(
      token ? { status: "authenticated" } : { status: "anonymous" },
    );
  }

  private setPhase(phase: AuthPhase): void {
    if (this.phase.status === phase.status) return;
    this.phase = Object.freeze(phase);
    for (const listener of this.listeners) listener();
  }

  getSnapshot = (): AuthPhase => this.phase;

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  async restore(): Promise<void> {
    this.setPhase({ status: "checking" });
    try {
      await this.renewSession();
    } catch (error) {
      if (error instanceof ApiError && error.status === 401) {
        this.setAccessToken(null);
        return;
      }
      this.setPhase({ status: "unavailable", error });
      throw error;
    }
  }

  async logout(): Promise<void> {
    const headers = this.authorizationHeaders();
    this.setAccessToken(null);
    try {
      await unwrap(await this.client.POST("/session/abmelden", { headers }));
    } finally {
      this.setAccessToken(null);
    }
  }

  private async authenticated<T>(
    request: () => Promise<{
      data?: T;
      error?: unknown;
      response: Response;
    }>,
  ): Promise<T> {
    const token = this.accessToken;
    const result = await request();
    if (result.response.status !== 401 || !token) {
      return unwrap<T>(result);
    }
    if (this.accessToken !== token) return unwrap<T>(await request());

    try {
      await this.renewSession();
    } catch (error) {
      if (error instanceof ApiError && error.status === 401) {
        this.setAccessToken(null);
        return unwrap<T>(result);
      }
      throw error;
    }
    if (this.accessToken === token) return unwrap<T>(result);
    return unwrap<T>(await request());
  }

  private async renewSession(): Promise<void> {
    if (!this.renewal) {
      this.renewal = this.performRenewal().finally(() => {
        this.renewal = null;
      });
    }
    return this.renewal;
  }

  private async performRenewal(): Promise<void> {
    const revision = this.authRevision;
    const renewed = await fetch(`${this.baseUrl}/session/erneuern`, {
      method: "POST",
      credentials: "include",
    });
    if (!renewed.ok) {
      let error: unknown = `Session renewal failed (${renewed.status})`;
      try {
        error = await renewed.json();
      } catch {
        // Keep status-bearing message
      }
      throw ApiError.fromUnknown(error, renewed.status);
    }
    const content = (await renewed.json()) as { accessToken: string };
    if (revision !== this.authRevision) return;
    this.setAccessToken(content.accessToken);
  }

  async health(): Promise<string> {
    return unwrap(await this.client.GET("/health"));
  }

  private authorizationHeaders() {
    return {
      Authorization: this.accessToken
        ? `Bearer ${this.accessToken}`
        : undefined,
    };
  }

  private async postMultipart<T>(path: string, file: File): Promise<T> {
    return this.authenticated<T>(async () => {
      const form = new FormData();
      form.append("file", file);
      const headers: Record<string, string> = {};
      if (this.accessToken) {
        headers.Authorization = `Bearer ${this.accessToken}`;
      }
      const response = await fetch(`${this.baseUrl}${path}`, {
        method: "POST",
        credentials: "include",
        headers,
        body: form,
      });
      if (!response.ok) {
        let error: unknown = `Upload failed (${response.status})`;
        try {
          error = await response.json();
        } catch {
          // Ignore non-JSON body
        }
        return { error, response };
      }
      const data = (await response.json()) as T;
      return { data, response };
    });
  }

  // ---------------------------------------------------------------- Decks

  async decksList(): Promise<StarterDeck[]> {
    return this.authenticated(() =>
      this.client.GET("/decks", { headers: this.authorizationHeaders() }),
    );
  }

  // ---------------------------------------------------------------- Games

  async gamesList(): Promise<GameSummary[]> {
    return this.authenticated(() =>
      this.client.GET("/games", { headers: this.authorizationHeaders() }),
    );
  }

  async gameByInviteCode(inviteCode: string): Promise<GameSummary> {
    return this.authenticated(() =>
      this.client.GET("/games/by-code/{invite_code}", {
        params: { path: { invite_code: inviteCode } },
        headers: this.authorizationHeaders(),
      }),
    );
  }

  async gameCreate(input: CreateGameInput): Promise<GameDetail> {
    return this.authenticated(() =>
      this.client.POST("/games", {
        body: input,
        headers: this.authorizationHeaders(),
      }),
    );
  }

  async gameJoin(inviteCode: string): Promise<GameDetail> {
    return this.authenticated(() =>
      this.client.POST("/games/join", {
        body: { inviteCode },
        headers: this.authorizationHeaders(),
      }),
    );
  }

  async gameDetail(gameId: string): Promise<GameDetail> {
    return this.authenticated(() =>
      this.client.GET("/games/{game_id}", {
        params: { path: { game_id: gameId } },
        headers: this.authorizationHeaders(),
      }),
    );
  }

  async gameUpdateSettings(
    gameId: string,
    input: UpdateGameSettingsInput,
  ): Promise<GameDetail> {
    return this.authenticated(() =>
      this.client.PUT("/games/{game_id}/settings", {
        params: { path: { game_id: gameId } },
        body: input,
        headers: this.authorizationHeaders(),
      }),
    );
  }

  async gameStartDrafting(gameId: string): Promise<GameDetail> {
    return this.authenticated(() =>
      this.client.POST("/games/{game_id}/start-drafting", {
        params: { path: { game_id: gameId } },
        headers: this.authorizationHeaders(),
      }),
    );
  }

  async gameSavePrompts(
    gameId: string,
    prompts: DraftPromptRequestItem[],
  ): Promise<GameDetail> {
    return this.authenticated(() =>
      this.client.PUT("/games/{game_id}/prompts", {
        params: { path: { game_id: gameId } },
        body: { prompts },
        headers: this.authorizationHeaders(),
      }),
    );
  }

  async gameLockPrompts(gameId: string): Promise<GameDetail> {
    return this.authenticated(() =>
      this.client.POST("/games/{game_id}/prompts/lock", {
        params: { path: { game_id: gameId } },
        headers: this.authorizationHeaders(),
      }),
    );
  }

  // ---------------------------------------------------------------- Captures & Media

  async captureSubmit(
    gameId: string,
    tileId: string,
    file: File,
  ): Promise<CaptureOutcome> {
    return this.postMultipart<CaptureOutcome>(
      `/games/${encodeURIComponent(gameId)}/tiles/${encodeURIComponent(tileId)}/capture`,
      file,
    );
  }

  async fetchMediaBlob(storageKey: string): Promise<Blob> {
    return this.authenticated<Blob>(async () => {
      const headers: Record<string, string> = {};
      if (this.accessToken) {
        headers.Authorization = `Bearer ${this.accessToken}`;
      }
      const response = await fetch(
        `${this.baseUrl}/media/${encodeURIComponent(storageKey)}`,
        {
          method: "GET",
          credentials: "include",
          headers,
        },
      );
      if (!response.ok) {
        let error: unknown = `Media fetch failed (${response.status})`;
        try {
          error = await response.json();
        } catch {
          // Ignore non-JSON body
        }
        return { error, response };
      }
      const data = await response.blob();
      return { data, response };
    });
  }

  // ---------------------------------------------------------------- Profiles & Galleries

  async profileGet(userId: string): Promise<UserProfile> {
    return this.authenticated(() =>
      this.client.GET("/users/{user_id}", {
        params: { path: { user_id: userId } },
        headers: this.authorizationHeaders(),
      }),
    );
  }

  async userGallery(userId: string): Promise<UserGallery> {
    return this.authenticated(() =>
      this.client.GET("/users/{user_id}/gallery", {
        params: { path: { user_id: userId } },
        headers: this.authorizationHeaders(),
      }),
    );
  }

  async profileUpdate(input: UpdateProfileInput): Promise<UserProfile> {
    return this.authenticated(() =>
      this.client.PUT("/profil", {
        body: { ...input, equippedTitle: input.equippedTitle ?? undefined },
        headers: this.authorizationHeaders(),
      }),
    );
  }

  async profileUploadAvatar(file: File): Promise<UserProfile> {
    return this.postMultipart<UserProfile>("/profil/avatar", file);
  }

  // ---------------------------------------------------------------- Konto

  async registrierungStarten(eMailAdresse: string): Promise<void> {
    await unwrap(
      await this.client.POST("/registrierung/start", {
        body: { eMailAdresse },
      }),
    );
  }

  async registrierungAbschliessen(
    token: string,
    passwort: string,
  ): Promise<void> {
    const result = await unwrap(
      await this.client.POST("/registrierung/abschluss", {
        body: { token, passwort },
      }),
    );
    this.setAccessToken(result.accessToken);
  }

  async anmelden(eMailAdresse: string, passwort: string): Promise<void> {
    const result = await unwrap(
      await this.client.POST("/anmeldung", {
        body: { eMailAdresse, passwort },
      }),
    );
    this.setAccessToken(result.accessToken);
  }

  async passwortVergessen(eMailAdresse: string): Promise<void> {
    await unwrap(
      await this.client.POST("/passwort/vergessen", { body: { eMailAdresse } }),
    );
  }

  async passwortWiederherstellen(
    token: string,
    neuesPasswort: string,
  ): Promise<void> {
    await unwrap(
      await this.client.POST("/passwort/wiederherstellen", {
        body: { token, neuesPasswort },
      }),
    );
    this.setAccessToken(null);
  }

  async benutzerAbrufen(): Promise<AuthenticatedBenutzer> {
    return this.authenticated(() =>
      this.client.GET("/benutzer", {
        headers: this.authorizationHeaders(),
      }),
    );
  }

  async benutzerLoeschen(passwort: string, bestaetigt: boolean): Promise<void> {
    await this.authenticated(() =>
      this.client.POST("/benutzer/loeschung", {
        body: { passwort, bestaetigt },
        headers: this.authorizationHeaders(),
      }),
    );
    this.setAccessToken(null);
  }
}
