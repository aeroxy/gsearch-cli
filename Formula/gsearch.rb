class Gsearch < Formula
  desc "Standalone Google Search CLI powered by Gemini API"
  homepage "https://github.com/aeroxy/gsearch-cli"
  url "https://github.com/aeroxy/gsearch-cli/releases/download/v0.1.1/gsearch_macos_arm64.zip"
  sha256 "cba03ef3e97f93fc92f04ee1a6e1dbd1cb45f87b108e2961d1e9efadadd1183c"
  license "MIT"

  def install
    bin.install "gsearch"
  end

  test do
    assert_match "gsearch-cli #{version}", shell_output("#{bin}/gsearch --version")
  end
end
