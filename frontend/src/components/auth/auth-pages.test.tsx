import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ApiError } from "@/api/errors";
import {
  type OidcProviderItem,
  OidcProviderKindSchema,
} from "@/api/friendarazzi-api";

import {
  EmailVerificationPage,
  LoginPage,
  OidcCallbackPage,
  RegistrationPage,
} from "./auth-pages";

const mockReplace = vi.fn();
let mockSearchQuery = "";
let mockRegisterError: unknown = null;
let mockProviders: OidcProviderItem[] = [];
const mockRegisterMutate = vi.fn();
const mockConfirmEmailMutate = vi.fn();
const mockExchangeOidcMutate = vi.fn();
let mockConfirmSuccess = false;

vi.mock("next/navigation", () => ({
  useRouter: () => ({ replace: mockReplace, push: vi.fn() }),
  useSearchParams: () => new URLSearchParams(mockSearchQuery),
}));

vi.mock("@/api/provider", () => ({
  useFriendarazziApi: () => ({
    api: {
      oidcLoginStartUrl: (providerId: string, nextPath?: string) =>
        nextPath
          ? `/api/auth/oidc/${providerId}/start?next=${encodeURIComponent(nextPath)}`
          : `/api/auth/oidc/${providerId}/start`,
    },
  }),
}));

vi.mock("@/api/hooks", () => ({
  useAuthProvidersQuery: () => ({
    data: { providers: mockProviders },
  }),
  useAuthMutations: () => ({
    login: { mutate: vi.fn(), isPending: false, error: null },
    exchangeOidc: {
      mutate: mockExchangeOidcMutate,
      isPending: false,
      isError: false,
      error: null,
    },
    register: {
      mutate: mockRegisterMutate,
      isPending: false,
      error: mockRegisterError,
    },
    confirmEmail: {
      mutate: mockConfirmEmailMutate,
      isPending: false,
      isSuccess: mockConfirmSuccess,
      isError: false,
      error: null,
    },
    resendVerificationEmail: { mutate: vi.fn(), isPending: false },
    forgotPassword: { mutate: vi.fn(), isPending: false, error: null },
    resetPassword: { mutate: vi.fn(), isPending: false, error: null },
  }),
}));

describe("Auth pages & OIDC flows", () => {
  beforeEach(() => {
    mockSearchQuery = "";
    mockRegisterError = null;
    mockConfirmSuccess = false;
    mockProviders = [];
    mockReplace.mockReset();
    mockRegisterMutate.mockReset();
    mockConfirmEmailMutate.mockReset();
    mockExchangeOidcMutate.mockReset();
  });

  it("submits single-step registration with optional display name and shows 409 conflict message", () => {
    mockRegisterError = new ApiError(
      "Diese E-Mail-Adresse ist bereits vergeben",
      {
        message: "Diese E-Mail-Adresse ist bereits vergeben",
        attachments: [],
        sources: [],
      },
      409,
    );

    render(<RegistrationPage />);

    fireEvent.change(screen.getByLabelText(/E-Mail-Adresse/i), {
      target: { value: "lena@example.at" },
    });
    fireEvent.change(screen.getByLabelText(/Anzeigename/i), {
      target: { value: "Lena" },
    });
    const [pwInput, confirmInput] = screen.getAllByLabelText(/Passwort/i);
    fireEvent.change(pwInput, {
      target: { value: "korrekt-pferd-batterie" },
    });
    fireEvent.change(confirmInput, {
      target: { value: "korrekt-pferd-batterie" },
    });

    fireEvent.click(screen.getByRole("button", { name: /Konto erstellen/i }));
    expect(mockRegisterMutate).toHaveBeenCalledWith(
      {
        eMailAdresse: "lena@example.at",
        passwort: "korrekt-pferd-batterie",
        displayName: "Lena",
      },
      expect.any(Object),
    );
    expect(
      screen.getByText("Diese E-Mail-Adresse ist bereits vergeben."),
    ).toBeInTheDocument();
  });

  it("submits email verification token on mount and shows confirmation state", () => {
    mockSearchQuery = "token=tok-123";
    mockConfirmSuccess = true;
    render(<EmailVerificationPage />);
    expect(mockConfirmEmailMutate).toHaveBeenCalledWith("tok-123");
    expect(
      screen.getByText(/Deine E-Mail-Adresse ist bestätigt/i),
    ).toBeInTheDocument();
  });

  it("renders dynamic OIDC provider buttons only when configured and preserves redirect destination", () => {
    mockSearchQuery = "redirect=%2Fjoin%3Fcode%3DABCD23";
    const { rerender } = render(<LoginPage />);
    expect(
      screen.queryByRole("link", { name: /Mit Google anmelden/i }),
    ).not.toBeInTheDocument();

    mockProviders = [
      {
        id: "google",
        kind: OidcProviderKindSchema.google,
        displayName: "Google",
      },
      {
        id: "apple",
        kind: OidcProviderKindSchema.apple,
        displayName: "Apple",
      },
    ];
    rerender(<LoginPage />);

    const googleBtn = screen.getByRole("link", {
      name: /Mit Google anmelden/i,
    });
    expect(googleBtn).toHaveAttribute(
      "href",
      "/api/auth/oidc/google/start?next=%2Fjoin%3Fcode%3DABCD23",
    );
    expect(
      screen.getByRole("link", { name: /Mit Apple anmelden/i }),
    ).toBeInTheDocument();
  });

  it("exchanges OIDC code once on callback mount and shows explicit email_conflict guidance", () => {
    mockSearchQuery = "code=oidc-code-1&next=%2Fgames";
    const { unmount } = render(<OidcCallbackPage />);
    expect(mockExchangeOidcMutate).toHaveBeenCalledWith(
      "oidc-code-1",
      expect.any(Object),
    );
    unmount();

    mockExchangeOidcMutate.mockReset();
    mockSearchQuery = "error=email_conflict&provider=google";
    render(<OidcCallbackPage />);
    expect(mockExchangeOidcMutate).not.toHaveBeenCalled();
    expect(
      screen.getByText(/existiert bereits ein Konto/i),
    ).toBeInTheDocument();
  });
});
