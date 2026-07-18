const cards = [
  {
    icon: '🎪',
    title: 'Events',
    value: '3 drafts',
    color: 'bg-coral',
  },
  {
    icon: '💌',
    title: 'Invites',
    value: '24 queued',
    color: 'bg-teal',
  },
  {
    icon: '📎',
    title: 'Attachments',
    value: 'PDFs soon',
    color: 'bg-lilac',
  },
];

export function DashboardPage() {
  return (
    <section className="space-y-6">
      <div className="rounded-lg border-4 border-ink bg-white p-6 shadow-sticker">
        <p className="text-sm font-black uppercase text-teal">Dashboard</p>
        <h2 className="mt-2 text-4xl font-black leading-tight">
          A bright starting point for every gathering.
        </h2>
        <p className="mt-4 max-w-2xl text-lg text-slate-700">
          Placeholder cards mark the core areas that later issues will fill with
          real event planning flows.
        </p>
      </div>
      <div className="grid gap-4 md:grid-cols-3">
        {cards.map((card) => (
          <article
            className="min-h-40 rounded-lg border-4 border-ink bg-paper p-5 shadow-sticker"
            key={card.title}
          >
            <div
              className={`mb-4 inline-flex size-14 items-center justify-center rounded-lg border-2 border-ink ${card.color} text-3xl`}
            >
              {card.icon}
            </div>
            <h3 className="text-xl font-black">{card.title}</h3>
            <p className="mt-2 text-slate-700">{card.value}</p>
          </article>
        ))}
      </div>
    </section>
  );
}
