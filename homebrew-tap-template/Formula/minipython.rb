class Minipython < Formula
  desc "Sandbox for untrusted Python code, aligned with CPython 3.14.6"
  homepage "https://github.com/lhwzds/minipython"
  version "0.1.0"
  license all_of: ["MIT", "PSF-2.0"]

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/lhwzds/minipython/releases/download/v#{version}/minipython-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_ARM_MAC"
    else
      url "https://github.com/lhwzds/minipython/releases/download/v#{version}/minipython-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_X86_MAC"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/lhwzds/minipython/releases/download/v#{version}/minipython-aarch64-unknown-linux-musl.tar.gz"
      sha256 "PLACEHOLDER_SHA256_ARM_LINUX"
    else
      url "https://github.com/lhwzds/minipython/releases/download/v#{version}/minipython-x86_64-unknown-linux-musl.tar.gz"
      sha256 "PLACEHOLDER_SHA256_X86_LINUX"
    end
  end

  def install
    bin.install "mnpy"
  end

  test do
    assert_match "mnpy #{version}", shell_output("#{bin}/mnpy --version")
    assert_equal "3\n", shell_output("#{bin}/mnpy -c 'print(1 + 2)'")
    assert_match "No module named 'os'", shell_output("#{bin}/mnpy -c 'import os' 2>&1", 1)
  end
end
