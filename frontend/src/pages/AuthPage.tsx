import { useState, type FormEvent, type ReactNode } from 'react';
import { Link, useSearchParams } from 'react-router-dom';

import { friendlyErrorMessage } from '../api/errors';
import { useAuth } from '../auth/useAuth';

const tabs = [
  { key: 'login', label: 'Log in' },
  { key: 'signup', label: 'Sign up' },
  { key: 'verify', label: 'Verify email' },
  { key: 'reset', label: 'Reset' },
] as const;

type AuthTab = (typeof tabs)[number]['key'];

export function AuthPage() {
  const auth = useAuth();
  const [searchParams, setSearchParams] = useSearchParams();
  const activeTab = readTab(searchParams.get('mode'));
  const nextPath = searchParams.get('next') ?? '/dashboard';
  const [email, setEmail] = useState('');
  const [localMessage, setLocalMessage] = useState<string | null>(null);
  const [localError, setLocalError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const displayName =
    auth.user?.name?.trim() || auth.user?.email.split('@')[0] || 'friend';

  async function runAction(action: () => Promise<void>, success: string) {
    setBusy(true);
    setLocalError(null);
    setLocalMessage(null);

    try {
      await action();
      setLocalMessage(success);
    } catch (error) {
      setLocalError(friendlyErrorMessage(error, 'auth'));
    } finally {
      setBusy(false);
    }
  }

  function handleResetRequest(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    void runAction(
      () => auth.requestPasswordReset(email),
      'Check your inbox for a fresh sign-in link.',
    );
  }

  function handleCompleteReset() {
    void runAction(
      () => auth.completePasswordReset(),
      'Use the platform sign-in link to finish getting back in.',
    );
  }

  return (
    <section className="space-y-6">
      <div className="rounded-lg border-4 border-ink bg-white p-6 shadow-sticker">
        <p className="text-sm font-black uppercase text-coral">Account</p>
        <h2 className="mt-2 text-4xl font-black leading-tight">
          Your host pass lives here.
        </h2>
        <p className="mt-4 max-w-2xl text-lg text-slate-700">
          Sign in, finish registration, check email status, or ask for a fresh
          recovery link. Everything stays simple and ready for the next plan.
        </p>
      </div>

      <div className="flex gap-2 overflow-x-auto" aria-label="Auth sections">
        {tabs.map((tab) => (
          <button
            className={[
              'min-h-11 shrink-0 rounded-lg border-2 border-ink px-4 py-2 text-sm font-black transition',
              activeTab === tab.key
                ? 'bg-teal text-white shadow-sticker'
                : 'bg-white hover:bg-mint',
            ].join(' ')}
            key={tab.key}
            onClick={() => setSearchParams({ mode: tab.key, next: nextPath })}
            type="button"
          >
            {tab.label}
          </button>
        ))}
      </div>

      {(localMessage || auth.message) && (
        <div className="rounded-lg border-4 border-ink bg-mint p-4 font-black">
          {localMessage ?? auth.message}
        </div>
      )}

      {(localError || auth.error) && (
        <div className="rounded-lg border-4 border-ink bg-coral/20 p-4 font-black text-ink">
          {localError ?? auth.error}
        </div>
      )}

      {auth.status === 'signed-in' && auth.user ? (
        <div className="flex flex-col gap-4 rounded-lg border-4 border-ink bg-paper p-5 shadow-sticker md:flex-row md:items-center md:justify-between">
          <div className="flex items-center gap-4">
            {auth.user.pictureUrl ? (
              <img
                alt=""
                className="size-16 rounded-lg border-2 border-ink object-cover"
                src={auth.user.pictureUrl}
              />
            ) : (
              <div className="flex size-16 items-center justify-center rounded-lg border-2 border-ink bg-sunny text-2xl font-black">
                {displayName.slice(0, 1).toUpperCase()}
              </div>
            )}
            <div>
              <p className="text-sm font-black uppercase text-teal">
                Signed in
              </p>
              <p className="text-2xl font-black">{displayName}</p>
              <p className="text-slate-700">{auth.user.email}</p>
            </div>
          </div>
          <Link
            className="inline-flex min-h-11 items-center justify-center rounded-lg border-2 border-ink bg-white px-4 py-2 font-black transition hover:bg-mint"
            to="/profile"
          >
            Open profile
          </Link>
        </div>
      ) : null}

      {activeTab === 'login' && (
        <AuthPanel eyebrow="Log in" title="Come on in.">
          <p className="text-slate-700">
            Continue with the platform sign-in flow and land right back where
            you were headed.
          </p>
          <div className="mt-5 flex flex-col gap-3 sm:flex-row">
            <a className={primaryButton} href={auth.buildLoginUrl(nextPath)}>
              Continue with platform login
            </a>
            <a
              className={secondaryButton}
              href={auth.buildGoogleLoginUrl(nextPath)}
            >
              Continue with Google
            </a>
          </div>
        </AuthPanel>
      )}

      {activeTab === 'signup' && (
        <AuthPanel eyebrow="Sign up" title="Save your spot at the table.">
          <p className="text-slate-700">
            New hosts use the same secure platform sign-in. Once you are back,
            finish registration here and we will remember your profile.
          </p>
          {auth.status === 'signed-in' ? (
            <button
              className={`${primaryButton} mt-5`}
              disabled={busy}
              onClick={() =>
                void runAction(
                  auth.registerCurrentUser,
                  'Registration is complete. Welcome in.',
                )
              }
              type="button"
            >
              Finish registration
            </button>
          ) : (
            <a
              className={`${primaryButton} mt-5`}
              href={auth.buildGoogleLoginUrl('/auth?mode=signup')}
            >
              Start with Google
            </a>
          )}
        </AuthPanel>
      )}

      {activeTab === 'verify' && (
        <AuthPanel eyebrow="Email" title="Check the little green light.">
          <p className="text-slate-700">
            We read email verification from the platform session. No extra code
            or secret handshake for you to manage.
          </p>
          {auth.status === 'signed-in' ? (
            <button
              className={`${primaryButton} mt-5`}
              disabled={busy}
              onClick={() =>
                void runAction(
                  auth.verifyEmailStatus,
                  auth.user?.emailVerified
                    ? 'Your email is verified.'
                    : 'Your session is valid. The platform has not marked this email verified yet.',
                )
              }
              type="button"
            >
              Check verification
            </button>
          ) : (
            <a
              className={`${primaryButton} mt-5`}
              href={auth.buildLoginUrl('/auth?mode=verify')}
            >
              Sign in to check
            </a>
          )}
        </AuthPanel>
      )}

      {activeTab === 'reset' && (
        <AuthPanel eyebrow="Reset" title="Lost the thread? We will send one.">
          <p className="text-slate-700">
            Enter your email and we will send a friendly recovery note with a
            secure platform sign-in link.
          </p>
          <form
            className="mt-5 flex flex-col gap-3"
            onSubmit={handleResetRequest}
          >
            <label className="text-sm font-black uppercase text-slate-700">
              Email
              <input
                className="mt-2 min-h-11 w-full rounded-lg border-2 border-ink bg-white px-3 py-2 text-base font-normal"
                onChange={(event) => setEmail(event.target.value)}
                placeholder="you@example.com"
                required
                type="email"
                value={email}
              />
            </label>
            <div className="flex flex-col gap-3 sm:flex-row">
              <button className={primaryButton} disabled={busy} type="submit">
                Send recovery email
              </button>
              <button
                className={secondaryButton}
                disabled={busy}
                onClick={handleCompleteReset}
                type="button"
              >
                I have a reset link
              </button>
            </div>
          </form>
        </AuthPanel>
      )}
    </section>
  );
}

function AuthPanel({
  eyebrow,
  title,
  children,
}: {
  eyebrow: string;
  title: string;
  children: ReactNode;
}) {
  return (
    <article className="rounded-lg border-4 border-ink bg-white p-6 shadow-sticker">
      <p className="text-sm font-black uppercase text-teal">{eyebrow}</p>
      <h3 className="mt-2 text-3xl font-black leading-tight">{title}</h3>
      <div className="mt-4">{children}</div>
    </article>
  );
}

function readTab(value: string | null): AuthTab {
  return tabs.some((tab) => tab.key === value) ? (value as AuthTab) : 'login';
}

const primaryButton =
  'inline-flex min-h-11 items-center justify-center rounded-lg border-2 border-ink bg-sunny px-5 py-2 font-black shadow-sticker transition hover:-translate-y-0.5 disabled:cursor-not-allowed disabled:opacity-60';

const secondaryButton =
  'inline-flex min-h-11 items-center justify-center rounded-lg border-2 border-ink bg-white px-5 py-2 font-black transition hover:bg-mint disabled:cursor-not-allowed disabled:opacity-60';
