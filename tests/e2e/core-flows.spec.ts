import { expect, test, type Page, type Route } from '@playwright/test';

type MockUser = {
  sub: string;
  email: string;
  email_verified: boolean;
  name: string | null;
  picture_url: string | null;
  registered: boolean;
};

const hostUser: MockUser = {
  sub: 'host-1',
  email: 'host@example.com',
  email_verified: true,
  name: 'Harper Host',
  picture_url: null,
  registered: true,
};

const guestUser: MockUser = {
  sub: 'guest-1',
  email: 'guest@example.com',
  email_verified: true,
  name: 'Taylor Guest',
  picture_url: null,
  registered: true,
};

const eventRecord = {
  id: 'event-1',
  owner_sub: 'host-1',
  title: 'Rooftop Supper',
  description: 'Bring a jacket and a favorite story.',
  starts_at: '2026-08-01T01:30:00.000Z',
  timezone: 'America/New_York',
  cover_image_object_key: null,
  created_at: '2026-07-18T18:00:00.000Z',
  updated_at: '2026-07-18T18:00:00.000Z',
};

test('auth entry covers sign-up, verification, platform login, and Google login paths', async ({
  page,
}) => {
  let signedIn = false;
  let currentUser = hostUser;

  await mockApi(page, {
    user: () => (signedIn ? currentUser : null),
    register: () => {
      signedIn = true;
      currentUser = { ...hostUser, registered: true };
      return {
        user: currentUser,
        message: 'Registration is complete. Welcome in.',
        email_delivery: { status: 'skipped', id: null, message: null },
      };
    },
    verify: () => ({
      verified: true,
      message: 'Your email is verified.',
    }),
  });

  await page.goto('/auth?mode=login&next=%2Fdashboard');

  await expect(
    page.getByRole('heading', { name: 'Come on in.' }),
  ).toBeVisible();
  await expect(
    page.getByRole('link', { name: 'Continue with platform login' }),
  ).toHaveAttribute('href', /\/api\/auth\/login/);
  await expect(
    page.getByRole('link', { name: 'Continue with Google' }),
  ).toHaveAttribute('href', /\/api\/auth\/google/);

  await page.getByRole('button', { name: 'Sign up' }).click();
  await expect(
    page.getByRole('link', { name: 'Start with Google' }),
  ).toHaveAttribute('href', /\/api\/auth\/google/);

  signedIn = true;
  await page.goto('/auth?mode=signup');
  await page.getByRole('button', { name: 'Finish registration' }).click();
  await expect(
    page.getByText('Registration is complete. Welcome in.'),
  ).toBeVisible();

  await page.getByRole('button', { name: 'Verify email' }).click();
  await page.getByRole('button', { name: 'Check verification' }).click();
  await expect(page.getByText('Your email is verified.')).toBeVisible();
});

test('core event journey creates, invites, RSVPs, comments, and updates activity', async ({
  page,
}) => {
  let currentUser = hostUser;
  let activity = [
    activityItem(
      'activity-edit',
      'event.edited',
      'Harper Host updated the event details.',
    ),
  ];
  const comments: unknown[] = [];
  const attendees = [
    {
      invitation_id: 'invite-1',
      event_id: 'event-1',
      invitee_sub: 'guest-1',
      invitee_email: 'guest@example.com',
      display_name: 'Taylor Guest',
      picture_url: null,
      invitation_status: 'invited',
      rsvp_response: null,
      rsvp_note: null,
      responded_at: null,
      updated_at: '2026-07-18T18:05:00.000Z',
    },
  ];

  await mockApi(page, {
    user: () => currentUser,
    dashboardEvents: () => ({
      events: [
        {
          ...eventRecord,
          cover_image_url: null,
          relationship:
            currentUser.sub === eventRecord.owner_sub ? 'organizer' : 'invited',
        },
      ],
    }),
    createEvent: () => eventRecord,
    event: () => eventRecord,
    attachments: () => ({ attachments: [] }),
    attendees: () => ({ attendees }),
    comments: () => ({ comments }),
    activity: () => ({ activity }),
    sendInvitations: () => ({
      invitations: [
        {
          invitation: {
            id: 'invite-1',
            event_id: 'event-1',
            inviter_sub: 'host-1',
            invitee_sub: null,
            invitee_email: 'guest@example.com',
            status: 'invited',
            message: null,
            created_at: '2026-07-18T18:05:00.000Z',
            updated_at: '2026-07-18T18:05:00.000Z',
          },
          email_delivery: {
            email: 'guest@example.com',
            status: 'sent',
            id: 'email-1',
            message: null,
          },
        },
      ],
    }),
    rsvp: () => {
      attendees[0] = {
        ...attendees[0],
        invitation_status: 'accepted',
        rsvp_response: 'yes',
        rsvp_note: 'Saving a seat.',
        responded_at: '2026-07-18T18:10:00.000Z',
      };
      activity = [
        activityItem('activity-rsvp', 'rsvp.updated', 'Taylor Guest is in.'),
        ...activity,
      ];
      return {
        invitation: {
          id: 'invite-1',
          event_id: 'event-1',
          inviter_sub: 'host-1',
          invitee_sub: 'guest-1',
          invitee_email: 'guest@example.com',
          status: 'accepted',
          message: null,
          created_at: '2026-07-18T18:05:00.000Z',
          updated_at: '2026-07-18T18:10:00.000Z',
        },
        rsvp: {
          id: 'rsvp-1',
          invitation_id: 'invite-1',
          event_id: 'event-1',
          user_sub: 'guest-1',
          response: 'yes',
          note: 'Saving a seat.',
          responded_at: '2026-07-18T18:10:00.000Z',
          created_at: '2026-07-18T18:10:00.000Z',
          updated_at: '2026-07-18T18:10:00.000Z',
        },
        email_delivery: { status: 'skipped', id: null, message: null },
      };
    },
    createComment: (body) => {
      const comment = {
        id: 'comment-1',
        event_id: 'event-1',
        author: {
          sub: 'guest-1',
          email: 'guest@example.com',
          name: 'Taylor Guest',
          picture_url: null,
        },
        body,
        created_at: '2026-07-18T18:15:00.000Z',
        updated_at: '2026-07-18T18:15:00.000Z',
      };
      comments.push(comment);
      activity = [
        activityItem(
          'activity-comment',
          'comment.created',
          'Taylor Guest added a comment.',
        ),
        ...activity,
      ];
      return { comment };
    },
  });

  await page.goto('/events/new');
  await page.getByLabel('Title').fill(eventRecord.title);
  await page.getByLabel('Description').fill(eventRecord.description);
  await page.getByLabel('Date and time').fill('2026-07-31T21:30');
  await page.getByRole('button', { name: 'Create event' }).click();
  await expect(page.getByText('Saved. Cue the confetti.')).toBeVisible();

  await page.goto('/events/event-1');
  await expect(
    page.getByRole('heading', { name: eventRecord.title }),
  ).toBeVisible();
  await page.getByLabel('Emails').fill('guest@example.com');
  await page.getByRole('button', { name: 'Send invites' }).click();
  await expect(page.getByText('Sent 1 invite.')).toBeVisible();

  currentUser = guestUser;
  await page.goto('/events/event-1');
  await page.getByRole('button', { name: 'Going' }).click();
  await page.getByLabel('Note').fill('Saving a seat.');
  await page.getByRole('button', { name: 'Save RSVP' }).click();
  await expect(page.getByText("You're on the list.")).toBeVisible();
  await expect(page.getByText('Taylor Guest is in.')).toBeVisible();

  await page.getByLabel('Add a comment').fill('Can bring dessert.');
  await page.getByRole('button', { name: 'Post comment' }).click();
  await expect(page.getByText('Can bring dessert.')).toBeVisible();
  await expect(page.getByText('Taylor Guest added a comment.')).toBeVisible();
  await expect(
    page.locator('div.vibe-pop', { hasText: 'New comment' }),
  ).toBeVisible();
});

async function mockApi(
  page: Page,
  handlers: {
    user: () => MockUser | null;
    register?: () => unknown;
    verify?: () => unknown;
    dashboardEvents?: () => unknown;
    createEvent?: () => unknown;
    event?: () => unknown;
    attachments?: () => unknown;
    attendees?: () => unknown;
    comments?: () => unknown;
    activity?: () => unknown;
    sendInvitations?: () => unknown;
    rsvp?: () => unknown;
    createComment?: (body: string) => unknown;
  },
) {
  const handleApiRoute = async (route: Route) => {
    const request = route.request();
    const url = new URL(request.url());
    const path = url.pathname;
    const method = request.method();

    if (path === '/api/me') {
      const user = handlers.user();
      return user ? json(route, user) : json(route, { error: {} }, 401);
    }

    if (path === '/api/auth/register' && method === 'POST') {
      return json(route, handlers.register?.() ?? {});
    }

    if (path === '/api/auth/verify') {
      return json(
        route,
        handlers.verify?.() ?? { verified: true, message: 'Verified.' },
      );
    }

    if (path === '/api/dashboard/events') {
      return json(route, handlers.dashboardEvents?.() ?? { events: [] });
    }

    if (path === '/api/events' && method === 'POST') {
      return json(route, handlers.createEvent?.() ?? eventRecord, 201);
    }

    if (path === '/api/events/event-1' && method === 'GET') {
      return json(route, handlers.event?.() ?? eventRecord);
    }

    if (path === '/api/events/event-1/attachments') {
      return json(route, handlers.attachments?.() ?? { attachments: [] });
    }

    if (path === '/api/events/event-1/attendees') {
      return json(route, handlers.attendees?.() ?? { attendees: [] });
    }

    if (path === '/api/events/event-1/comments' && method === 'GET') {
      return json(route, handlers.comments?.() ?? { comments: [] });
    }

    if (path === '/api/events/event-1/comments' && method === 'POST') {
      const body = (await request.postDataJSON()) as { body: string };
      return json(route, handlers.createComment?.(body.body) ?? {}, 201);
    }

    if (path === '/api/events/event-1/activity') {
      return json(route, handlers.activity?.() ?? { activity: [] });
    }

    if (path === '/api/events/event-1/invitations' && method === 'POST') {
      return json(
        route,
        handlers.sendInvitations?.() ?? { invitations: [] },
        201,
      );
    }

    if (path === '/api/events/event-1/rsvp' && method === 'PUT') {
      return json(route, handlers.rsvp?.() ?? {});
    }

    return json(
      route,
      { error: { message: `Unhandled ${method} ${path}` } },
      500,
    );
  };

  await page.route('http://localhost:8080/api/**', handleApiRoute);
  await page.route('http://127.0.0.1:8080/api/**', handleApiRoute);
  await page.route('http://127.0.0.1:5173/api/**', handleApiRoute);
}

function activityItem(id: string, activityType: string, message: string) {
  return {
    id,
    event_id: 'event-1',
    actor: {
      sub: activityType === 'event.edited' ? 'host-1' : 'guest-1',
      email:
        activityType === 'event.edited'
          ? 'host@example.com'
          : 'guest@example.com',
      name: activityType === 'event.edited' ? 'Harper Host' : 'Taylor Guest',
      picture_url: null,
    },
    activity_type: activityType,
    message,
    payload: {},
    created_at: '2026-07-18T18:15:00.000Z',
  };
}

async function json(route: Route, body: unknown, status = 200) {
  await route.fulfill({
    status,
    contentType: 'application/json',
    body: JSON.stringify(body),
  });
}
