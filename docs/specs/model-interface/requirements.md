# Feature 2 — Provider-Agnostic Model Interface

## Overview

A trait-based abstraction over AI model providers. Anthropic and OpenAI implementations. A factory resolves the correct implementation from config.

## Requirements

### Requirement 1: Model Trait

**User Story:** As the review system, I want a common interface for all AI providers so that the two-pass review logic is decoupled from any specific provider SDK.

#### Acceptance Criteria

1. The system SHALL define a `ModelProvider` trait with at least a `complete(prompt: &str) -> Result<String>` method (or async equivalent).
2. WHEN a provider implementation is called with a prompt, it SHALL return the model's text response as a `String`.
3. WHEN the provider API returns an error, the implementation SHALL return a descriptive `anyhow::Error` including the provider name and HTTP status if available.

---

### Requirement 2: Anthropic Implementation

**User Story:** As the review system, I want an Anthropic implementation of the model trait so that Claude models can be used for both review passes.

#### Acceptance Criteria

1. The Anthropic implementation SHALL send requests to the Anthropic Messages API using the model name from config.
2. The Anthropic implementation SHALL read the API key from the `INPUT_ANTHROPIC_API_KEY` environment variable.
3. WHEN `INPUT_ANTHROPIC_API_KEY` is absent or empty, the implementation SHALL return a descriptive error before making any API call.
4. WHEN the Anthropic API returns a rate limit error (HTTP 429), the implementation SHALL retry up to `rate_limiting.retries` times with `rate_limiting.backoff_seconds` delay between attempts.
5. WHEN all retries are exhausted, the implementation SHALL return an error indicating the number of attempts made.

---

### Requirement 3: OpenAI Implementation

**User Story:** As the review system, I want an OpenAI implementation of the model trait so that GPT models can be used for both review passes.

#### Acceptance Criteria

1. The OpenAI implementation SHALL send requests to the OpenAI Chat Completions API using the model name from config.
2. The OpenAI implementation SHALL read the API key from the `INPUT_OPENAI_API_KEY` environment variable.
3. WHEN `INPUT_OPENAI_API_KEY` is absent or empty, the implementation SHALL return a descriptive error before making any API call.
4. WHEN the OpenAI API returns a rate limit error (HTTP 429), the implementation SHALL retry up to `rate_limiting.retries` times with `rate_limiting.backoff_seconds` delay between attempts.
5. WHEN all retries are exhausted, the implementation SHALL return an error indicating the number of attempts made.

---

### Requirement 4: Factory Resolution

**User Story:** As the review system, I want a factory that resolves the correct provider implementation from config so that consumers don't need to instantiate providers directly.

#### Acceptance Criteria

1. The system SHALL expose a `create_provider(config: &ModelConfig) -> Result<Box<dyn ModelProvider>>` factory function.
2. WHEN `provider` is `anthropic`, the factory SHALL return an Anthropic implementation.
3. WHEN `provider` is `openai`, the factory SHALL return an OpenAI implementation.
4. WHEN `provider` is any other value, the factory SHALL return a descriptive error listing the accepted values.
