const DEFAULT_API_BASE = process.env.NEXT_PUBLIC_ROHAS_API_URL || "http://127.0.0.1:4400";

let cachedApiBaseUrl: string | null = null;

export function getApiBaseUrl(): string {
  if (typeof window !== "undefined") {
    const envUrl = process.env.NEXT_PUBLIC_ROHAS_API_URL;
    if (envUrl) {
      return envUrl;
    }
    
    try {
      const stored = localStorage.getItem("rohas_api_base_url");
      if (stored) {
        return stored;
      }
    } catch {
    }
  } else {
    const envUrl = process.env.ROHAS_API_URL;
    if (envUrl) {
      return envUrl;
    }
    
    if (cachedApiBaseUrl) {
      return cachedApiBaseUrl;
    }
  }
  
  return DEFAULT_API_BASE;
}

export function setApiBaseUrl(url: string): void {
  if (typeof window !== "undefined") {
    try {
      localStorage.setItem("rohas_api_base_url", url);
    } catch {
    }
  }
  cachedApiBaseUrl = url;
}

export type HttpMethod = "GET" | "POST" | "PUT" | "PATCH" | "DELETE";

export interface ApiRequestOptions {
  method?: HttpMethod;
  body?: unknown;
  headers?: Record<string, string>;
  params?: Record<string, string | number | boolean | null | undefined>;
  cache?: RequestCache;
  signal?: AbortSignal;
}

export class ApiError extends Error {
  constructor(
    message: string,
    public status: number,
    public statusText: string,
    public data?: unknown
  ) {
    super(message);
    this.name = "ApiError";
  }
}

async function tryApiRequest<T>(
  baseUrl: string,
  endpoint: string,
  options: ApiRequestOptions = {}
): Promise<T> {
  const {
    method = "GET",
    body,
    headers = {},
    params,
    cache = "no-store",
    signal,
  } = options;

  const url = new URL(endpoint, baseUrl);
  if (params) {
    Object.entries(params).forEach(([key, value]) => {
      if (value !== null && value !== undefined) {
        url.searchParams.append(key, String(value));
      }
    });
  }

  const getApiKey = (): string | null => {
    if (typeof window !== "undefined") {
      const envKey = process.env.NEXT_PUBLIC_ROHAS_WORKBENCH_API_KEY;
      if (envKey) return envKey;
      
      try {
        const stored = localStorage.getItem("rohas_workbench_api_key");
        if (stored) return stored;
      } catch {
      }
    } else {
      const envKey = process.env.ROHAS_WORKBENCH_API_KEY;
      if (envKey) return envKey;
    }
    return null;
  };

  const requestHeaders: Record<string, string> = {
    "Content-Type": "application/json",
    ...headers,
  };

  const apiKey = getApiKey();
  if (apiKey && endpoint.startsWith("/api/workbench")) {
    requestHeaders["Authorization"] = `Bearer ${apiKey}`;
    requestHeaders["X-API-Key"] = apiKey;
  }

  const requestConfig: RequestInit = {
    method,
    headers: requestHeaders,
    cache,
    signal,
  };

  if (method !== "GET" && body !== undefined) {
    requestConfig.body = JSON.stringify(body);
  }

  const response = await fetch(url.toString(), requestConfig);

  let responseData: unknown;
  const contentType = response.headers.get("content-type");
  if (contentType?.includes("application/json")) {
    try {
      responseData = await response.json();
    } catch {
      responseData = null;
    }
  } else {
    responseData = await response.text();
  }

  if (!response.ok) {
    const errorMessage =
      (responseData && typeof responseData === "object" && "error" in responseData
        ? String((responseData as { error: unknown }).error)
        : null) || response.statusText;

    throw new ApiError(
      `API request failed: ${errorMessage}`,
      response.status,
      response.statusText,
      responseData
    );
  }

  return responseData as T;
}

export async function apiRequest<T>(
  endpoint: string,
  options: ApiRequestOptions = {}
): Promise<T> {
  const baseUrl = getApiBaseUrl();

  try {
    return await tryApiRequest<T>(baseUrl, endpoint, options);
  } catch (error) {
    if (baseUrl === DEFAULT_API_BASE && typeof window !== "undefined") {
      const commonPorts = [3000, 4400, 8000, 8080];
      const host = new URL(baseUrl).hostname;
      
      for (const port of commonPorts) {
        if (port === 4400) continue;
        const testUrl = `http://${host}:${port}`;
        try {
          const result = await tryApiRequest<T>(testUrl, endpoint, options);
          setApiBaseUrl(testUrl);
          return result;
        } catch {
          continue;
        }
      }
    }
    
    throw error;
  }
}

export const api = {
  get: <T>(endpoint: string, options?: Omit<ApiRequestOptions, "method" | "body">) =>
    apiRequest<T>(endpoint, { ...options, method: "GET" }),

  post: <T>(endpoint: string, body?: unknown, options?: Omit<ApiRequestOptions, "method" | "body">) =>
    apiRequest<T>(endpoint, { ...options, method: "POST", body }),

  put: <T>(endpoint: string, body?: unknown, options?: Omit<ApiRequestOptions, "method" | "body">) =>
    apiRequest<T>(endpoint, { ...options, method: "PUT", body }),

  patch: <T>(endpoint: string, body?: unknown, options?: Omit<ApiRequestOptions, "method" | "body">) =>
    apiRequest<T>(endpoint, { ...options, method: "PATCH", body }),

  delete: <T>(endpoint: string, options?: Omit<ApiRequestOptions, "method" | "body">) =>
    apiRequest<T>(endpoint, { ...options, method: "DELETE" }),
};

