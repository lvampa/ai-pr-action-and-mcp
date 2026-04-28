require_relative "anthropic"
require_relative "openai"

module Models
  class Factory
    PROVIDERS = {
      "anthropic" => Anthropic,
      "openai" => OpenAI
    }.freeze

    def self.build(provider:, model:, api_key:)
      klass = PROVIDERS[provider.downcase]
      raise ArgumentError, "Unknown provider: #{provider}. Valid options: #{PROVIDERS.keys.join(', ')}" unless klass

      klass.new(model: model, api_key: api_key)
    end
  end
end
