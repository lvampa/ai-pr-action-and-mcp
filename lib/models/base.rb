module Models
  class Base
    def initialize(model:, api_key:)
      @model = model
      @api_key = api_key
    end

    def review(prompt)
      raise NotImplementedError, "#{self.class} must implement #review"
    end

    private

    attr_reader :model, :api_key
  end
end
