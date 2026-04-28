require "anthropic"
require_relative "base"

module Models
  class Anthropic < Base
    def review(prompt)
      client = ::Anthropic::Client.new(access_token: api_key)
      response = client.messages(
        parameters: {
          model: model,
          max_tokens: 4096,
          messages: [{ role: "user", content: prompt }]
        }
      )
      response.dig("content", 0, "text")
    end
  end
end
