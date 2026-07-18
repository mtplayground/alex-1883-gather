import { useCallback, useMemo, type ReactNode } from 'react';

import { config } from '../config';
import { AuthContext, type AuthContextValue } from './context';

export function AuthProvider({ children }: { children: ReactNode }) {
  const buildLoginUrl = useCallback((returnPath = '/') => {
    const returnTo = new URL(returnPath, config.appBaseUrl).href;
    const loginUrl = new URL('/login', config.authBaseUrl);

    loginUrl.searchParams.set('app_token', config.authAppToken);
    loginUrl.searchParams.set('return_to', returnTo);

    return loginUrl.href;
  }, []);

  const value = useMemo<AuthContextValue>(
    () => ({
      user: null,
      status: 'signed-out',
      buildLoginUrl,
    }),
    [buildLoginUrl],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}
