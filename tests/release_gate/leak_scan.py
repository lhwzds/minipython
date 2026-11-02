"""Scan a release candidate for leaked private data.

usage: leak_scan.py [--repo DIR] [--allowlist FILE] [--since REF] [--json OUT]

Scans every tracked file (`git ls-files`) and, for every commit since the
last release tag (or since the root commit when there is no tag), the commit
message and the author and committer identities. It looks for:

- absolute home paths (`/Users/<name>`, `/home/<name>`, `C:\\Users\\<name>`)
- `/Volumes/...`, `/private/tmp`, `/private/var`
- host names of private networks (`*.ts.net` tailnet names, `*.local`,
  `*.lan`, `*.internal`, `*.home.arpa`)
- IPv4 addresses outside loopback (127/8), the unspecified address, and the
  documentation ranges (192.0.2/24, 198.51.100/24, 203.0.113/24); and IPv6
  addresses outside ::1 and the documentation range 2001:db8::/32
- email addresses other than the author's and documentation domains
  (example.com/.org/.net, *.example, *.invalid, *.test, localhost)
- key and token shapes (GitHub, npm, Slack, AWS, Google, OpenAI/Anthropic
  style `sk-` keys, JWTs, private-key blocks, and long values assigned to
  names like api_key/token/secret/password)

Every finding must match an entry of the allowlist (a TOML file with
`[[allow]]` tables holding `rule`, `path` glob, `match` regex, and `reason`)
or the scan fails. Exit status: 0 clean, 1 findings, 2 usage error.
"""

from __future__ import annotations

import argparse
import fnmatch
import ipaddress
import json
import re
import subprocess
import sys
import tomllib
from dataclasses import asdict, dataclass
from pathlib import Path

AUTHOR_EMAILS = {"lhwzds@gmail.com"}
EXAMPLE_EMAIL_DOMAIN = re.compile(
    r"(^|\.)(example\.(com|org|net)|[a-z0-9-]+\.example|[a-z0-9-]+\.invalid|[a-z0-9-]+\.test|localhost)$",
    re.I,
)

RULES: list[tuple[str, re.Pattern[str]]] = [
    ("home-path", re.compile(r"(?<![\w.])/(?:Users|home)/(?!\$|\{|<|\*|\.\.\.)[A-Za-z0-9._-]+")),
    ("home-path", re.compile(r"[A-Za-z]:\\\\?Users\\\\?[A-Za-z0-9._-]+")),
    ("volume-path", re.compile(r"(?<![\w.])/Volumes/[A-Za-z0-9._ -]+")),
    ("private-tmp", re.compile(r"(?<![\w.])/private/(?:tmp|var)\b")),
    ("hostname", re.compile(r"\b[a-z0-9](?:[a-z0-9-]*[a-z0-9])?(?:\.[a-z0-9-]+)*\.(?:ts\.net|local|lan|internal|home\.arpa)\b", re.I)),
    ("ipv4", re.compile(r"(?<![\w.])(?:\d{1,3}\.){3}\d{1,3}(?!\w)(?!\.\d)")),
    ("ipv6", re.compile(r"(?<![\w:.])[0-9a-f]{0,4}(?::[0-9a-f]{0,4}){2,7}(?![\w:])", re.I)),
    ("email", re.compile(r"(?<![\\\w.%+-])[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}\b")),
    ("token", re.compile(r"\b(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{40,})\b")),
    ("token", re.compile(r"\bnpm_[A-Za-z0-9]{30,}\b")),
    ("token", re.compile(r"\bxox[abposr]-[A-Za-z0-9-]{10,}\b")),
    ("token", re.compile(r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b")),
    ("token", re.compile(r"\bAIza[0-9A-Za-z_-]{35}\b")),
    ("token", re.compile(r"\bsk-(?:ant-|proj-)?[A-Za-z0-9_-]{20,}\b")),
    ("token", re.compile(r"\beyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}")),
    ("token", re.compile(r"-----BEGIN (?:[A-Z]+ )?PRIVATE KEY-----")),
    (
        "token",
        re.compile(
            r"(?i)\b(?:api[_-]?key|access[_-]?key|secret|token|passw(?:or)?d)\b[\"']?\s*[:=]\s*[\"'][A-Za-z0-9/+_=.-]{20,}[\"']"
        ),
    ),
]


@dataclass
class Finding:
    rule: str
    where: str  # "file" or "commit"
    path: str  # file path, or commit sha
    line: int
    match: str
    context: str


def ip_is_allowed(text: str, rule: str) -> bool:
    try:
        address = ipaddress.ip_address(text)
    except ValueError:
        return True  # not an address (version numbers like 1.2.3.400)
    if rule == "ipv4":
        if address.is_loopback or address.is_unspecified:
            return True
        for network in ("192.0.2.0/24", "198.51.100.0/24", "203.0.113.0/24", "255.255.255.255/32"):
            if address in ipaddress.ip_network(network):
                return True
        return False
    if address.is_loopback or address.is_unspecified:
        return True
    return address in ipaddress.ip_network("2001:db8::/32")


def email_is_allowed(text: str) -> bool:
    if text.lower() in AUTHOR_EMAILS:
        return True
    domain = text.rsplit("@", 1)[1]
    return bool(EXAMPLE_EMAIL_DOMAIN.search(domain))


def scan_text(text: str, where: str, path: str) -> list[Finding]:
    findings = []
    for number, line in enumerate(text.splitlines(), 1):
        for rule, pattern in RULES:
            for match in pattern.finditer(line):
                value = match.group(0)
                if rule in ("ipv4", "ipv6") and ip_is_allowed(value, rule):
                    continue
                if rule == "ipv6" and not any(len(group) >= 3 for group in value.split(":")):
                    continue  # slices such as x[1::2], not addresses
                if rule == "email" and email_is_allowed(value):
                    continue
                findings.append(Finding(rule, where, path, number, value, line.strip()[:200]))
    return findings


def git(repo: Path, *args: str) -> str:
    return subprocess.run(["git", *args], cwd=repo, capture_output=True, text=True, check=True).stdout


def commit_range(repo: Path, since: str | None) -> tuple[str, str]:
    if since:
        return f"{since}..HEAD", since
    tags = subprocess.run(
        ["git", "describe", "--tags", "--abbrev=0", "--match", "v*"],
        cwd=repo,
        capture_output=True,
        text=True,
    )
    if tags.returncode == 0 and tags.stdout.strip():
        tag = tags.stdout.strip()
        return f"{tag}..HEAD", tag
    return "HEAD", "root"


def load_allowlist(path: Path) -> list[dict]:
    if not path.exists():
        return []
    entries = tomllib.loads(path.read_text()).get("allow", [])
    for entry in entries:
        for key in ("rule", "path", "match", "reason"):
            if not entry.get(key):
                raise SystemExit(f"allowlist entry {entry} lacks {key!r}")
        entry["_re"] = re.compile(entry["match"])
        entry["_used"] = 0
    return entries


def allowed(finding: Finding, allowlist: list[dict], allowlist_file: str = "") -> bool:
    target = finding.path if finding.where == "file" else f"commit:{finding.path}"
    if finding.where == "file" and target == allowlist_file:
        # The allowlist names the values it accepts; such a value is allowed
        # here only if an entry for the same rule matches it.
        return any(entry["rule"] == finding.rule and entry["_re"].search(finding.match) for entry in allowlist)
    for entry in allowlist:
        if entry["rule"] != finding.rule:
            continue
        if not fnmatch.fnmatch(target, entry["path"]):
            continue
        if entry["_re"].search(finding.match):
            entry["_used"] += 1
            return True
    return False


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--repo", default=".")
    parser.add_argument("--allowlist", default=None)
    parser.add_argument("--since", default=None, help="Scan commits after this ref instead of the last v* tag.")
    parser.add_argument("--json", default=None)
    args = parser.parse_args()
    repo = Path(args.repo).resolve()
    allowlist_path = Path(args.allowlist) if args.allowlist else repo / "tests/release_gate/leak-allowlist.toml"
    allowlist = load_allowlist(allowlist_path)

    findings: list[Finding] = []
    files = [name for name in git(repo, "ls-files", "-z").split("\0") if name]
    for name in files:
        findings += scan_text(name, "file", name)  # the path itself
        path = repo / name
        if path.is_symlink() or not path.is_file():
            continue
        data = path.read_bytes()
        if b"\0" in data[:8192]:
            continue  # binary
        findings += scan_text(data.decode("utf-8", errors="replace"), "file", name)

    revisions, base = commit_range(repo, args.since)
    log = git(repo, "log", "--format=%x1e%H%x1f%an <%ae>%x1f%cn <%ce>%x1f%B", revisions)
    commits = 0
    for record in log.split("\x1e"):
        if not record.strip():
            continue
        commits += 1
        sha, author, committer, body = record.split("\x1f", 3)
        for identity in (author, committer):
            email = identity.rsplit("<", 1)[-1].rstrip(">")
            if not email_is_allowed(email):
                findings.append(Finding("email", "commit", sha, 0, email, f"identity {identity}"))
        findings += scan_text(body, "commit", sha)

    try:
        allowlist_file = str(allowlist_path.resolve().relative_to(repo))
    except ValueError:
        allowlist_file = ""
    unallowed = [finding for finding in findings if not allowed(finding, allowlist, allowlist_file)]
    unused = [
        {"rule": entry["rule"], "path": entry["path"], "match": entry["match"]}
        for entry in allowlist
        if entry["_used"] == 0
    ]
    payload = {
        "files_scanned": len(files),
        "commits_scanned": commits,
        "commit_base": base,
        "findings": len(findings),
        "allowed": len(findings) - len(unallowed),
        "unallowed": [asdict(finding) for finding in unallowed],
        "unused_allowlist_entries": unused,
        "passed": not unallowed and not unused,
    }
    if args.json:
        Path(args.json).write_text(json.dumps(payload, indent=2) + "\n")
    print(
        f"leak scan: {len(files)} tracked files, {commits} commits since {base}: "
        f"{len(findings)} findings, {payload['allowed']} allowlisted, {len(unallowed)} unallowed, "
        f"{len(unused)} unused allowlist entries"
    )
    for finding in unallowed[:200]:
        location = f"{finding.path}:{finding.line}" if finding.where == "file" else f"commit {finding.path[:12]}"
        print(f"  {finding.rule}: {location}: {finding.match!r}  | {finding.context}")
    for entry in unused:
        print(f"  unused allowlist entry (remove it): {entry}")
    return 0 if payload["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
