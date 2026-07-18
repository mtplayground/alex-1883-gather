import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react';

import {
  ApiError,
  apiClient,
  type CurrentUserResponse,
} from '../api/client';
import { AuthContext, type AuthContextValue, type AuthUser } from './context';

function toAuthUser(user: CurrentUserResponse): AuthUser {
  return {
    name: user.name,
    email: user.email,
    emailVerified: user.email_verified,
    pictureUrl: user.picture_url,
    registered: user.registered,
  };
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<AuthUser | null>(null);
  const [status, setStatus] = useState<AuthContextValue['status']>('loading');
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refreshSession = useCallback(async () => {
    try {
      const currentUser = await apiClient.me();
      setUser(toAuthUser(currentUser));
      setStatus('signed-in');
      setError(null);
    } catch (sessionError) {
      setUser(null);
      setStatus('signed-out');

      if (!(sessionError instanceof ApiError && sessionError.status === 401)) {
        setError('We could not check your session. Try again in a moment.');
      }
    }
  }, []);

  useEffect(() => {
    let cancelled = false;

    apiClient
      .me()
      .then((currentUser) => {
        if (cancelled) {
          return;
        }

        setUser(toAuthUser(currentUser));
        setStatus('signed-in');
        setError(null);
      })
      .catch((sessionError: unknown) => {
        if (cancelled) {
          return;
        }

        setUser(null);
        setStatus('signed-out');

        if (!(sessionError instanceof ApiError && sessionError.status === 401)) {
          setError('We could not check your session. Try again in a moment.');
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  const buildLoginUrl = useCallback((returnPath = '/') => {
    return apiClient.loginUrl(returnPath);
  }, []);

  const buildGoogleLoginUrl = useCallback((returnPath = '/') => {
    return apiClient.googleLoginUrl(returnPath);
  }, []);

  const registerCurrentUser = useCallback(async () => {
    setError(null);
    const response = await apiClient.register();
    setUser(toAuthUser(response.user));
    setStatus('signed-in');
    setMessage(response.message);
  }, []);

  const verifyEmailStatus = useCallback(async () => {
    setError(null);
    const response = await apiClient.verifyEmail();
    setMessage(response.message);
    await refreshSession();
  }, [refreshSession]);

  const requestPasswordReset = useCallback(async (email: string) => {
    setError(null);
    const response = await apiClient.requestPasswordReset(email);
    setMessage(response.message);
  }, []);

  const completePasswordReset = useCallback(async () => {
    setError(null);
    const response = await apiClient.completePasswordReset();
    setMessage(response.message);
  }, []);

  const value = useMemo<AuthContextValue>(
    () => ({
      user,
      status,
      message,
      error,
      buildLoginUrl,
      buildGoogleLoginUrl,
      refreshSession,
      registerCurrentUser,
      verifyEmailStatus,
      requestPasswordReset,
      completePasswordReset,
    }),
    [
      user,
      status,
      message,
      error,
      buildLoginUrl,
      buildGoogleLoginUrl,
      refreshSession,
      registerCurrentUser,
      verifyEmailStatus,
      requestPasswordReset,
      completePasswordReset,
    ],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}
