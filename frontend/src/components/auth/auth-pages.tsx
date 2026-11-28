"use client";

import { Camera, CheckCircle2, CircleAlert, Sparkles } from "lucide-react";
import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import {
  type FormEvent,
  type ReactNode,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";

import { ApiError, apiErrorMessage } from "@/api/errors";
import {
  type OidcProviderItem,
  OidcProviderKindSchema,
} from "@/api/friendarazzi-api";
import { useAuthMutations, useAuthProvidersQuery } from "@/api/hooks";
import { useFriendarazziApi } from "@/api/provider";
import { Wordmark } from "@/components/brand/wordmark";
import { Alert } from "@/components/ui/alert";
import { SparkleIcon } from "@/components/ui/brutal-elements";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Field } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { PasswordInput } from "@/components/ui/password-input";
import { cn } from "@/lib/cn";
import { authHrefWithRedirect, safeRedirectPath } from "@/lib/navigation";

const PASSWORD_MIN = 8;
const PASSWORD_MAX = 128;
const POST_AUTH_REDIRECT_KEY = "friendarazzi:post_auth_redirect";

function useMailToken() {
  return useSearchParams().get("token") ?? "";
}

function useRedirectTarget() {
  const fromQuery = useSearchParams().get("redirect");
  if (fromQuery) {
    return safeRedirectPath(fromQuery, "/");
  }
  if (typeof window !== "undefined") {
    try {
      const stored = window.sessionStorage.getItem(POST_AUTH_REDIRECT_KEY);
      return safeRedirectPath(stored, "/");
    } catch {
      return "/";
    }
  }
  return "/";
}

function clearStoredRedirect() {
  if (typeof window === "undefined") return;
  try {
    window.sessionStorage.removeItem(POST_AUTH_REDIRECT_KEY);
  } catch {
    // Ignore storage errors
  }
}

function passwordWithinPolicy(password: string) {
  const length = Array.from(password).length;
  return length >= PASSWORD_MIN && length <= PASSWORD_MAX;
}

type AuthFailure = {
  scope: "form" | "page";
  message: string;
};

type AuthFailureMessages = {
  unauthorized?: string;
  badRequest?: string;
  conflict?: string;
  tooManyRequests?: string;
  invalidLink?: string;
  fallback: string;
};

function authFailure(
  error: unknown,
  messages: AuthFailureMessages,
): AuthFailure | null {
  if (!error) return null;
  if (
    messages.invalidLink &&
    error instanceof ApiError &&
    error.status === 400
  ) {
    return { scope: "page", message: messages.invalidLink };
  }
  const message = apiErrorMessage(error, messages);
  return message ? { scope: "form", message } : null;
}

function AuthAlert({
  variant,
  message,
}: {
  variant: "error" | "success";
  message: string;
}) {
  return (
    <Alert
      variant={variant}
      icon={
        variant === "error" ? (
          <CircleAlert aria-hidden="true" />
        ) : (
          <CheckCircle2 aria-hidden="true" />
        )
      }
    >
      <p>{message}</p>
    </Alert>
  );
}

type AuthStudioProps = {
  mode: "login" | "register" | "password";
  title: string;
  description: ReactNode;
  redirectTarget?: string;
  pageError?: string | null;
  footer?: ReactNode;
  children: ReactNode;
};

/**
 * Two-column neubrutalist Auth Studio: vibrant campaign poster on the left,
 * focused card with mode switcher on the right (no paper dividers).
 */
function AuthStudio({
  mode,
  title,
  description,
  redirectTarget,
  pageError,
  footer,
  children,
}: AuthStudioProps) {
  return (
    <div className="mx-auto flex w-full max-w-5xl items-center px-4 py-8 sm:px-6 lg:py-14">
      <div className="grid w-full items-stretch gap-6 lg:grid-cols-[0.95fr_1.05fr]">
        {/* Left Campaign Poster */}
        <aside className="brutal-shadow-lg hidden flex-col justify-between rounded-md border-[2.5px] border-strong bg-primary p-8 text-primary-foreground lg:flex">
          <div className="space-y-6">
            <div className="flex items-center justify-between">
              <span className="rounded-sm border-2 border-strong bg-surface px-2.5 py-1 font-mono text-xs font-bold text-ink uppercase">
                Photo-Hunt Bingo
              </span>
              <SparkleIcon tone="accent" />
            </div>

            <div className="space-y-3">
              <Wordmark size="lg" />
              <p className="font-display text-2xl leading-tight uppercase">
                Schnapp deine Freunde in flagranti!
              </p>
              <p className="text-sm font-bold">
                Erstelle eine Lobby, würfle Aufträge und hol dir die
                Goldmedaille direkt im Browser.
              </p>
            </div>
          </div>

          <div className="mt-8 grid grid-cols-2 gap-3">
            <div className="brutal-shadow-sm -rotate-2 rounded-sm border-2 border-strong bg-surface p-2.5 text-ink">
              <div className="flex aspect-video items-center justify-center rounded-sm border-2 border-strong bg-cyan-subtle">
                <Camera className="size-7 text-ink" aria-hidden="true" />
              </div>
              <p className="mt-2 font-mono text-[11px] font-bold uppercase">
                Tap to Capture
              </p>
            </div>
            <div className="brutal-shadow-sm rotate-2 rounded-sm border-2 border-strong bg-surface p-2.5 text-ink">
              <div className="flex aspect-video items-center justify-center rounded-sm border-2 border-strong bg-pink-subtle">
                <Sparkles className="size-7 text-ink" aria-hidden="true" />
              </div>
              <p className="mt-2 font-mono text-[11px] font-bold uppercase">
                Undercover Mode
              </p>
            </div>
          </div>
        </aside>

        {/* Right Form Card */}
        <div className="flex flex-col justify-center space-y-5">
          {mode !== "password" ? (
            <nav
              aria-label="Anmeldeart wählen"
              className="brutal-shadow-sm grid grid-cols-2 gap-1 rounded-md border-2 border-strong bg-surface-muted p-1"
            >
              <Link
                href={authHrefWithRedirect("/anmeldung", redirectTarget)}
                className={cn(
                  "flex min-h-11 items-center justify-center rounded-sm border-2 text-sm font-bold transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus",
                  mode === "login"
                    ? "border-strong bg-primary text-primary-foreground"
                    : "border-transparent text-ink hover:bg-surface",
                )}
              >
                Anmelden
              </Link>
              <Link
                href={authHrefWithRedirect("/registrierung", redirectTarget)}
                className={cn(
                  "flex min-h-11 items-center justify-center rounded-sm border-2 text-sm font-bold transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus",
                  mode === "register"
                    ? "border-strong bg-primary text-primary-foreground"
                    : "border-transparent text-ink hover:bg-surface",
                )}
              >
                Registrieren
              </Link>
            </nav>
          ) : null}

          <div className="space-y-1.5">
            <h1 className="font-display text-2xl tracking-tight text-ink sm:text-3xl">
              {title}
            </h1>
            <p className="text-sm font-medium text-secondary">{description}</p>
          </div>

          {pageError ? <AuthAlert variant="error" message={pageError} /> : null}

          <Card>
            <CardContent className="pt-6">{children}</CardContent>
          </Card>

          {footer ? (
            <div className="flex flex-wrap items-center justify-between gap-2 text-sm font-medium text-secondary">
              {footer}
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}

function AuthLink({ href, children }: { href: string; children: ReactNode }) {
  return (
    <Link
      href={href}
      className="inline-flex min-h-11 items-center rounded-sm px-1 font-bold text-ink underline decoration-2 underline-offset-4 hover:bg-primary-subtle focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
    >
      {children}
    </Link>
  );
}

function oidcButtonLabel(provider: OidcProviderItem): string {
  switch (provider.kind) {
    case OidcProviderKindSchema.google:
      return "Mit Google anmelden";
    case OidcProviderKindSchema.apple:
      return "Mit Apple anmelden";
    default:
      return `Mit ${provider.displayName} anmelden`;
  }
}

function OidcProviderButtons({ redirectTarget }: { redirectTarget: string }) {
  const { api } = useFriendarazziApi();
  const providersQuery = useAuthProvidersQuery();
  const providers = providersQuery.data?.providers ?? [];

  if (providers.length === 0) {
    return null;
  }

  const nextPath =
    redirectTarget && redirectTarget !== "/" ? redirectTarget : undefined;

  return (
    <div className="mt-5 space-y-2 border-t-2 border-subtle pt-4">
      <p className="font-mono text-xs font-bold tracking-wider text-secondary uppercase">
        Oder direkt anmelden mit
      </p>
      <div className="space-y-2">
        {providers.map((provider) => (
          <Button
            key={provider.id}
            type="button"
            variant="secondary"
            size="field"
            className="w-full"
            asChild
          >
            <a href={api.oidcLoginStartUrl(provider.id, nextPath)}>
              {oidcButtonLabel(provider)}
            </a>
          </Button>
        ))}
      </div>
    </div>
  );
}

export function LoginPage() {
  const router = useRouter();
  const redirectTarget = useRedirectTarget();
  const auth = useAuthMutations();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");

  const failure = authFailure(auth.login.error, {
    unauthorized: "E-Mail-Adresse oder Passwort sind falsch.",
    tooManyRequests:
      "Zu viele Anmeldeversuche. Bitte warte kurz und versuche es erneut.",
    fallback:
      "Die Anmeldung konnte nicht verarbeitet werden. Bitte versuche es später erneut.",
  });

  function submit(event: FormEvent) {
    event.preventDefault();
    auth.login.mutate(
      { eMailAdresse: email, passwort: password },
      {
        onSuccess: () => {
          clearStoredRedirect();
          router.replace(redirectTarget);
        },
      },
    );
  }

  return (
    <AuthStudio
      mode="login"
      title="Willkommen zurück"
      description="Melde dich mit deiner E-Mail-Adresse und deinem Passwort an."
      redirectTarget={redirectTarget}
      footer={<AuthLink href="/passwort">Passwort vergessen?</AuthLink>}
    >
      <form className="space-y-5" onSubmit={submit}>
        <Field label="E-Mail-Adresse">
          <Input
            autoComplete="email"
            type="email"
            required
            value={email}
            onChange={(event) => setEmail(event.target.value)}
          />
        </Field>

        <Field label="Passwort">
          <PasswordInput
            autoComplete="current-password"
            required
            value={password}
            onChange={(event) => setPassword(event.target.value)}
          />
        </Field>

        {failure?.scope === "form" ? (
          <AuthAlert variant="error" message={failure.message} />
        ) : null}

        <Button
          type="submit"
          size="field"
          className="w-full"
          disabled={auth.login.isPending}
        >
          {auth.login.isPending ? "Wird angemeldet …" : "Anmelden"}
        </Button>
      </form>
      <OidcProviderButtons redirectTarget={redirectTarget} />
    </AuthStudio>
  );
}

export function OidcCallbackPage() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const auth = useAuthMutations();
  const code = searchParams.get("code") ?? "";
  const next = searchParams.get("next");
  const errorParam = searchParams.get("error");
  const providerParam = searchParams.get("provider");
  const triggeredForRef = useRef<string | null>(null);
  const { mutate, isPending, isError } = auth.exchangeOidc;

  useEffect(() => {
    if (!code || errorParam || triggeredForRef.current === code) return;
    triggeredForRef.current = code;
    mutate(code, {
      onSuccess: () => {
        clearStoredRedirect();
        router.replace(safeRedirectPath(next, "/"));
      },
    });
  }, [code, errorParam, mutate, next, router]);

  let errorMessage: string | null = null;
  if (errorParam === "email_conflict") {
    const providerLabel = providerParam ? ` (${providerParam})` : "";
    errorMessage = `Für diese E-Mail-Adresse existiert bereits ein Konto. Bitte melde dich zuerst mit deiner bestehenden Methode an und verknüpfe den Anbieter${providerLabel} unter Konto-Optionen.`;
  } else if (errorParam === "email_not_verified") {
    errorMessage =
      "Dein Anbieter hat diese E-Mail-Adresse nicht bestätigt. Bitte bestätige zuerst deine E-Mail-Adresse beim Anbieter.";
  } else if (errorParam === "cancelled") {
    errorMessage = "Die Anmeldung beim Anbieter wurde abgebrochen.";
  } else if (errorParam || isError || (!code && !isPending)) {
    errorMessage =
      "Der Anmeldelink ist ungültig oder abgelaufen. Bitte starte die Anmeldung erneut.";
  }

  return (
    <AuthStudio
      mode="password"
      title="Anmeldung wird abgeschlossen"
      description="Deine Anmeldung über den externen Anbieter wird geprüft."
    >
      <div className="space-y-4">
        {isPending && !errorMessage ? (
          <p className="font-mono text-sm font-bold text-ink">
            Sitzung wird gestartet …
          </p>
        ) : null}

        {errorMessage ? (
          <AuthAlert variant="error" message={errorMessage} />
        ) : null}

        {errorMessage ? (
          <Button size="field" className="w-full" asChild>
            <Link href="/anmeldung">Zur Anmeldung</Link>
          </Button>
        ) : null}
      </div>
    </AuthStudio>
  );
}

export function EmailVerificationPage() {
  const token = useMailToken();
  const redirectTarget = useRedirectTarget();
  const auth = useAuthMutations();
  const { mutate, isPending, isSuccess, isError, error } = auth.confirmEmail;
  const triggeredForRef = useRef<string | null>(null);

  useEffect(() => {
    if (!token || triggeredForRef.current === token) return;
    triggeredForRef.current = token;
    mutate(token);
  }, [token, mutate]);

  const confirmFailure = authFailure(error, {
    badRequest:
      "Der Bestätigungslink ist ungültig oder abgelaufen. Du kannst im Menü jederzeit eine neue Bestätigungs-Mail anfordern.",
    tooManyRequests:
      "Zu viele Versuche. Bitte warte kurz und versuche es erneut.",
    fallback:
      "Die E-Mail-Adresse konnte nicht bestätigt werden. Bitte versuche es erneut.",
  });

  return (
    <AuthStudio
      mode="password"
      title="E-Mail-Adresse bestätigen"
      description="Wir schalten dein Konto für beliebig viele Runden und E-Mail-Hinweise frei."
      redirectTarget={redirectTarget}
    >
      <div className="space-y-4">
        {!token ? (
          <AuthAlert
            variant="error"
            message="Dieser Bestätigungslink enthält kein gültiges Token."
          />
        ) : isPending ? (
          <p className="font-mono text-sm font-bold text-ink">
            Bestätige deine E-Mail-Adresse …
          </p>
        ) : isError ? (
          <AuthAlert
            variant="error"
            message={
              confirmFailure?.message ??
              "Der Bestätigungslink ist ungültig oder abgelaufen."
            }
          />
        ) : isSuccess ? (
          <AuthAlert
            variant="success"
            message="Deine E-Mail-Adresse ist bestätigt! Du kannst jetzt unbegrenzt viele Spiele starten und beitreten."
          />
        ) : null}

        <Button size="field" className="w-full" asChild>
          <Link href={redirectTarget || "/"}>Weiter zu Friendarazzi</Link>
        </Button>
      </div>
    </AuthStudio>
  );
}

export function RegistrationPage() {
  const router = useRouter();
  const redirectTarget = useRedirectTarget();
  const auth = useAuthMutations();
  const token = useMailToken();
  const [email, setEmail] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [password, setPassword] = useState("");
  const [confirmation, setConfirmation] = useState("");

  const mismatch = useMemo(
    () =>
      password.length > 0 &&
      confirmation.length > 0 &&
      password !== confirmation,
    [password, confirmation],
  );

  const registerFailure = authFailure(auth.register.error, {
    conflict: "Diese E-Mail-Adresse ist bereits vergeben.",
    badRequest:
      "Bitte prüfe deine Eingaben (Passwort 8 bis 128 Zeichen, gültige E-Mail).",
    tooManyRequests:
      "Zu viele Registrierungsversuche. Bitte warte kurz und versuche es erneut.",
    fallback:
      "Das Konto konnte nicht erstellt werden. Bitte versuche es später erneut.",
  });

  if (token) {
    return <EmailVerificationPage />;
  }

  function submit(event: FormEvent) {
    event.preventDefault();
    if (mismatch) return;
    const trimmedName = displayName.trim();
    auth.register.mutate(
      {
        eMailAdresse: email,
        passwort: password,
        ...(trimmedName ? { displayName: trimmedName } : {}),
      },
      {
        onSuccess: () => {
          clearStoredRedirect();
          router.replace(redirectTarget);
        },
      },
    );
  }

  return (
    <AuthStudio
      mode="register"
      title="Konto erstellen"
      description="In einem Schritt registrieren und direkt losspielen – deine E-Mail bestätigst du ganz bequem danach."
      redirectTarget={redirectTarget}
    >
      <form className="space-y-4" onSubmit={submit}>
        <Field label="E-Mail-Adresse">
          <Input
            autoComplete="email"
            type="email"
            required
            value={email}
            onChange={(event) => setEmail(event.target.value)}
          />
        </Field>

        <Field
          label="Anzeigename (optional)"
          hint="Wird deinen Freunden in der Lobby angezeigt."
        >
          <Input
            autoComplete="nickname"
            type="text"
            maxLength={60}
            placeholder="z. B. Lena"
            value={displayName}
            onChange={(event) => setDisplayName(event.target.value)}
          />
        </Field>

        <Field label="Passwort" hint="8 bis 128 Zeichen.">
          <PasswordInput
            autoComplete="new-password"
            minLength={PASSWORD_MIN}
            maxLength={PASSWORD_MAX}
            required
            value={password}
            onChange={(event) => setPassword(event.target.value)}
          />
        </Field>

        <Field
          label="Passwort bestätigen"
          error={mismatch ? "Die Passwörter stimmen nicht überein." : undefined}
        >
          <PasswordInput
            autoComplete="new-password"
            minLength={PASSWORD_MIN}
            maxLength={PASSWORD_MAX}
            required
            aria-invalid={mismatch || undefined}
            value={confirmation}
            onChange={(event) => setConfirmation(event.target.value)}
          />
        </Field>

        {registerFailure ? (
          <AuthAlert variant="error" message={registerFailure.message} />
        ) : null}

        <Button
          type="submit"
          size="field"
          className="w-full"
          disabled={auth.register.isPending || mismatch}
        >
          {auth.register.isPending ? "Wird erstellt …" : "Konto erstellen"}
        </Button>
      </form>
      <OidcProviderButtons redirectTarget={redirectTarget} />
    </AuthStudio>
  );
}

export function PasswordPage() {
  const router = useRouter();
  const auth = useAuthMutations();
  const token = useMailToken();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [sent, setSent] = useState(false);
  const resetting = Boolean(token);

  const requestFailure = authFailure(auth.forgotPassword.error, {
    badRequest: "Bitte gib eine gültige E-Mail-Adresse ein.",
    tooManyRequests:
      "Zu viele Anfragen. Bitte warte kurz und versuche es erneut.",
    fallback:
      "Die Anfrage konnte nicht verarbeitet werden. Bitte versuche es später erneut.",
  });

  const resetFailure = authFailure(auth.resetPassword.error, {
    invalidLink: passwordWithinPolicy(password)
      ? "Der Wiederherstellungslink ist ungültig oder abgelaufen. Bitte fordere einen neuen Link an."
      : undefined,
    badRequest: "Das Passwort muss 8 bis 128 Zeichen lang sein.",
    tooManyRequests:
      "Zu viele Anfragen. Bitte warte kurz und versuche es erneut.",
    fallback:
      "Das Passwort konnte nicht geändert werden. Bitte versuche es später erneut.",
  });

  function request(event: FormEvent) {
    event.preventDefault();
    auth.forgotPassword.mutate(email, { onSuccess: () => setSent(true) });
  }

  function reset(event: FormEvent) {
    event.preventDefault();
    auth.resetPassword.mutate(
      { token, neuesPasswort: password },
      { onSuccess: () => router.replace("/anmeldung") },
    );
  }

  return (
    <AuthStudio
      mode="password"
      title={resetting ? "Neues Passwort" : "Passwort wiederherstellen"}
      description={
        resetting
          ? "Lege ein neues Passwort für dein Konto fest."
          : "Wir senden dir einen Link, mit dem du ein neues Passwort festlegst."
      }
      pageError={resetFailure?.scope === "page" ? resetFailure.message : null}
      footer={
        <>
          <AuthLink href="/anmeldung">Zurück zur Anmeldung</AuthLink>
          {resetting ? (
            <AuthLink href="/passwort">Neuen Link anfordern</AuthLink>
          ) : null}
        </>
      }
    >
      {!resetting ? (
        <form className="space-y-5" onSubmit={request}>
          <Field
            label="E-Mail-Adresse"
            hint="Falls ein Konto zu dieser Adresse gehört, erhältst du eine E-Mail."
          >
            <Input
              autoComplete="email"
              type="email"
              required
              value={email}
              onChange={(event) => setEmail(event.target.value)}
            />
          </Field>

          {sent ? (
            <AuthAlert
              variant="success"
              message="Falls ein Konto zu dieser Adresse gehört, erhältst du eine E-Mail mit weiteren Schritten."
            />
          ) : null}

          {requestFailure ? (
            <AuthAlert variant="error" message={requestFailure.message} />
          ) : null}

          <Button
            type="submit"
            size="field"
            className="w-full"
            disabled={auth.forgotPassword.isPending}
          >
            {auth.forgotPassword.isPending
              ? "Wird gesendet …"
              : "Wiederherstellungslink anfordern"}
          </Button>
        </form>
      ) : (
        <form className="space-y-5" onSubmit={reset}>
          <Field label="Neues Passwort" hint="8 bis 128 Zeichen.">
            <PasswordInput
              autoComplete="new-password"
              minLength={PASSWORD_MIN}
              maxLength={PASSWORD_MAX}
              required
              value={password}
              onChange={(event) => setPassword(event.target.value)}
            />
          </Field>

          {resetFailure?.scope === "form" ? (
            <AuthAlert variant="error" message={resetFailure.message} />
          ) : null}

          <Button
            type="submit"
            size="field"
            className="w-full"
            disabled={auth.resetPassword.isPending}
          >
            {auth.resetPassword.isPending
              ? "Wird geändert …"
              : "Passwort ändern"}
          </Button>
        </form>
      )}
    </AuthStudio>
  );
}
