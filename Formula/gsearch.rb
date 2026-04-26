class Gsearch < Formula
  desc "Standalone Google Search CLI powered by Gemini API"
  homepage "https://github.com/aeroxy/gsearch-cli"
  url "https://github.com/aeroxy/gsearch-cli/releases/download/v0.1.0/gsearch_macos_arm64.zip"
  sha256 "87d9090670ae6dd32625958f7cbcc31e7e6fa4fac8bc34659b49c34db7d95e50"
  license "MIT"

  def install
    bin.install "gsearch"
  end

  test do
    assert_match "Usage: gsearch", shell_output("#{bin}/gsearch --help")
  end
end
