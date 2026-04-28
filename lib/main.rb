require_relative "reviewer"

def input(name)
  ENV.fetch("INPUT_#{name.upcase}", nil)
end

provider  = input("provider") || "anthropic"
model     = input("model") || "claude-opus-4-5"
api_key   = provider == "openai" ? input("openai_api_key") : input("anthropic_api_key")
gh_token  = input("github_token")
prompt    = input("prompt")

repo      = ENV.fetch("GITHUB_REPOSITORY")
pr_number = ENV.fetch("GITHUB_REF").match(%r{refs/pull/(\d+)/})[1].to_i

abort "Missing API key for provider: #{provider}" if api_key.nil? || api_key.empty?
abort "Missing github_token input" if gh_token.nil? || gh_token.empty?

Reviewer.new(
  provider: provider,
  model: model,
  api_key: api_key,
  github_token: gh_token,
  repo: repo,
  pr_number: pr_number,
  custom_prompt: prompt
).run
