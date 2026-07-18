type FrontendConfig = {
  apiBaseUrl: string;
  appBaseUrl: string;
};

const fallbackApiBaseUrl = 'http://localhost:8080';
const fallbackAppBaseUrl =
  globalThis.location?.origin ?? 'http://localhost:5173';

export const config: FrontendConfig = {
  apiBaseUrl: readUrl('VITE_API_BASE_URL', fallbackApiBaseUrl),
  appBaseUrl: readUrl('VITE_APP_BASE_URL', fallbackAppBaseUrl),
};

function readUrl(
  name: 'VITE_API_BASE_URL' | 'VITE_APP_BASE_URL',
  fallback: string,
) {
  const value = import.meta.env[name];

  if (typeof value !== 'string' || value.trim() === '') {
    return fallback;
  }

  try {
    const url = new URL(value);
    return url.href.replace(/\/$/, '');
  } catch {
    throw new Error(`${name} must be a valid absolute URL`);
  }
}
