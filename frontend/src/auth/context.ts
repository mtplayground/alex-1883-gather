import { createContext } from 'react';

export type AuthUser = {
  name: string;
  email: string;
  pictureUrl?: string;
};

export type AuthContextValue = {
  user: AuthUser | null;
  status: 'signed-out' | 'signed-in';
  buildLoginUrl: (returnPath?: string) => string;
};

export const AuthContext = createContext<AuthContextValue | undefined>(
  undefined,
);
