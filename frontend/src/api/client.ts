import { config } from '../config';

export type HealthResponse = {
  status: string;
  service: string;
  database: string;
  email: string;
  object_storage: string;
};

class ApiClient {
  constructor(private readonly baseUrl: string) {}

  health() {
    return this.get<HealthResponse>('/api/health');
  }

  private async get<T>(path: string): Promise<T> {
    const response = await fetch(`${this.baseUrl}${path}`, {
      headers: {
        Accept: 'application/json',
      },
    });

    if (!response.ok) {
      throw new Error(`API request failed: ${response.status}`);
    }

    return response.json() as Promise<T>;
  }
}

export const apiClient = new ApiClient(config.apiBaseUrl);
