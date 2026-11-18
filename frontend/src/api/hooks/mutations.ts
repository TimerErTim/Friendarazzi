import { useMutation, useQueryClient } from "@tanstack/react-query";

import type {
  CreateGameInput,
  DraftPromptRequestItem,
  UpdateGameSettingsInput,
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

  return {
    createGame: useMutation({
      mutationFn: (input: CreateGameInput) => api.gameCreate(input),
      ...SITE_HANDLED,
      onSuccess: (game) => invalidateGame(game.id),
    }),
    joinGame: useMutation({
      mutationFn: (inviteCode: string) => api.gameJoin(inviteCode),
      ...SITE_HANDLED,
      onSuccess: (game) => invalidateGame(game.id),
    }),
    updateSettings: useMutation({
      mutationFn: (vars: { gameId: string; input: UpdateGameSettingsInput }) =>
        api.gameUpdateSettings(vars.gameId, vars.input),
      ...SITE_HANDLED,
      onSuccess: (game) => invalidateGame(game.id),
    }),
    startDrafting: useMutation({
      mutationFn: (gameId: string) => api.gameStartDrafting(gameId),
      ...SITE_HANDLED,
      onSuccess: (game) => invalidateGame(game.id),
    }),
    savePrompts: useMutation({
      mutationFn: (vars: {
        gameId: string;
        prompts: DraftPromptRequestItem[];
      }) => api.gameSavePrompts(vars.gameId, vars.prompts),
      ...SITE_HANDLED,
      onSuccess: (game) => invalidateGame(game.id),
    }),
    lockPrompts: useMutation({
      mutationFn: (gameId: string) => api.gameLockPrompts(gameId),
      ...SITE_HANDLED,
      onSuccess: (game) => invalidateGame(game.id),
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
