import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ApiError } from "@/api/errors";

import { EmailVerificationPage, RegistrationPage } from "./auth-pages";

const mockReplace = vi.fn();
let mockToken = "";
let mockRegisterError: unknown = null;
const mockRegisterMutate = vi.fn();
const mockConfirmEmailMutate = vi.fn();
let mockConfirmSuccess = false;

vi.mock("next/navigation", () => ({
  useRouter: () => ({ replace: mockReplace, push: vi.fn() }),
  useSearchParams: () =>
    new URLSearchParams(mockToken ? `token=${mockToken}` : ""),
}));

vi.mock("@/api/hooks", () => ({
  useAuthMutations: () => ({
    login: { mutate: vi.fn(), isPending: false, error: null },
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

describe("RegistrationPage & EmailVerificationPage", () => {
  beforeEach(() => {
    mockToken = "";
    mockRegisterError = null;
    mockConfirmSuccess = false;
    mockReplace.mockReset();
    mockRegisterMutate.mockReset();
    mockConfirmEmailMutate.mockReset();
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
    mockToken = "tok-123";
    mockConfirmSuccess = true;
    render(<EmailVerificationPage />);
    expect(mockConfirmEmailMutate).toHaveBeenCalledWith("tok-123");
    expect(
      screen.getByText(/Deine E-Mail-Adresse ist bestätigt/i),
    ).toBeInTheDocument();
  });
});
