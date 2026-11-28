import { useMutation, useQueryClient } from "@tanstack/react-query";

import type {
  CreateGameInput,
  DraftPromptRequestItem,
  GameDetail,
  NotificationsInbox,
  UpdateGameSettingsInput,
  UpdateNotificationPreferencesInput,
  UpdateProfileInput,
} from "../friendarazzi-api";
import { useFriendarazziApi } from "../provider";
import { friendarazziKeys } from "../query-keys";

const SITE_HANDLED = { meta: { handlesError: true } } as const;

export function useFriendarazziMutations() {
  const { api } = useFriendarazziApi();
  const queryClient = useQueryClient();

  const invalidateGame = (gameId: string) => {
    void queryClient.invalidateQueries({
      queryKey: friendarazziKeys.games.detail(gameId),
    });
    void queryClient.invalidateQueries({
      queryKey: friendarazziKeys.games.all(),
    });
  };

  const syncGameDetail = (game: GameDetail) => {
    queryClient.setQueryData(friendarazziKeys.games.detail(game.id), game);
    void queryClient.invalidateQueries({
      queryKey: friendarazziKeys.games.all(),
    });
  };

  return {
    createGame: useMutation({
      mutationFn: (input: CreateGameInput) => api.gameCreate(input),
      ...SITE_HANDLED,
      onSuccess: syncGameDetail,
    }),
    joinGame: useMutation({
      mutationFn: (inviteCode: string) => api.gameJoin(inviteCode),
      ...SITE_HANDLED,
      onSuccess: syncGameDetail,
    }),
    updateSettings: useMutation({
      mutationFn: (vars: { gameId: string; input: UpdateGameSettingsInput }) =>
        api.gameUpdateSettings(vars.gameId, vars.input),
      ...SITE_HANDLED,
      onSuccess: syncGameDetail,
    }),
    startDrafting: useMutation({
      mutationFn: (gameId: string) => api.gameStartDrafting(gameId),
      ...SITE_HANDLED,
      onSuccess: syncGameDetail,
    }),
    savePrompts: useMutation({
      mutationFn: (vars: {
        gameId: string;
        prompts: DraftPromptRequestItem[];
      }) => api.gameSavePrompts(vars.gameId, vars.prompts),
      ...SITE_HANDLED,
      onSuccess: syncGameDetail,
    }),
    lockPrompts: useMutation({
      mutationFn: (gameId: string) => api.gameLockPrompts(gameId),
      ...SITE_HANDLED,
      onSuccess: syncGameDetail,
    }),
    unlockPrompts: useMutation({
      mutationFn: (gameId: string) => api.gameUnlockPrompts(gameId),
      ...SITE_HANDLED,
      onSuccess: syncGameDetail,
    }),
    hostFillDrafts: useMutation({
      mutationFn: (vars: { gameId: string; startImmediately: boolean }) =>
        api.gameHostFillDrafts(vars.gameId, vars.startImmediately),
      ...SITE_HANDLED,
      onSuccess: syncGameDetail,
    }),
    setDraftDeadline: useMutation({
      mutationFn: (vars: { gameId: string; draftDeadlineAt: string | null }) =>
        api.gameSetDraftDeadline(vars.gameId, vars.draftDeadlineAt),
      ...SITE_HANDLED,
      onSuccess: syncGameDetail,
    }),
    setGameWatch: useMutation({
      mutationFn: (vars: { gameId: string; watching: boolean }) =>
        api.gameSetWatch(vars.gameId, vars.watching),
      ...SITE_HANDLED,
      onMutate: async (vars) => {
        const key = friendarazziKeys.games.detail(vars.gameId);
        await queryClient.cancelQueries({ queryKey: key });
        const previous = queryClient.getQueryData<GameDetail>(key);
        if (previous) {
          queryClient.setQueryData<GameDetail>(key, {
            ...previous,
            watching: vars.watching,
          });
        }
        return { previous };
      },
      onError: (_err, vars, context) => {
        if (context?.previous) {
          queryClient.setQueryData(
            friendarazziKeys.games.detail(vars.gameId),
            context.previous,
          );
        }
      },
      onSuccess: syncGameDetail,
    }),
    markNotificationRead: useMutation({
      mutationFn: (notificationId: string) =>
        api.notificationMarkRead(notificationId),
      ...SITE_HANDLED,
      onMutate: async (notificationId) => {
        const key = friendarazziKeys.notifications.inbox();
        await queryClient.cancelQueries({ queryKey: key });
        const previous = queryClient.getQueryData<NotificationsInbox>(key);
        if (previous) {
          let decremented = 0;
          const nowIso = new Date().toISOString();
          const notifications = previous.notifications.map((item) => {
            if (item.id === notificationId && !item.readAt) {
              decremented = 1;
              return { ...item, readAt: nowIso };
            }
            return item;
          });
          queryClient.setQueryData<NotificationsInbox>(key, {
            unreadCount: Math.max(0, previous.unreadCount - decremented),
            notifications,
          });
        }
        return { previous };
      },
      onError: (_err, _vars, context) => {
        if (context?.previous) {
          queryClient.setQueryData(
            friendarazziKeys.notifications.inbox(),
            context.previous,
          );
        }
      },
      onSettled: () => {
        void queryClient.invalidateQueries({
          queryKey: friendarazziKeys.notifications.inbox(),
        });
      },
    }),
    markAllNotificationsRead: useMutation({
      mutationFn: () => api.notificationsMarkAllRead(),
      ...SITE_HANDLED,
      onMutate: async () => {
        const key = friendarazziKeys.notifications.inbox();
        await queryClient.cancelQueries({ queryKey: key });
        const previous = queryClient.getQueryData<NotificationsInbox>(key);
        if (previous) {
          const nowIso = new Date().toISOString();
          queryClient.setQueryData<NotificationsInbox>(key, {
            unreadCount: 0,
            notifications: previous.notifications.map((item) =>
              item.readAt ? item : { ...item, readAt: nowIso },
            ),
          });
        }
        return { previous };
      },
      onError: (_err, _vars, context) => {
        if (context?.previous) {
          queryClient.setQueryData(
            friendarazziKeys.notifications.inbox(),
            context.previous,
          );
        }
      },
      onSettled: () => {
        void queryClient.invalidateQueries({
          queryKey: friendarazziKeys.notifications.inbox(),
        });
      },
    }),
    updateNotificationPreferences: useMutation({
      mutationFn: (input: UpdateNotificationPreferencesInput) =>
        api.notificationPreferencesUpdate(input),
      ...SITE_HANDLED,
      onSuccess: (updated) => {
        queryClient.setQueryData(
          friendarazziKeys.notifications.preferences(),
          updated,
        );
      },
    }),
    unsubscribeNotification: useMutation({
      mutationFn: (token: string) => api.notificationUnsubscribe(token),
      ...SITE_HANDLED,
      onSuccess: () => {
        void queryClient.invalidateQueries({
          queryKey: friendarazziKeys.notifications.preferences(),
        });
      },
    }),
    submitCapture: useMutation({
      mutationFn: (vars: { gameId: string; tileId: string; file: File }) =>
        api.captureSubmit(vars.gameId, vars.tileId, vars.file),
      ...SITE_HANDLED,
      onSuccess: (outcome, vars) => {
        invalidateGame(vars.gameId);
        if (outcome.winningLine) {
          void queryClient.invalidateQueries({
            queryKey: friendarazziKeys.benutzer(),
          });
          void queryClient.invalidateQueries({
            queryKey: friendarazziKeys.profiles.all(),
          });
        }
      },
    }),
    retractCapture: useMutation({
      mutationFn: (vars: { gameId: string; captureId: string }) =>
        api.captureRetract(vars.gameId, vars.captureId),
      ...SITE_HANDLED,
      onSuccess: (detail) => {
        syncGameDetail(detail);
        void queryClient.invalidateQueries({
          queryKey: friendarazziKeys.profiles.all(),
        });
      },
    }),
    updateProfile: useMutation({
      mutationFn: (input: UpdateProfileInput) => api.profileUpdate(input),
      ...SITE_HANDLED,
      onSuccess: (profile) => {
        void queryClient.invalidateQueries({
          queryKey: friendarazziKeys.benutzer(),
        });
        void queryClient.invalidateQueries({
          queryKey: friendarazziKeys.profiles.detail(profile.id),
        });
        void queryClient.invalidateQueries({
          queryKey: friendarazziKeys.profiles.gallery(profile.id),
        });
      },
    }),
    uploadAvatar: useMutation({
      mutationFn: (file: File) => api.profileUploadAvatar(file),
      ...SITE_HANDLED,
      onSuccess: (profile) => {
        void queryClient.invalidateQueries({
          queryKey: friendarazziKeys.benutzer(),
        });
        void queryClient.invalidateQueries({
          queryKey: friendarazziKeys.profiles.detail(profile.id),
        });
        void queryClient.invalidateQueries({
          queryKey: friendarazziKeys.profiles.gallery(profile.id),
        });
      },
    }),
  };
}

export function useAuthMutations() {
  const { api, authSession } = useFriendarazziApi();
  const queryClient = useQueryClient();
  const loggedIn = () =>
    queryClient.invalidateQueries({ queryKey: friendarazziKeys.benutzer() });
  const clear = () => {
    queryClient.removeQueries({ queryKey: friendarazziKeys.benutzer() });
    queryClient.removeQueries({ queryKey: friendarazziKeys.games.all() });
    queryClient.removeQueries({ queryKey: friendarazziKeys.profiles.all() });
    queryClient.removeQueries({
      queryKey: friendarazziKeys.notifications.all(),
    });
  };

  return {
    login: useMutation({
      mutationFn: ({
        eMailAdresse,
        passwort,
      }: {
        eMailAdresse: string;
        passwort: string;
      }) => api.anmelden(eMailAdresse, passwort),
      ...SITE_HANDLED,
      onSuccess: loggedIn,
    }),
    registerStart: useMutation({
      mutationFn: api.registrierungStarten.bind(api),
      ...SITE_HANDLED,
    }),
    registerFinish: useMutation({
      mutationFn: ({ token, passwort }: { token: string; passwort: string }) =>
        api.registrierungAbschliessen(token, passwort),
      ...SITE_HANDLED,
      onSuccess: loggedIn,
    }),
    forgotPassword: useMutation({
      mutationFn: api.passwortVergessen.bind(api),
      ...SITE_HANDLED,
    }),
    resetPassword: useMutation({
      mutationFn: ({
        token,
        neuesPasswort,
      }: {
        token: string;
        neuesPasswort: string;
      }) => api.passwortWiederherstellen(token, neuesPasswort),
      ...SITE_HANDLED,
      onSuccess: clear,
    }),
    logout: useMutation({
      mutationFn: () => authSession.logout(),
      onSettled: clear,
    }),
    deleteAccount: useMutation({
      mutationFn: ({
        passwort,
        bestaetigt,
      }: {
        passwort: string;
        bestaetigt: boolean;
      }) => api.benutzerLoeschen(passwort, bestaetigt),
      ...SITE_HANDLED,
      onSettled: clear,
    }),
  };
}
