"""Unit tests for the release gate's leak scanner (run by gate step d).

The leak-shaped strings are assembled from pieces so this file does not
itself trip the scan.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import leak_scan  # noqa: E402


def rules(text: str) -> list[tuple[str, str]]:
    return [(finding.rule, finding.match) for finding in leak_scan.scan_text(text, "file", "sample")]


class LeakScanTests(unittest.TestCase):
    def test_flags_private_paths(self):
        text = "/" + "Users/alice/src /" + "home/bob /" + "Volumes/backup /" + "private/tmp/x"
        self.assertEqual(
            [rule for rule, _ in rules(text)],
            ["home-path", "home-path", "volume-path", "private-tmp"],
        )

    def test_ignores_placeholder_paths(self):
        self.assertEqual(rules("/" + "Users/<name> and /" + "home/$USER"), [])

    def test_flags_private_hosts_and_addresses(self):
        text = "studio.tail0." + "ts.net mini." + "local 10.1." + "2.3 fd7a:115c:" + "a1" + "e0::53"
        self.assertEqual(
            [rule for rule, _ in rules(text)],
            ["hostname", "hostname", "ipv4", "ipv6"],
        )

    def test_allows_loopback_documentation_and_version_numbers(self):
        self.assertEqual(rules("127.0.0.1 192.0.2.7 198.51.100.1 203.0.113.9 ::1 2001:db8::1 3.14.6"), [])

    def test_allows_python_slices(self):
        self.assertEqual(rules("x[1::2] a[::3]"), [])

    def test_emails_other_than_the_author_and_examples(self):
        address = "ann@" + "corp.io"
        self.assertEqual(rules(address), [("email", address)])
        self.assertEqual(rules("lhwzds@gmail.com ann@example.com bob@" + "mail.invalid"), [])
        self.assertEqual(rules("\\n@functools.cache"), [])

    def test_flags_token_shapes(self):
        samples = [
            "gh" + "p_" + "a" * 36,
            "np" + "m_" + "b" * 36,
            "xo" + "xb-" + "1" * 12,
            "AK" + "IA" + "ABCDEFGHIJKLMNOP",
            "s" + "k-" + "c" * 24,
            "-----BEGIN " + "PRIVATE KEY-----",
            "api" + "_key = '" + "d" * 24 + "'",
        ]
        for sample in samples:
            self.assertEqual([rule for rule, _ in rules(sample)], ["token"], sample)

    def test_allowlist_file_may_name_only_allowed_values(self):
        entry = {"rule": "token", "path": "src/x.rs", "match": "^s" + "k-fake-[a-z-]+$", "reason": "sample"}
        entry["_re"] = __import__("re").compile(entry["match"])
        entry["_used"] = 0
        named = leak_scan.Finding("token", "file", "allow.toml", 3, "s" + "k-fake-value-for-tests", "")
        other = leak_scan.Finding("token", "file", "allow.toml", 4, "s" + "k-" + "z" * 24, "")
        self.assertTrue(leak_scan.allowed(named, [entry], "allow.toml"))
        self.assertFalse(leak_scan.allowed(other, [entry], "allow.toml"))

    def test_allowlist_requires_rule_path_and_match(self):
        address = "a@" + "b.co"
        finding = leak_scan.Finding("email", "file", "tests/x.rs", 1, address, "")
        entry = {"rule": "email", "path": "tests/*.rs", "match": "^a@b\\.co$", "reason": "sample"}
        entry["_re"] = __import__("re").compile(entry["match"])
        entry["_used"] = 0
        self.assertTrue(leak_scan.allowed(finding, [entry]))
        other = leak_scan.Finding("email", "file", "src/x.rs", 1, address, "")
        self.assertFalse(leak_scan.allowed(other, [entry]))


if __name__ == "__main__":
    unittest.main()
