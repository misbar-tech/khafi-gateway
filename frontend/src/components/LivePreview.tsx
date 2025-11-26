import { useState, useEffect } from 'react';
import {
  CheckCircle,
  XCircle,
  AlertCircle,
  Download,
  Loader2,
  Eye,
  Code2,
  Rocket,
} from 'lucide-react';
import { api, ApiError } from '@/services/api';
import type { BusinessRulesDSL, ValidateResponse, DeployResponse } from '@/types/dsl';

interface LivePreviewProps {
  dsl: BusinessRulesDSL | null;
  autoValidate?: boolean;
}

export function LivePreview({ dsl, autoValidate = true }: LivePreviewProps) {
  const [validationResult, setValidationResult] = useState<ValidateResponse | null>(null);
  const [compiledCode, setCompiledCode] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [compiling, setCompiling] = useState(false);
  const [generating, setGenerating] = useState(false);
  const [deploying, setDeploying] = useState(false);
  const [showCode, setShowCode] = useState(false);
  const [deploymentResult, setDeploymentResult] = useState<DeployResponse | null>(null);

  useEffect(() => {
    if (dsl && autoValidate) {
      validateDSL();
    } else {
      setValidationResult(null);
      setCompiledCode(null);
    }
  }, [dsl, autoValidate]);

  async function validateDSL() {
    if (!dsl) return;

    try {
      setLoading(true);
      const result = await api.validateDSL(dsl);
      setValidationResult(result);
    } catch (err) {
      setValidationResult({
        valid: false,
        error: err instanceof ApiError ? err.message : 'Validation failed',
      });
    } finally {
      setLoading(false);
    }
  }

  async function compileDSL() {
    if (!dsl) return;

    try {
      setCompiling(true);
      const result = await api.compileDSL(dsl);
      if (result.success && result.code) {
        setCompiledCode(result.code);
        setShowCode(true);
      } else {
        setCompiledCode(null);
        alert(result.error || 'Compilation failed');
      }
    } catch (err) {
      alert(err instanceof ApiError ? err.message : 'Compilation failed');
    } finally {
      setCompiling(false);
    }
  }

  async function generateAndDownloadSDK() {
    if (!dsl) return;

    try {
      setGenerating(true);
      const result = await api.generateSDK(dsl);
      if (result.success && result.sdk_id) {
        // Download SDK
        await api.downloadSDK(result.sdk_id);
      } else {
        alert(result.error || 'SDK generation failed');
      }
    } catch (err) {
      alert(err instanceof ApiError ? err.message : 'SDK generation failed');
    } finally {
      setGenerating(false);
    }
  }

  async function deployToGateway() {
    if (!dsl) return;

    // Generate a simple customer ID based on use_case
    const customerId = `customer-${dsl.use_case.toLowerCase().replace(/\s+/g, '-')}-${Date.now()}`;

    try {
      setDeploying(true);
      setDeploymentResult(null);
      const result = await api.deployDSL(dsl, customerId);

      setDeploymentResult(result);

      if (!result.success) {
        alert(result.error || 'Deployment failed');
      }
    } catch (err) {
      alert(err instanceof ApiError ? err.message : 'Deployment failed');
    } finally {
      setDeploying(false);
    }
  }

  const isValid = validationResult?.valid ?? false;

  return (
    <div className="h-full flex flex-col bg-white">
      <div className="p-4 border-b border-gray-200">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center">
            <Eye className="w-5 h-5 text-primary-600 mr-2" />
            <h2 className="text-lg font-semibold text-gray-900">Live Preview</h2>
          </div>
        </div>

        {/* Validation Status */}
        <div className="card p-4 mb-4">
          <h3 className="text-sm font-semibold text-gray-700 mb-3">
            Validation Status
          </h3>

          {loading && (
            <div className="flex items-center text-gray-600">
              <Loader2 className="w-5 h-5 animate-spin mr-2" />
              Validating...
            </div>
          )}

          {!loading && !validationResult && !dsl && (
            <div className="flex items-center text-gray-500">
              <AlertCircle className="w-5 h-5 mr-2" />
              No DSL to validate
            </div>
          )}

          {!loading && validationResult && (
            <div>
              {isValid ? (
                <div className="flex items-start text-green-600">
                  <CheckCircle className="w-5 h-5 mr-2 mt-0.5 flex-shrink-0" />
                  <div>
                    <p className="font-medium">DSL is valid</p>
                    <p className="text-sm text-gray-600 mt-1">
                      Use case: {validationResult.parsed_dsl?.use_case}
                    </p>
                    <p className="text-sm text-gray-600">
                      Rules: {validationResult.parsed_dsl?.validation_rules.length || 0}
                    </p>
                  </div>
                </div>
              ) : (
                <div className="flex items-start text-red-600">
                  <XCircle className="w-5 h-5 mr-2 mt-0.5 flex-shrink-0" />
                  <div>
                    <p className="font-medium">Validation failed</p>
                    {validationResult.error && (
                      <p className="text-sm text-red-700 mt-1 break-words">
                        {validationResult.error}
                      </p>
                    )}
                  </div>
                </div>
              )}
            </div>
          )}
        </div>

        {/* Actions */}
        <div className="space-y-2">
          <button
            onClick={compileDSL}
            disabled={!isValid || compiling}
            className="w-full btn btn-primary flex items-center justify-center"
          >
            {compiling ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin mr-2" />
                Compiling...
              </>
            ) : (
              <>
                <Code2 className="w-4 h-4 mr-2" />
                Compile to Code
              </>
            )}
          </button>

          <button
            onClick={deployToGateway}
            disabled={!isValid || deploying}
            className="w-full btn btn-success flex items-center justify-center"
          >
            {deploying ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin mr-2" />
                Deploying...
              </>
            ) : (
              <>
                <Rocket className="w-4 h-4 mr-2" />
                Deploy to Gateway
              </>
            )}
          </button>

          <button
            onClick={generateAndDownloadSDK}
            disabled={!isValid || generating}
            className="w-full btn btn-secondary flex items-center justify-center text-sm"
          >
            {generating ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin mr-2" />
                Generating...
              </>
            ) : (
              <>
                <Download className="w-4 h-4 mr-2" />
                Download SDK (Legacy)
              </>
            )}
          </button>
        </div>
      </div>

      {/* Deployment Result */}
      {deploymentResult && deploymentResult.success && (
        <div className="p-4 border-b border-gray-200 bg-green-50">
          <div className="card p-4">
            <div className="flex items-start text-green-600 mb-3">
              <CheckCircle className="w-5 h-5 mr-2 mt-0.5 flex-shrink-0" />
              <div>
                <p className="font-semibold">Deployment Successful!</p>
              </div>
            </div>
            <div className="space-y-2 text-sm">
              <div>
                <span className="font-medium text-gray-700">Customer ID:</span>{' '}
                <code className="text-xs bg-gray-100 px-2 py-1 rounded">
                  {deploymentResult.customer_id}
                </code>
              </div>
              <div>
                <span className="font-medium text-gray-700">Image ID:</span>{' '}
                <code className="text-xs bg-gray-100 px-2 py-1 rounded break-all">
                  {deploymentResult.image_id}
                </code>
              </div>
              <div>
                <span className="font-medium text-gray-700">API Endpoint:</span>{' '}
                <code className="text-xs bg-gray-100 px-2 py-1 rounded break-all">
                  {deploymentResult.api_endpoint}
                </code>
              </div>
              <p className="text-xs text-gray-600 mt-3">
                Your guest program has been deployed and is ready to generate proofs!
              </p>
            </div>
          </div>
        </div>
      )}

      {/* Code Preview */}
      {showCode && compiledCode && (
        <div className="flex-1 overflow-y-auto p-4">
          <div className="card">
            <div className="p-3 border-b border-gray-200 flex items-center justify-between">
              <h3 className="text-sm font-semibold text-gray-700">
                Generated Guest Program
              </h3>
              <button
                onClick={() => setShowCode(false)}
                className="text-sm text-gray-600 hover:text-gray-900"
              >
                Hide
              </button>
            </div>
            <pre className="p-4 text-xs bg-gray-50 overflow-x-auto">
              <code className="text-gray-800">{compiledCode}</code>
            </pre>
          </div>
        </div>
      )}

      {!showCode && (
        <div className="flex-1 flex items-center justify-center p-8 text-center">
          <div>
            <Code2 className="w-12 h-12 text-gray-400 mx-auto mb-4" />
            <p className="text-gray-600">
              Compile DSL to see generated code
            </p>
          </div>
        </div>
      )}
    </div>
  );
}
