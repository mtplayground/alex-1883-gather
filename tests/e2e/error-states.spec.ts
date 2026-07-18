import { expect, test, type Page, type Route } from '@playwright/test';

const user = {
  sub: 'host-1',
  email: 'host@example.com',
  email_verified: true,
  name: 'Harper Host',
  picture_url: null,
  registered: true,
};

test('dashboard permission failures use friendly copy', async ({ page }) => {
  await mockApi(page, async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;

    if (path === '/api/me') {
      return json(route, user);
    }

    if (path === '/api/dashboard/events') {
      return json(
        route,
        {
          error: {
            code: 'event_forbidden',
            message: 'you do not have access to this event',
          },
        },
        403,
      );
    }

    return json(route, { error: { message: 'Unhandled test route' } }, 500);
  });

  await page.goto('/dashboard');

  await expect(
    page.getByRole('heading', { name: 'The event list did not load.' }),
  ).toBeVisible();
  await expect(
    page.getByText('That event is private to its organizer and guest list.'),
  ).toBeVisible();
});

test('expired sessions get a helpful account-action message', async ({
  page,
}) => {
  await mockApi(page, async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;

    if (path === '/api/me') {
      return json(route, user);
    }

    if (path === '/api/auth/register') {
      return json(
        route,
        {
          error: {
            code: 'not_authenticated',
            message: 'valid platform session required',
          },
        },
        401,
      );
    }

    return json(route, { error: { message: 'Unhandled test route' } }, 500);
  });

  await page.goto('/auth?mode=signup');
  await page.getByRole('button', { name: 'Finish registration' }).click();

  await expect(
    page.getByText(
      'Your session has expired. Sign in again and we will bring you back.',
    ),
  ).toBeVisible();
});

async function mockApi(page: Page, handler: (route: Route) => Promise<void>) {
  await page.route('http://localhost:8080/api/**', handler);
  await page.route('http://127.0.0.1:8080/api/**', handler);
  await page.route('http://127.0.0.1:5173/api/**', handler);
}

async function json(route: Route, body: unknown, status = 200) {
  await route.fulfill({
    status,
    contentType: 'application/json',
    body: JSON.stringify(body),
  });
}
