import { useParams } from 'react-router-dom';

export function InvitePage() {
  const { inviteCode } = useParams();

  return (
    <section className="rounded-lg border-4 border-ink bg-paper p-6 shadow-sticker">
      <p className="text-sm font-black uppercase text-teal">Invite</p>
      <h2 className="mt-2 text-4xl font-black">💌 {inviteCode}</h2>
      <p className="mt-4 max-w-2xl text-lg text-slate-700">
        This public invite route is ready for RSVP forms, event details, and
        cheerful confirmation states.
      </p>
    </section>
  );
}
