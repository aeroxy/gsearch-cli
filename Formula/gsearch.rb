class Gsearch < Formula
  desc "Standalone Google Search CLI powered by Gemini API"
  homepage "https://github.com/aeroxy/gsearch-cli"
  url "https://github.com/aeroxy/gsearch-cli/releases/download/v0.1.1/gsearch_macos_arm64.zip"
  sha256 "e5115eb189dda57c29d8c1522c8de6bc455712831eaa9d3f3c8141d2cafda7a2"
  license "MIT"

  def install
    bin.install "gsearch"
  end

  test do
    assert_match "gsearch-cli #{version}", shell_output("#{bin}/gsearch --version")
  end
end
