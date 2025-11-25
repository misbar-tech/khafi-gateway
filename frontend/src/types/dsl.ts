// DSL Types matching the backend schema

export interface BusinessRulesDSL {
  use_case: string;
  description: string;
  version: string;
  private_inputs: InputSchema;
  public_params: ParamSchema;
  validation_rules: ValidationRule[];
  outputs?: OutputSchema;
}

export type InputSchema = ObjectSchema | Record<string, ObjectSchema>;

export interface ObjectSchema {
  type: string;
  fields: Record<string, string>;
}

export type ParamSchema = Record<string, string> | ObjectSchema;

export interface OutputSchema {
  compliance_result: string;
  [key: string]: string;
}

export type ValidationRule =
  | SignatureCheckRule
  | RangeCheckRule
  | AgeVerificationRule
  | BlacklistCheckRule
  | ArrayIntersectionCheckRule
  | CustomRule;

export interface SignatureCheckRule {
  type: 'signature_check';
  description?: string;
  field: string;
  algorithm: string;
  public_key_param: string;
  message_fields: string[];
}

export interface RangeCheckRule {
  type: 'range_check';
  description?: string;
  field: string;
  min?: number;
  max?: number;
  min_param?: string;
  max_param?: string;
}

export interface AgeVerificationRule {
  type: 'age_verification';
  description?: string;
  dob_field: string;
  min_age?: number;
  min_age_param?: string;
}

export interface BlacklistCheckRule {
  type: 'blacklist_check';
  description?: string;
  field: string;
  blacklist_param: string;
}

export interface ArrayIntersectionCheckRule {
  type: 'array_intersection_check';
  description?: string;
  field: string;
  prohibited_param: string;
  must_be_empty?: boolean;
}

export interface CustomRule {
  type: 'custom';
  description?: string;
  code: string;
}

// API Response Types

export interface ValidateResponse {
  valid: boolean;
  error?: string;
  parsed_dsl?: BusinessRulesDSL;
}

export interface CompileResponse {
  success: boolean;
  code?: string;
  error?: string;
}

export interface GenerateSdkResponse {
  success: boolean;
  sdk_id?: string;
  error?: string;
}

export interface TemplateInfo {
  name: string;
  title: string;
  description: string;
  category: string;
}

export interface TemplatesResponse {
  templates: TemplateInfo[];
}
