import type {
  BusinessRulesDSL,
  ValidateResponse,
  CompileResponse,
  GenerateSdkResponse,
  DeployResponse,
  TemplatesResponse,
} from '@/types/dsl';

const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:8082';

class ApiError extends Error {
  constructor(
    message: string,
    public status?: number,
    public response?: unknown
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

async function fetchAPI<T>(
  endpoint: string,
  options?: RequestInit
): Promise<T> {
  const url = `${API_BASE_URL}${endpoint}`;

  try {
    const response = await fetch(url, {
      ...options,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
      },
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({}));
      throw new ApiError(
        error.error || `HTTP ${response.status}: ${response.statusText}`,
        response.status,
        error
      );
    }

    return await response.json();
  } catch (error) {
    if (error instanceof ApiError) {
      throw error;
    }
    throw new ApiError(
      `Network error: ${error instanceof Error ? error.message : 'Unknown error'}`
    );
  }
}

export const api = {
  /**
   * Health check
   */
  async healthCheck(): Promise<{ status: string; service: string }> {
    return fetchAPI('/health');
  },

  /**
   * Validate DSL without compiling
   */
  async validateDSL(dsl: BusinessRulesDSL): Promise<ValidateResponse> {
    return fetchAPI('/api/validate', {
      method: 'POST',
      body: JSON.stringify({ dsl }),
    });
  },

  /**
   * Compile DSL to guest program code
   */
  async compileDSL(dsl: BusinessRulesDSL): Promise<CompileResponse> {
    return fetchAPI('/api/compile', {
      method: 'POST',
      body: JSON.stringify({ dsl }),
    });
  },

  /**
   * Generate SDK package
   */
  async generateSDK(dsl: BusinessRulesDSL): Promise<GenerateSdkResponse> {
    return fetchAPI('/api/sdk/generate', {
      method: 'POST',
      body: JSON.stringify({ dsl }),
    });
  },

  /**
   * Deploy DSL to gateway
   */
  async deployDSL(dsl: BusinessRulesDSL, customerId: string): Promise<DeployResponse> {
    return fetchAPI('/api/deploy', {
      method: 'POST',
      body: JSON.stringify({ dsl, customer_id: customerId }),
    });
  },

  /**
   * Download SDK package
   */
  async downloadSDK(sdkId: string): Promise<void> {
    const url = `${API_BASE_URL}/api/sdk/download/${sdkId}`;

    try {
      const response = await fetch(url);
      if (!response.ok) {
        throw new Error(`Download failed: ${response.statusText}`);
      }

      // Get filename from Content-Disposition header or use default
      const contentDisposition = response.headers.get('Content-Disposition');
      let filename = `${sdkId}.tar.gz`;

      if (contentDisposition) {
        const filenameMatch = contentDisposition.match(/filename="?(.+?)"?$/);
        if (filenameMatch) {
          filename = filenameMatch[1];
        }
      }

      // Download the blob
      const blob = await response.blob();
      const blobUrl = window.URL.createObjectURL(blob);

      // Create temporary link and click it
      const link = document.createElement('a');
      link.href = blobUrl;
      link.download = filename;
      document.body.appendChild(link);
      link.click();

      // Cleanup
      document.body.removeChild(link);
      window.URL.revokeObjectURL(blobUrl);
    } catch (error) {
      console.error('Download failed:', error);
      throw error;
    }
  },

  /**
   * List available templates
   */
  async listTemplates(): Promise<TemplatesResponse> {
    return fetchAPI('/api/templates');
  },

  /**
   * Get a specific template
   */
  async getTemplate(name: string): Promise<BusinessRulesDSL> {
    return fetchAPI(`/api/templates/${name}`);
  },
};

export { ApiError };
