require "octokit"

class GitHub
  def initialize(token:, repo:, pr_number:)
    @client = Octokit::Client.new(access_token: token)
    @repo = repo
    @pr_number = pr_number
  end

  def diff
    @client.pull_request(@repo, @pr_number, accept: "application/vnd.github.v3.diff")
  end

  def post_comment(body)
    @client.add_comment(@repo, @pr_number, body)
  end
end
