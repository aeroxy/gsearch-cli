class Gsearch < Formula
  desc "Standalone Google Search CLI powered by Gemini API"
  homepage "https://github.com/aeroxy/gsearch-cli"
  url "https://github.com/aeroxy/gsearch-cli/releases/download/v0.1.2/gsearch_macos_arm64.zip"
  sha256 "71c7d785a11ac276038f8118eb04f7656f14796bd6f38da7784e27e47786a36c"
  license "MIT"

  def install
    bin.install "gsearch"
  end

  test do
    assert_match "gsearch-cli #{version}", shell_output("#{bin}/gsearch --version")
  end
end
