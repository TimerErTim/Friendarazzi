import {
  CompletionReason,
  type components,
  GameStatus,
  NotificationKind,
  PromptSource,
  SuperlativeKind,
  VisibilityMode,
} from "./schema";

export {
  CompletionReason,
  GameStatus,
  NotificationKind,
  PromptSource,
  SuperlativeKind,
  VisibilityMode,
};

/** Transport-neutral API surface for Friendarazzi. */
export interface FriendarazziApi {
  health(): Promise<string>;

  // ---------------------------------------------------------------- Decks
  decksList(): Promise<StarterDeck[]>;

  // ---------------------------------------------------------------- Games
  gamesList(): Promise<GameSummary[]>;
  gameByInviteCode(inviteCode: string): Promise<GameInvitePreview>;
  gameCreate(input: CreateGameInput): Promise<GameDetail>;
  gameJoin(inviteCode: string): Promise<GameDetail>;
  gameDetail(gameId: string): Promise<GameDetail>;
  gameUpdateSettings(
    gameId: string,
    input: UpdateGameSettingsInput,
  ): Promise<GameDetail>;
  gameStartDrafting(gameId: string): Promise<GameDetail>;
  gameSavePrompts(
    gameId: string,
    prompts: DraftPromptRequestItem[],
  ): Promise<GameDetail>;
  gameLockPrompts(gameId: string): Promise<GameDetail>;
  gameUnlockPrompts(gameId: string): Promise<GameDetail>;
  gameHostFillDrafts(
    gameId: string,
    startImmediately: boolean,
  ): Promise<GameDetail>;
  gameSetDraftDeadline(
    gameId: string,
    draftDeadlineAt: string | null,
  ): Promise<GameDetail>;
  gameSetWatch(gameId: string, watching: boolean): Promise<GameDetail>;
  gameLeave(gameId: string): Promise<void>;
  gameKickPlayer(gameId: string, userId: string): Promise<GameDetail>;
  gameTransferHost(gameId: string, newHostUserId: string): Promise<GameDetail>;
  gameAbandon(gameId: string): Promise<GameDetail>;

  // ---------------------------------------------------------------- Notifications
  notificationsList(): Promise<NotificationsInbox>;
  notificationMarkRead(notificationId: string): Promise<void>;
  notificationsMarkAllRead(): Promise<void>;
  notificationPreferencesGet(): Promise<NotificationPreferences>;
  notificationPreferencesUpdate(
    input: UpdateNotificationPreferencesInput,
  ): Promise<NotificationPreferences>;
  notificationUnsubscribe(token: string): Promise<UnsubscribeOutcome>;

  // ---------------------------------------------------------------- Captures & Media
  captureSubmit(
    gameId: string,
    tileId: string,
    file: File,
  ): Promise<CaptureOutcome>;
  captureRetract(gameId: string, captureId: string): Promise<GameDetail>;
  fetchMediaBlob(storageKey: string): Promise<Blob>;

  // ---------------------------------------------------------------- Profiles & Galleries
  profileGet(userId: string): Promise<UserProfile>;
  userGallery(userId: string): Promise<UserGallery>;
  profileUpdate(input: UpdateProfileInput): Promise<UserProfile>;
  profileUploadAvatar(file: File): Promise<UserProfile>;

  // ---------------------------------------------------------------- Konto
  registrieren(input: RegistrierungInput): Promise<void>;
  emailBestaetigen(token: string): Promise<void>;
  emailBestaetigungErneutSenden(): Promise<void>;
  anmelden(eMailAdresse: string, passwort: string): Promise<void>;
  passwortVergessen(eMailAdresse: string): Promise<void>;
  passwortWiederherstellen(token: string, neuesPasswort: string): Promise<void>;
  benutzerAbrufen(): Promise<AuthenticatedBenutzer>;
  benutzerLoeschen(passwort: string, bestaetigt: boolean): Promise<void>;
}

export type AuthPhase =
  | { status: "checking" }
  | { status: "anonymous" }
  | { status: "authenticated" }
  | { status: "unavailable"; error: unknown };

export interface AuthSession {
  restore(): Promise<void>;
  logout(): Promise<void>;
  getSnapshot(): AuthPhase;
  subscribe(listener: () => void): () => void;
}

export type AuthenticatedBenutzer =
  components["schemas"]["AuthenticatedBenutzer"];
export type PlayerBadge = components["schemas"]["PlayerBadge"];
export type UserProfile = components["schemas"]["UserProfile"];
export type StarterDeck = components["schemas"]["StarterDeck"];
export type DraftTargetQuota = components["schemas"]["DraftTargetQuota"];
export type PromptSubmissionView =
  components["schemas"]["PromptSubmissionView"];
export type BoardTileView = components["schemas"]["BoardTileView"];
export type BoardView = components["schemas"]["BoardView"];
export type CaptureFeedItem = components["schemas"]["CaptureFeedItem"];
export type SuperlativeView = components["schemas"]["SuperlativeView"];
export type GameSummary = components["schemas"]["GameSummary"];
export type GameInvitePreview = components["schemas"]["GameInvitePreview"];
export type GameDetail = components["schemas"]["GameDetail"];
export type RegistrierungInput = components["schemas"]["RegistrierungRequest"];
export type GridCoordinate = components["schemas"]["GridCoordinate"];
export type CaptureOutcome = components["schemas"]["CaptureOutcome"];
export type GalleryGameGroup = components["schemas"]["GalleryGameGroup"];
export type UserGallery = components["schemas"]["UserGallery"];

export type CreateGameInput = components["schemas"]["CreateGameRequest"];
export type UpdateGameSettingsInput =
  components["schemas"]["UpdateGameSettingsRequest"];
export type DraftPromptRequestItem =
  components["schemas"]["DraftPromptRequestItem"];
export type UpdateProfileInput = components["schemas"]["UpdateProfileRequest"];
export type NotificationView = components["schemas"]["NotificationView"];
export type NotificationsInbox = components["schemas"]["NotificationsInbox"];
export type NotificationPreferences =
  components["schemas"]["NotificationPreferences"];
export type UpdateNotificationPreferencesInput =
  components["schemas"]["UpdateNotificationPreferencesRequest"];
export type UnsubscribeOutcome = components["schemas"]["UnsubscribeOutcome"];
