type FrontendConfig = {
  apiBaseUrl: string;
  appBaseUrl: string;
  authBaseUrl: string;
  authAppToken: string;
};

const fallbackApiBaseUrl = 'http://localhost:8080';
const fallbackAppBaseUrl =
  globalThis.location?.origin ?? 'http://localhost:5173';
const fallbackAuthBaseUrl = 'https://auth.mctai.app';
const fallbackAuthAppToken = 'app_alex-1883-gather-f41811';

export const config: FrontendConfig = {
  apiBaseUrl: readUrl('VITE_API_BASE_URL', fallbackApiBaseUrl),
  appBaseUrl: readUrl('VITE_APP_BASE_URL', fallbackAppBaseUrl),
  authBaseUrl: readUrl('VITE_MCTAI_AUTH_URL', fallbackAuthBaseUrl),
  authAppToken: readString('VITE_MCTAI_AUTH_APP_TOKEN', fallbackAuthAppToken),
};

function readUrl(
  name: 'VITE_API_BASE_URL' | 'VITE_APP_BASE_URL' | 'VITE_MCTAI_AUTH_URL',
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

function readString(name: 'VITE_MCTAI_AUTH_APP_TOKEN', fallback: string) {
  const value = import.meta.env[name];

  if (typeof value !== 'string' || value.trim() === '') {
    return fallback;
  }

  return value.trim();
}
