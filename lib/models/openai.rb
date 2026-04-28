require "openai"
require_relative "base"

module Models
  class OpenAI < Base
    def review(prompt)
      client = ::OpenAI::Client.new(access_token: api_key)
      response = client.chat(
        parameters: {
          model: model,
          max_tokens: 4096,
          messages: [{ role: "user", content: prompt }]
        }
      )
      response.dig("choices", 0, "message", "content")
    end
  end
end
