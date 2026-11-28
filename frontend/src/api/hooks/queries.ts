import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { useAuth } from "../auth-provider";
import { useFriendarazziApi } from "../provider";
import { friendarazziKeys } from "../query-keys";

function useDocumentVisible(): boolean {
  const [visible, setVisible] = useState(() =>
    typeof document === "undefined"
      ? true
      : document.visibilityState !== "hidden",
  );

  useEffect(() => {
    if (typeof document === "undefined") return;
    const onChange = () => {
      setVisible(document.visibilityState !== "hidden");
    };
    document.addEventListener("visibilitychange", onChange);
    return () => document.removeEventListener("visibilitychange", onChange);
  }, []);

  return visible;
}

export function useHealthQuery() {
  const { api } = useFriendarazziApi();
  return useQuery({
    queryKey: friendarazziKeys.health(),
    queryFn: () => api.health(),
  });
}

export function useAuthProvidersQuery() {
  const { api } = useFriendarazziApi();
  return useQuery({
    queryKey: friendarazziKeys.authProviders(),
    queryFn: () => api.authProviders(),
    staleTime: 5 * 60_000,
    retry: false,
  });
}

export function useLoginMethodsQuery() {
  const { api } = useFriendarazziApi();
  const phase = useAuth();
  return useQuery({
    queryKey: friendarazziKeys.loginMethods(),
    queryFn: () => api.loginMethods(),
    enabled: phase.status === "authenticated",
    retry: false,
  });
}

export function useBenutzerQuery() {
  const { api } = useFriendarazziApi();
  const phase = useAuth();
  return useQuery({
    queryKey: friendarazziKeys.benutzer(),
    queryFn: () => api.benutzerAbrufen(),
    enabled: phase.status === "authenticated",
    retry: false,
  });
}

export function useDecksQuery() {
  const { api } = useFriendarazziApi();
  const phase = useAuth();
  return useQuery({
    queryKey: friendarazziKeys.decks(),
    queryFn: () => api.decksList(),
    enabled: phase.status === "authenticated",
    staleTime: 10 * 60_000,
  });
}

export function useGamesQuery() {
  const { api } = useFriendarazziApi();
  const phase = useAuth();
  const visible = useDocumentVisible();
  return useQuery({
    queryKey: friendarazziKeys.games.all(),
    queryFn: () => api.gamesList(),
    enabled: phase.status === "authenticated",
    refetchInterval: visible ? 5000 : false,
    retry: false,
  });
}

export function useGameByCodeQuery(code: string | null) {
  const { api } = useFriendarazziApi();
  return useQuery({
    queryKey: friendarazziKeys.games.byCode(code ?? ""),
    queryFn: () => api.gameByInviteCode(code ?? ""),
    enabled: Boolean(code?.trim()),
    retry: false,
  });
}

export function useGameDetailQuery(gameId: string | null) {
  const { api } = useFriendarazziApi();
  const phase = useAuth();
  const visible = useDocumentVisible();
  return useQuery({
    queryKey: friendarazziKeys.games.detail(gameId ?? ""),
    queryFn: () => api.gameDetail(gameId ?? ""),
    enabled: phase.status === "authenticated" && Boolean(gameId),
    refetchInterval: (query) =>
      !visible || query.state.data?.status === "Completed" ? false : 5000,
    retry: false,
  });
}

export function useUserProfileQuery(userId: string | null) {
  const { api } = useFriendarazziApi();
  const phase = useAuth();
  return useQuery({
    queryKey: friendarazziKeys.profiles.detail(userId ?? ""),
    queryFn: () => api.profileGet(userId ?? ""),
    enabled: phase.status === "authenticated" && Boolean(userId),
    retry: false,
  });
}

export function useUserGalleryQuery(userId: string | null) {
  const { api } = useFriendarazziApi();
  const phase = useAuth();
  return useQuery({
    queryKey: friendarazziKeys.profiles.gallery(userId ?? ""),
    queryFn: () => api.userGallery(userId ?? ""),
    enabled: phase.status === "authenticated" && Boolean(userId),
    retry: false,
  });
}

export function useNotificationsQuery() {
  const { api } = useFriendarazziApi();
  const phase = useAuth();
  const visible = useDocumentVisible();
  return useQuery({
    queryKey: friendarazziKeys.notifications.inbox(),
    queryFn: () => api.notificationsList(),
    enabled: phase.status === "authenticated",
    refetchInterval: visible ? 10_000 : false,
    retry: false,
  });
}

export function useNotificationPreferencesQuery() {
  const { api } = useFriendarazziApi();
  const phase = useAuth();
  return useQuery({
    queryKey: friendarazziKeys.notifications.preferences(),
    queryFn: () => api.notificationPreferencesGet(),
    enabled: phase.status === "authenticated",
    retry: false,
  });
}

/** Fetch an authenticated media blob and return a managed object URL. */
export function useMediaObjectUrl(storageKey: string | null | undefined) {
  const { api } = useFriendarazziApi();
  const phase = useAuth();
  const query = useQuery({
    queryKey: friendarazziKeys.media(storageKey ?? ""),
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
