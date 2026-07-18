import { useAuth } from '../auth/useAuth';

export function ProfilePage() {
  const auth = useAuth();

  return (
    <section className="rounded-lg border-4 border-ink bg-white p-6 shadow-sticker">
      <p className="text-sm font-black uppercase text-lilac">Profile</p>
      <h2 className="mt-2 text-4xl font-black">🌟 Host profile</h2>
      <p className="mt-4 max-w-2xl text-lg text-slate-700">
        Auth is currently a placeholder. Later work can hydrate this page from
        the verified session and profile API.
      </p>
      <div className="mt-6 rounded-lg border-2 border-ink bg-mint p-5 font-black">
        Status: {auth.status}
      </div>
    </section>
  );
}
