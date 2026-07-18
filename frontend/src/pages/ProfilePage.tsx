import { useEffect, useState, type ChangeEvent, type FormEvent } from 'react';

import {
  ApiError,
  apiClient,
  type ProfileRecord,
  type ProfileResponse,
} from '../api/client';
import { useAuth } from '../auth/useAuth';

const acceptedImageTypes = ['image/jpeg', 'image/png', 'image/webp', 'image/gif'];
const maxPhotoBytes = 5 * 1024 * 1024;

export function ProfilePage() {
  const auth = useAuth();
  const [profile, setProfile] = useState<ProfileRecord | null>(null);
  const [displayName, setDisplayName] = useState('');
  const [bio, setBio] = useState('');
  const [photoUrl, setPhotoUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const fallbackName =
    auth.user?.name?.trim() || auth.user?.email.split('@')[0] || 'friend';
  const visibleName = displayName.trim() || fallbackName;
  const visiblePhoto = photoUrl ?? auth.user?.pictureUrl ?? null;

  useEffect(() => {
    let cancelled = false;

    apiClient
      .profile()
      .then((response) => {
        if (cancelled) {
          return;
        }

        applyProfile(response);
        setLoading(false);
        setError(null);
      })
      .catch((profileError: unknown) => {
        if (cancelled) {
          return;
        }

        setLoading(false);
        setError(readError(profileError));
      });

    return () => {
      cancelled = true;
    };
  }, []);

  function applyProfile(response: ProfileResponse) {
    setProfile(response.profile);
    setDisplayName(response.profile.display_name);
    setBio(response.profile.bio ?? '');
  }

  async function handleSave(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSaving(true);
    setMessage(null);
    setError(null);

    try {
      const response = await apiClient.updateProfile({
        display_name: displayName,
        photo_object_key: profile?.photo_object_key ?? null,
        bio: bio.trim() === '' ? null : bio,
      });
      applyProfile(response);
      setMessage('Profile saved. Looking sharp.');
    } catch (saveError) {
      setError(readError(saveError));
    } finally {
      setSaving(false);
    }
  }

  async function handlePhotoChange(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    event.target.value = '';

    if (!file) {
      return;
    }

    if (!acceptedImageTypes.includes(file.type)) {
      setError('Pick a JPEG, PNG, WebP, or GIF image.');
      return;
    }

    if (file.size > maxPhotoBytes) {
      setError('Pick an image that is 5 MB or smaller.');
      return;
    }

    setUploading(true);
    setMessage(null);
    setError(null);

    try {
      const response = await apiClient.uploadProfilePhoto(file);
      setProfile(response.profile);
      setPhotoUrl(response.access_url);
      setMessage('New profile photo is ready.');
    } catch (uploadError) {
      setError(readError(uploadError));
    } finally {
      setUploading(false);
    }
  }

  return (
    <section className="space-y-6">
      <div className="rounded-lg border-4 border-ink bg-white p-6 shadow-sticker">
        <p className="text-sm font-black uppercase text-lilac">Profile</p>
        <h2 className="mt-2 text-4xl font-black leading-tight">
          Tune your host card.
        </h2>
        <p className="mt-4 max-w-2xl text-lg text-slate-700">
          Add a friendly photo, keep your name tidy, and leave a quick note for
          the gatherings you are shaping.
        </p>
      </div>

      {message ? (
        <div className="rounded-lg border-4 border-ink bg-mint p-4 font-black">
          {message}
        </div>
      ) : null}

      {error ? (
        <div className="rounded-lg border-4 border-ink bg-coral/20 p-4 font-black text-ink">
          {error}
        </div>
      ) : null}

      <div className="grid gap-6 lg:grid-cols-[minmax(260px,360px)_1fr]">
        <aside className="rounded-lg border-4 border-ink bg-paper p-6 shadow-sticker">
          <div className="flex flex-col items-start gap-5">
            {visiblePhoto ? (
              <img
                alt=""
                className="aspect-square w-full max-w-64 rounded-lg border-4 border-ink object-cover"
                src={visiblePhoto}
              />
            ) : (
              <div className="flex aspect-square w-full max-w-64 items-center justify-center rounded-lg border-4 border-ink bg-sunny text-6xl font-black">
                {visibleName.slice(0, 1).toUpperCase()}
              </div>
            )}
            <div>
              <p className="text-sm font-black uppercase text-teal">
                {auth.user?.emailVerified ? 'Verified email' : 'Email pending'}
              </p>
              <h3 className="mt-1 text-3xl font-black">{visibleName}</h3>
              <p className="mt-1 text-slate-700">{auth.user?.email}</p>
            </div>
            <label className={secondaryButton}>
              {uploading ? 'Uploading...' : 'Upload photo'}
              <input
                accept={acceptedImageTypes.join(',')}
                className="sr-only"
                disabled={uploading}
                onChange={handlePhotoChange}
                type="file"
              />
            </label>
            <p className="text-sm text-slate-700">
              JPEG, PNG, WebP, or GIF. Keep it under 5 MB.
            </p>
          </div>
        </aside>

        <form
          className="rounded-lg border-4 border-ink bg-white p-6 shadow-sticker"
          onSubmit={handleSave}
        >
          <p className="text-sm font-black uppercase text-coral">Settings</p>
          <h3 className="mt-2 text-3xl font-black">Profile details</h3>

          {loading ? (
            <p className="mt-5 text-lg font-black">Loading your profile...</p>
          ) : (
            <div className="mt-5 space-y-5">
              <label className="block text-sm font-black uppercase text-slate-700">
                Display name
                <input
                  className="mt-2 min-h-11 w-full rounded-lg border-2 border-ink bg-white px-3 py-2 text-base font-normal"
                  maxLength={80}
                  onChange={(event) => setDisplayName(event.target.value)}
                  required
                  value={displayName}
                />
              </label>

              <label className="block text-sm font-black uppercase text-slate-700">
                Bio
                <textarea
                  className="mt-2 min-h-32 w-full resize-y rounded-lg border-2 border-ink bg-white px-3 py-2 text-base font-normal"
                  maxLength={280}
                  onChange={(event) => setBio(event.target.value)}
                  placeholder="A few words about your hosting style."
                  value={bio}
                />
              </label>

              <div className="grid gap-3 rounded-lg border-2 border-ink bg-mint p-4 sm:grid-cols-2">
                <AccountLine label="Email" value={auth.user?.email ?? 'Unknown'} />
                <AccountLine
                  label="Status"
                  value={auth.user?.emailVerified ? 'Verified' : 'Pending'}
                />
              </div>

              <button className={primaryButton} disabled={saving} type="submit">
                {saving ? 'Saving...' : 'Save settings'}
              </button>
            </div>
          )}
        </form>
      </div>
    </section>
  );
}

function AccountLine({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-xs font-black uppercase text-teal">{label}</p>
      <p className="break-words font-black">{value}</p>
    </div>
  );
}

function readError(error: unknown) {
  if (error instanceof ApiError) {
    return error.message;
  }

  if (error instanceof Error) {
    return error.message;
  }

  return 'That did not land. Give it another try.';
}

const primaryButton =
  'inline-flex min-h-11 items-center justify-center rounded-lg border-2 border-ink bg-sunny px-5 py-2 font-black shadow-sticker transition hover:-translate-y-0.5 disabled:cursor-not-allowed disabled:opacity-60';

const secondaryButton =
  'inline-flex min-h-11 cursor-pointer items-center justify-center rounded-lg border-2 border-ink bg-white px-5 py-2 font-black transition hover:bg-mint has-[:disabled]:cursor-not-allowed has-[:disabled]:opacity-60';
