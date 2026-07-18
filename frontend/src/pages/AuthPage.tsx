import { useAuth } from '../auth/useAuth';

export function AuthPage() {
  const auth = useAuth();

  return (
    <section className="rounded-lg border-4 border-ink bg-white p-6 shadow-sticker">
      <p className="text-sm font-black uppercase text-coral">Auth</p>
      <h2 className="mt-2 text-4xl font-black">🔐 Session placeholder</h2>
      <p className="mt-4 max-w-2xl text-lg text-slate-700">
        The shell is prepared for platform login and future session hydration.
      </p>
      <a
        className="mt-6 inline-flex min-h-11 items-center justify-center rounded-lg border-2 border-ink bg-sunny px-5 py-2 font-black shadow-sticker transition hover:-translate-y-0.5"
        href={auth.buildLoginUrl('/dashboard')}
      >
        Continue with platform login
      </a>
    </section>
  );
}
