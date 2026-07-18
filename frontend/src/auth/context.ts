import { createContext } from 'react';

export type AuthUser = {
  name: string | null;
  email: string;
  emailVerified: boolean;
  pictureUrl: string | null;
  registered: boolean;
};

export type AuthContextValue = {
  user: AuthUser | null;
  status: 'loading' | 'signed-out' | 'signed-in';
  message: string | null;
  error: string | null;
  buildLoginUrl: (returnPath?: string) => string;
  buildGoogleLoginUrl: (returnPath?: string) => string;
  refreshSession: () => Promise<void>;
  registerCurrentUser: () => Promise<void>;
  verifyEmailStatus: () => Promise<void>;
  requestPasswordReset: (email: string) => Promise<void>;
  completePasswordReset: () => Promise<void>;
};

export const AuthContext = createContext<AuthContextValue | undefined>(
  undefined,
);
