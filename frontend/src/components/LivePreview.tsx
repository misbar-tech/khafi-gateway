import { useState, useEffect } from 'react';
import {
  CheckCircle2,
  XCircle,
  AlertCircle,
  Download,
  Loader2,
  Eye,
  Code2,
  Rocket,
  Copy,
  ExternalLink,
  Zap,
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
  const [copied, setCopied] = useState<string | null>(null);

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
      }
    } catch (err) {
      console.error('Compilation failed:', err);
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
        await api.downloadSDK(result.sdk_id);
      }
    } catch (err) {
      console.error('SDK generation failed:', err);
    } finally {
      setGenerating(false);
    }
  }

  async function deployToGateway() {
    if (!dsl) return;

    const customerId = `customer-${dsl.use_case.toLowerCase().replace(/\s+/g, '-')}-${Date.now()}`;

    try {
      setDeploying(true);
      setDeploymentResult(null);
      const result = await api.deployDSL(dsl, customerId);
      setDeploymentResult(result);
    } catch (err) {
      console.error('Deployment failed:', err);
    } finally {
      setDeploying(false);
    }
  }

  function copyToClipboard(text: string, field: string) {
    navigator.clipboard.writeText(text);
    setCopied(field);
    setTimeout(() => setCopied(null), 2000);
  }

  const isValid = validationResult?.valid ?? false;

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="p-4 border-b border-dark-800/50">
        <div className="flex items-center space-x-2 mb-4">
          <Eye className="w-5 h-5 text-primary-400" />
          <h2 className="font-semibold text-white">Live Preview</h2>
        </div>

        {/* Validation Status Card */}
        <div className="rounded-xl bg-dark-800/30 border border-dark-700/50 p-4 mb-4">
          <h3 className="text-xs font-medium text-dark-400 uppercase tracking-wider mb-3">
            Validation Status
          </h3>

          {loading && (
            <div className="flex items-center text-dark-400">
              <Loader2 className="w-5 h-5 animate-spin mr-2 text-primary-400" />
              Validating...
            </div>
          )}

          {!loading && !validationResult && !dsl && (
            <div className="flex items-center text-dark-500">
              <AlertCircle className="w-5 h-5 mr-2" />
              No DSL to validate
            </div>
          )}

          {!loading && validationResult && (
            <div>
              {isValid ? (
                <div className="flex items-start">
                  <div className="w-8 h-8 rounded-lg bg-success-500/20 flex items-center justify-center mr-3 flex-shrink-0">
                    <CheckCircle2 className="w-4 h-4 text-success-400" />
                  </div>
                  <div>
                    <p className="font-medium text-success-400">DSL is valid</p>
                    <div className="flex flex-wrap gap-2 mt-2">
                      <span className="badge badge-success text-xs">
                        {validationResult.parsed_dsl?.use_case}
                      </span>
                      <span className="badge bg-dark-700/50 text-dark-300 border border-dark-600/50 text-xs">
                        {validationResult.parsed_dsl?.validation_rules.length || 0} rules
                      </span>
                    </div>
                  </div>
                </div>
              ) : (
                <div className="flex items-start">
                  <div className="w-8 h-8 rounded-lg bg-error-500/20 flex items-center justify-center mr-3 flex-shrink-0">
                    <XCircle className="w-4 h-4 text-error-400" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="font-medium text-error-400">Validation failed</p>
                    {validationResult.error && (
                      <p className="text-xs text-error-400/70 mt-1 font-mono break-words">
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
            className="w-full btn btn-secondary flex items-center justify-center"
          >
            {compiling ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin mr-2" />
                Compiling...
              </>
            ) : (
              <>
                <Code2 className="w-4 h-4 mr-2" />
                Preview Generated Code
              </>
            )}
          </button>

          <button
            onClick={deployToGateway}
            disabled={!isValid || deploying}
            className="w-full btn btn-primary flex items-center justify-center group"
          >
            {deploying ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin mr-2" />
                Deploying to Gateway...
              </>
            ) : (
              <>
                <Rocket className="w-4 h-4 mr-2 group-hover:animate-bounce" />
                Deploy to Gateway
              </>
            )}
          </button>

          <button
            onClick={generateAndDownloadSDK}
            disabled={!isValid || generating}
            className="w-full btn btn-ghost flex items-center justify-center text-xs"
          >
            {generating ? (
              <>
                <Loader2 className="w-3.5 h-3.5 animate-spin mr-1.5" />
                Generating...
              </>
            ) : (
              <>
                <Download className="w-3.5 h-3.5 mr-1.5" />
                Download SDK (Legacy)
              </>
            )}
          </button>
        </div>
      </div>

      {/* Deployment Result */}
      {deploymentResult && deploymentResult.success && (
        <div className="p-4 border-b border-dark-800/50 animate-slide-up">
          <div className="rounded-xl bg-primary-500/10 border border-primary-500/30 p-4">
            <div className="flex items-start mb-4">
              <div className="w-10 h-10 rounded-xl bg-primary-500/20 flex items-center justify-center mr-3 flex-shrink-0">
                <Loader2 className="w-5 h-5 text-primary-400 animate-spin" />
              </div>
              <div>
                <p className="font-semibold text-primary-400">Build Queued!</p>
                <p className="text-xs text-primary-400/70 mt-0.5">
                  Your ZK circuit is being built. This may take a few minutes.
                </p>
              </div>
            </div>

            <div className="space-y-3">
              {/* Job ID */}
              {deploymentResult.job_id && (
                <div className="rounded-lg bg-dark-900/50 p-3">
                  <div className="flex items-center justify-between mb-1">
                    <span className="text-xs text-dark-400">Build Job ID</span>
                    <button
                      onClick={() => copyToClipboard(deploymentResult.job_id || '', 'job_id')}
                      className="text-dark-400 hover:text-white transition-colors"
                    >
                      {copied === 'job_id' ? (
                        <CheckCircle2 className="w-3.5 h-3.5 text-success-400" />
                      ) : (
                        <Copy className="w-3.5 h-3.5" />
                      )}
                    </button>
                  </div>
                  <code className="text-xs text-accent-300 font-mono break-all">
                    {deploymentResult.job_id}
                  </code>
                </div>
              )}

              {/* Customer ID */}
              <div className="rounded-lg bg-dark-900/50 p-3">
                <div className="flex items-center justify-between mb-1">
                  <span className="text-xs text-dark-400">Customer ID</span>
                  <button
                    onClick={() => copyToClipboard(deploymentResult.customer_id || '', 'customer_id')}
                    className="text-dark-400 hover:text-white transition-colors"
                  >
                    {copied === 'customer_id' ? (
                      <CheckCircle2 className="w-3.5 h-3.5 text-success-400" />
                    ) : (
                      <Copy className="w-3.5 h-3.5" />
                    )}
                  </button>
                </div>
                <code className="text-xs text-primary-300 font-mono break-all">
                  {deploymentResult.customer_id}
                </code>
              </div>

              {/* API Endpoint */}
              <div className="rounded-lg bg-dark-900/50 p-3">
                <div className="flex items-center justify-between mb-1">
                  <span className="text-xs text-dark-400">API Endpoint</span>
                  <div className="flex items-center space-x-2">
                    <button
                      onClick={() => copyToClipboard(deploymentResult.api_endpoint || '', 'api_endpoint')}
                      className="text-dark-400 hover:text-white transition-colors"
                    >
                      {copied === 'api_endpoint' ? (
                        <CheckCircle2 className="w-3.5 h-3.5 text-success-400" />
                      ) : (
                        <Copy className="w-3.5 h-3.5" />
                      )}
                    </button>
                    <a
                      href={deploymentResult.api_endpoint}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-dark-400 hover:text-white transition-colors"
                    >
                      <ExternalLink className="w-3.5 h-3.5" />
                    </a>
                  </div>
                </div>
                <code className="text-xs text-white font-mono break-all">
                  {deploymentResult.api_endpoint}
                </code>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Code Preview */}
      {showCode && compiledCode && (
        <div className="flex-1 overflow-hidden flex flex-col">
          <div className="p-3 border-b border-dark-800/50 flex items-center justify-between">
            <div className="flex items-center space-x-2">
              <Zap className="w-4 h-4 text-warning-400" />
              <h3 className="text-sm font-medium text-white">Generated Guest Program</h3>
              <span className="badge badge-warning text-xs">Rust</span>
            </div>
            <button
              onClick={() => setShowCode(false)}
              className="text-xs text-dark-400 hover:text-white transition-colors"
            >
              Hide
            </button>
          </div>
          <div className="flex-1 overflow-auto bg-dark-950 p-4">
            <pre className="text-xs font-mono text-dark-200 whitespace-pre-wrap">
              <code>{compiledCode}</code>
            </pre>
          </div>
        </div>
      )}

      {!showCode && !deploymentResult?.success && (
        <div className="flex-1 flex items-center justify-center p-8 text-center">
          <div>
            <div className="w-16 h-16 rounded-2xl bg-dark-800/30 flex items-center justify-center mx-auto mb-4">
              <Code2 className="w-8 h-8 text-dark-600" />
            </div>
            <p className="text-dark-500 text-sm">
              Compile DSL to preview generated code
            </p>
            <p className="text-dark-600 text-xs mt-1">
              or deploy directly to the gateway
            </p>
          </div>
        </div>
      )}
    </div>
  );
}
