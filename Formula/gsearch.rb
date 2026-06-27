class Gsearch < Formula
  desc "Standalone Google Search CLI powered by Gemini API"
  homepage "https://github.com/aeroxy/gsearch-cli"
  url "https://github.com/aeroxy/gsearch-cli/releases/download/v0.2.1/gsearch_macos_arm64.zip"
  sha256 "674c55a91f3902a05193991e454940ed4add19534a28fa6ed5fc1baeb575ab34"
  license "MIT"

  def install
    bin.install "gsearch"
  end

  test do
    assert_match "gsearch-cli #{version}", shell_output("#{bin}/gsearch --version")
  end
end
