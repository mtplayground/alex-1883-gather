import { useParams } from 'react-router-dom';

export function EventDetailPage() {
  const { eventId } = useParams();

  return (
    <section className="rounded-lg border-4 border-ink bg-white p-6 shadow-sticker">
      <p className="text-sm font-black uppercase text-coral">Event detail</p>
      <h2 className="mt-2 text-4xl font-black">🎈 {eventId}</h2>
      <div className="mt-6 grid gap-4 md:grid-cols-[1.2fr_0.8fr]">
        <div className="rounded-lg border-2 border-ink bg-mint p-5">
          <h3 className="font-black">Cover story</h3>
          <p className="mt-2 text-slate-700">
            Future event covers, schedules, hosts, and attachment previews land
            here.
          </p>
        </div>
        <div className="rounded-lg border-2 border-ink bg-sunny p-5">
          <h3 className="font-black">Guest pulse</h3>
          <p className="mt-2 text-slate-800">RSVP widgets arrive later.</p>
        </div>
      </div>
    </section>
  );
}
