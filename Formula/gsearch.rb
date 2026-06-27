class Gsearch < Formula
  desc "Standalone Google Search CLI powered by Gemini API"
  homepage "https://github.com/aeroxy/gsearch-cli"
  url "https://github.com/aeroxy/gsearch-cli/releases/download/v0.2.0/gsearch_macos_arm64.zip"
  sha256 "a9ffc077551a7420de9c50477dc292eac1b8ac7aa41fb0211d2b89df29c32be9"
  license "MIT"

  def install
    bin.install "gsearch"
  end

  test do
    assert_match "gsearch-cli #{version}", shell_output("#{bin}/gsearch --version")
  end
end
