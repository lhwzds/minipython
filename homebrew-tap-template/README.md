# Homebrew tap template

`Formula/minipython.rb` is the formula for the `lhwzds/homebrew-tap`
repository (`brew install lhwzds/tap/minipython`, which installs `mnpy`). It
downloads the release archives `minipython-<target>.tar.gz` for macOS (arm64,
x86-64) and Linux (musl, arm64 and x86-64) and checks their SHA-256. Its test
block runs `mnpy --version`, a print, and a blocked `import os`.

The release gate (`tests/run.sh --release-gate`, step e) renders this template
for a locally built archive, installs it from a throwaway local tap, runs
`brew test` and `brew audit --strict`, and removes it again. It never touches
the real tap.

## What the tap needs

These are changes to the `lhwzds/homebrew-tap` repository, made there by its
owner; nothing in this repository pushes to the tap.

1. After the first release (`v0.1.0`) is published, add
   `Formula/minipython.rb`: this template with the release version and the
   four SHA-256 values from the release's `checksums.txt` in place of the
   `PLACEHOLDER_SHA256_*` strings. Check it with
   `brew audit --strict --online lhwzds/tap/minipython` and
   `brew install lhwzds/tap/minipython && brew test lhwzds/tap/minipython`.
2. Add an `update-minipython` job to `.github/workflows/update-formula.yml`
   next to `update-codocia`, so the scheduled run (every six hours) and the
   manual `workflow_dispatch` also pick up new MiniPython releases. Add it
   only once a release exists: before that, `gh release view` fails and the
   job would fail on every run. The job below mirrors `update-codocia`: it
   accepts only a plain version, requires exactly one 64-character hash per
   archive, and edits the version and hashes of the committed formula in
   place, so the formula's test block lives in one file.

```yaml
  update-minipython:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Get version
        id: version
        env:
          GH_TOKEN: ${{ github.token }}
          REQUESTED_VERSION: ${{ inputs.version }}
        run: |
          if [ -n "$REQUESTED_VERSION" ]; then
            VERSION="$REQUESTED_VERSION"
          else
            TAG=$(gh release view --repo lhwzds/minipython --json tagName --jq .tagName)
            if [[ "$TAG" != v* ]]; then
              echo "::error::Latest release tag '$TAG' does not start with v"
              exit 1
            fi
            VERSION="$TAG"
          fi
          VERSION="${VERSION#v}"
          if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.]+)?$ ]]; then
            echo "::error::Invalid version: '$VERSION'"
            exit 1
          fi
          CURRENT=$(grep -E '^[[:space:]]*version[[:space:]]+"' Formula/minipython.rb | head -1 | sed -E 's/.*"([^"]+)".*/\1/')
          echo "VERSION=$VERSION" >> "$GITHUB_OUTPUT"
          if [ "$VERSION" = "$CURRENT" ]; then
            echo "minipython formula already at $VERSION"
            echo "UPDATED=false" >> "$GITHUB_OUTPUT"
          else
            echo "Updating minipython formula: ${CURRENT:-unknown} -> $VERSION"
            echo "UPDATED=true" >> "$GITHUB_OUTPUT"
          fi

      - name: Download checksums
        if: steps.version.outputs.UPDATED == 'true'
        env:
          VERSION: ${{ steps.version.outputs.VERSION }}
        run: |
          curl -fsSL "https://github.com/lhwzds/minipython/releases/download/v${VERSION}/checksums.txt" -o "$RUNNER_TEMP/checksums.txt"
          cat "$RUNNER_TEMP/checksums.txt"

      - name: Update formula
        if: steps.version.outputs.UPDATED == 'true'
        env:
          VERSION: ${{ steps.version.outputs.VERSION }}
        run: |
          python3 - "$RUNNER_TEMP/checksums.txt" <<'PY'
          import os, re, sys
          version = os.environ["VERSION"]
          sums = {}
          for line in open(sys.argv[1]):
              parts = line.split()
              if len(parts) == 2:
                  sums.setdefault(parts[1].lstrip("*"), []).append(parts[0])
          path = "Formula/minipython.rb"
          text = open(path).read()
          text, count = re.subn(r'(?m)^(\s*version\s+)"[^"]+"', r'\g<1>"%s"' % version, text, count=1)
          assert count == 1, "no version line"
          def replace(match):
              name = match.group(2)
              hashes = sums.get(name, [])
              if len(hashes) != 1 or not re.fullmatch(r"[0-9a-f]{64}", hashes[0]):
                  sys.exit(f"expected one SHA256 for {name} in checksums.txt")
              return f'{match.group(1)}{name}"\n{match.group(3)}sha256 "{hashes[0]}"'
          text, count = re.subn(
              r'(url "https://github\.com/lhwzds/minipython/releases/download/v#\{version\}/)(minipython-[\w.-]+\.tar\.gz)"\n(\s*)sha256 "[^"]*"',
              replace, text)
          assert count == 4, f"expected 4 url/sha256 pairs, found {count}"
          open(path, "w").write(text)
          PY
          git diff

      - name: Commit and push
        if: steps.version.outputs.UPDATED == 'true'
        env:
          VERSION: ${{ steps.version.outputs.VERSION }}
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add Formula/minipython.rb
          if git diff --cached --quiet; then
            echo "No formula changes detected."
            exit 0
          fi
          git commit -m "Update minipython to $VERSION"
          git push
```

If both jobs run in the same workflow run, the second push can race the
first; give the jobs `needs:` (or a `concurrency:` group) so they push one
after the other.
