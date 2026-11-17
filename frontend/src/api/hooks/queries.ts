import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { useAuth } from "../auth-provider";
import { useLogandaimApi } from "../provider";
import { logandaimKeys } from "../query-keys";

export function useHealthQuery() {
  const { api } = useLogandaimApi();
  return useQuery({
    queryKey: logandaimKeys.health(),
    queryFn: () => api.health(),
  });
}

export function useBenutzerQuery() {
  const { api } = useLogandaimApi();
  const phase = useAuth();
  return useQuery({
    queryKey: logandaimKeys.benutzer(),
    queryFn: () => api.benutzerAbrufen(),
    enabled: phase.status === "authenticated",
    retry: false,
  });
}

export function useDecksQuery() {
  const { api } = useLogandaimApi();
  const phase = useAuth();
  return useQuery({
    queryKey: logandaimKeys.decks(),
    queryFn: () => api.decksList(),
    enabled: phase.status === "authenticated",
    staleTime: 10 * 60_000,
  });
}

export function useGamesQuery() {
  const { api } = useLogandaimApi();
  const phase = useAuth();
  return useQuery({
    queryKey: logandaimKeys.games.all(),
    queryFn: () => api.gamesList(),
    enabled: phase.status === "authenticated",
    refetchInterval: 5000,
    retry: false,
  });
}

export function useGameByCodeQuery(code: string | null) {
  const { api } = useLogandaimApi();
  const phase = useAuth();
  return useQuery({
    queryKey: logandaimKeys.games.byCode(code ?? ""),
    queryFn: () => api.gameByInviteCode(code ?? ""),
    enabled: phase.status === "authenticated" && Boolean(code?.trim()),
    retry: false,
  });
}

export function useGameDetailQuery(gameId: string | null) {
  const { api } = useLogandaimApi();
  const phase = useAuth();
  return useQuery({
    queryKey: logandaimKeys.games.detail(gameId ?? ""),
    queryFn: () => api.gameDetail(gameId ?? ""),
    enabled: phase.status === "authenticated" && Boolean(gameId),
    refetchInterval: (query) =>
      query.state.data?.status === "Completed" ? false : 5000,
    retry: false,
  });
}

export function useUserProfileQuery(userId: string | null) {
  const { api } = useLogandaimApi();
  const phase = useAuth();
  return useQuery({
    queryKey: logandaimKeys.profiles.detail(userId ?? ""),
    queryFn: () => api.profileGet(userId ?? ""),
    enabled: phase.status === "authenticated" && Boolean(userId),
    retry: false,
  });
}

export function useUserGalleryQuery(userId: string | null) {
  const { api } = useLogandaimApi();
  const phase = useAuth();
  return useQuery({
    queryKey: logandaimKeys.profiles.gallery(userId ?? ""),
    queryFn: () => api.userGallery(userId ?? ""),
    enabled: phase.status === "authenticated" && Boolean(userId),
    retry: false,
  });
}

/** Fetch an authenticated media blob and return a managed object URL. */
export function useMediaObjectUrl(storageKey: string | null | undefined) {
  const { api } = useLogandaimApi();
  const phase = useAuth();
  const query = useQuery({
    queryKey: logandaimKeys.media(storageKey ?? ""),
    queryFn: () => api.fetchMediaBlob(storageKey ?? ""),
    enabled: phase.status === "authenticated" && Boolean(storageKey),
    staleTime: 10 * 60_000,
    retry: false,
  });

  const [objectUrl, setObjectUrl] = useState<string | null>(null);
  useEffect(() => {
    if (!query.data) {
      setObjectUrl(null);
      return;
    }
    const url = URL.createObjectURL(query.data);
    setObjectUrl(url);
    return () => {
      URL.revokeObjectURL(url);
    };
  }, [query.data]);

  return {
    url: objectUrl,
    isLoading: query.isLoading,
    isError: query.isError,
  };
}
