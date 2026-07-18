import { useAuth } from '../auth/useAuth';

export function ProfilePage() {
  const auth = useAuth();
  const displayName =
    auth.user?.name?.trim() || auth.user?.email.split('@')[0] || 'friend';

  return (
    <section className="space-y-6">
      <div className="rounded-lg border-4 border-ink bg-white p-6 shadow-sticker">
        <p className="text-sm font-black uppercase text-lilac">Profile</p>
        <h2 className="mt-2 text-4xl font-black">Host profile</h2>
        <p className="mt-4 max-w-2xl text-lg text-slate-700">
          Your host identity is ready here, with room for gathering preferences
          as the planning tools grow.
        </p>
      </div>

      <div className="flex flex-col gap-5 rounded-lg border-4 border-ink bg-paper p-6 shadow-sticker md:flex-row md:items-center">
        {auth.user?.pictureUrl ? (
          <img
            alt=""
            className="size-24 rounded-lg border-2 border-ink object-cover"
            src={auth.user.pictureUrl}
          />
        ) : (
          <div className="flex size-24 items-center justify-center rounded-lg border-2 border-ink bg-sunny text-4xl font-black">
            {displayName.slice(0, 1).toUpperCase()}
          </div>
        )}
        <div className="space-y-2">
          <p className="text-sm font-black uppercase text-teal">
            {auth.user?.emailVerified ? 'Verified email' : 'Email pending'}
          </p>
          <h3 className="text-3xl font-black">{displayName}</h3>
          <p className="text-slate-700">{auth.user?.email}</p>
          <p className="font-black">
            Session status: {auth.status === 'signed-in' ? 'active' : auth.status}
          </p>
        </div>
      </div>
    </section>
  );
}
