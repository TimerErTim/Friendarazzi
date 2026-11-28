import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  type LoginMethodsResponse,
  OidcAccountActionMode,
  OidcProviderKindSchema,
} from "@/api/friendarazzi-api";

import OptionsPage from "./page";

const mockReplace = vi.fn();
let mockSearchQuery = "";
let mockLoginMethods: LoginMethodsResponse;
const mockDeleteMutate = vi.fn();
const mockUnlinkMutate = vi.fn();
const mockSetPasswordMutate = vi.fn();
const mockStartOidcActionMutate = vi.fn();

vi.mock("next/navigation", () => ({
  useRouter: () => ({ replace: mockReplace, push: vi.fn() }),
  useSearchParams: () => new URLSearchParams(mockSearchQuery),
}));

vi.mock("@/api/auth-provider", () => ({
  useAuth: () => ({ status: "authenticated" }),
}));

vi.mock("@/api/hooks", () => ({
  useErfolgsmeldung: () => ({
    melden: vi.fn(),
    verwerfen: vi.fn(),
  }),
  useNotificationPreferencesQuery: () => ({
    isLoading: false,
    isError: false,
    data: {
      defaultWatchGames: true,
      emailGameStarted: true,
      emailCapturePosted: true,
      emailGameCompleted: true,
    },
  }),
  useLoginMethodsQuery: () => ({
    isLoading: false,
    isError: false,
    data: mockLoginMethods,
  }),
  useFriendarazziMutations: () => ({
    updateNotificationPreferences: {
      mutate: vi.fn(),
      isPending: false,
      isError: false,
      error: null,
    },
  }),
  useAuthMutations: () => ({
    deleteAccount: {
      mutate: mockDeleteMutate,
      isPending: false,
      error: null,
      reset: vi.fn(),
    },
    unlinkExternalIdentity: {
      mutate: mockUnlinkMutate,
      isPending: false,
      error: null,
    },
    setInitialPassword: {
      mutate: mockSetPasswordMutate,
      isPending: false,
      error: null,
      reset: vi.fn(),
    },
    startOidcAccountAction: {
      mutate: mockStartOidcActionMutate,
      isPending: false,
      error: null,
    },
  }),
}));

describe("OptionsPage Anmeldemethoden & passwordless deletion", () => {
  beforeEach(() => {
    mockSearchQuery = "";
    mockReplace.mockReset();
    mockDeleteMutate.mockReset();
    mockUnlinkMutate.mockReset();
    mockSetPasswordMutate.mockReset();
    mockStartOidcActionMutate.mockReset();
    mockLoginMethods = {
      hasPassword: false,
      externalIdentities: [
        {
          id: "id-apple-1",
          providerId: "apple",
          providerDisplayName: "Apple",
          providerEmail: "relay@privaterelay.appleid.com",
          createdAt: "2026-11-28T12:00:00Z",
        },
      ],
      availableProviders: [
        {
          id: "apple",
          kind: OidcProviderKindSchema.apple,
          displayName: "Apple",
        },
        {
          id: "google",
          kind: OidcProviderKindSchema.google,
          displayName: "Google",
        },
      ],
    };
  });

  it("disables unlinking the final login method and offers linking + setting initial password", () => {
    render(<OptionsPage />);

    expect(screen.getByText("Kein Passwort gesetzt")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Trennen" })).toBeDisabled();

    fireEvent.click(
      screen.getByRole("button", { name: /Mit Google verknüpfen/i }),
    );
    expect(mockStartOidcActionMutate).toHaveBeenCalledWith(
      { providerId: "google", mode: OidcAccountActionMode.link },
      expect.any(Object),
    );

    fireEvent.click(
      screen.getByRole("button", { name: /Passwort festlegen/i }),
    );
    const pwInput = screen.getByLabelText("Neues Passwort", {
      selector: "input",
    });
    const confirmInput = screen.getByLabelText("Passwort bestätigen", {
      selector: "input",
    });
    fireEvent.change(pwInput, { target: { value: "korrekt-pferd-batterie" } });
    fireEvent.change(confirmInput, {
      target: { value: "korrekt-pferd-batterie" },
    });
    fireEvent.click(
      screen.getByRole("button", { name: /Passwort speichern/i }),
    );
    expect(mockSetPasswordMutate).toHaveBeenCalledWith(
      "korrekt-pferd-batterie",
      expect.any(Object),
    );
  });

  it("supports passwordless account deletion via OIDC re-authentication code", () => {
    mockSearchQuery = "delete_reauth_code=reauth-tok-123";
    render(<OptionsPage />);

    expect(mockReplace).toHaveBeenCalledWith("/optionen");
    expect(screen.getByText(/Identität bestätigt/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Endgültig löschen/i }));
    expect(mockDeleteMutate).toHaveBeenCalledWith(
      { reauthCode: "reauth-tok-123", bestaetigt: true },
      expect.any(Object),
    );
  });
});
