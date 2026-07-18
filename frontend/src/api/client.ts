import { config } from '../config';

export type HealthResponse = {
  status: string;
  service: string;
  database: string;
  email: string;
  object_storage: string;
};

export type CurrentUserResponse = {
  sub: string;
  email: string;
  email_verified: boolean;
  name: string | null;
  picture_url: string | null;
  registered: boolean;
};

export type EmailDelivery = {
  status: 'sent' | 'skipped' | 'rate_limited' | 'failed';
  id: string | null;
  message: string | null;
};

export type AuthActionResponse = {
  user: CurrentUserResponse;
  message: string;
  email_delivery: EmailDelivery;
};

export type VerificationStatusResponse = {
  verified: boolean;
  message: string;
};

export type PasswordResetResponse = {
  message: string;
  login_url: string;
  email_delivery: EmailDelivery;
};

class ApiClient {
  constructor(private readonly baseUrl: string) {}

  health() {
    return this.get<HealthResponse>('/api/health');
  }

  me() {
    return this.get<CurrentUserResponse>('/api/me');
  }

  register() {
    return this.post<AuthActionResponse>('/api/auth/register');
  }

  verifyEmail() {
    return this.get<VerificationStatusResponse>('/api/auth/verify');
  }

  requestPasswordReset(email: string, returnTo = '/dashboard') {
    return this.post<PasswordResetResponse>('/api/auth/password-reset/request', {
      email,
      return_to: this.frontendReturnTo(returnTo),
    });
  }

  completePasswordReset(returnTo = '/dashboard') {
    return this.post<PasswordResetResponse>('/api/auth/password-reset/complete', {
      return_to: this.frontendReturnTo(returnTo),
    });
  }

  loginUrl(returnPath = '/dashboard') {
    return this.authRedirectUrl('/api/auth/login', returnPath);
  }

  googleLoginUrl(returnPath = '/dashboard') {
    return this.authRedirectUrl('/api/auth/google', returnPath);
  }

  private async get<T>(path: string): Promise<T> {
    const response = await fetch(`${this.baseUrl}${path}`, {
      credentials: 'include',
      headers: {
        Accept: 'application/json',
      },
    });

    await this.ensureOk(response);

    return response.json() as Promise<T>;
  }

  private async post<T>(path: string, body?: unknown): Promise<T> {
    const response = await fetch(`${this.baseUrl}${path}`, {
      method: 'POST',
      credentials: 'include',
      headers: {
        Accept: 'application/json',
        'Content-Type': 'application/json',
      },
      body: body === undefined ? undefined : JSON.stringify(body),
    });

    await this.ensureOk(response);

    return response.json() as Promise<T>;
  }

  private async ensureOk(response: Response) {
    if (response.ok) {
      return;
    }

    let message = `API request failed: ${response.status}`;
    try {
      const body = (await response.json()) as {
        error?: { message?: string };
      };
      message = body.error?.message ?? message;
    } catch {
      // Keep the status-based message when the response is not JSON.
    }

    throw new ApiError(response.status, message);
  }

  private authRedirectUrl(path: string, returnPath: string) {
    const url = new URL(path, this.baseUrl);
    url.searchParams.set('return_to', this.frontendReturnTo(returnPath));
    return url.href;
  }

  private frontendReturnTo(returnPath: string) {
    return new URL(returnPath, config.appBaseUrl).href;
  }
}

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    message: string,
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

export const apiClient = new ApiClient(config.apiBaseUrl);
