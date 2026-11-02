# SPDX-License-Identifier: MIT

"""A skill-shaped text tool built on `re` (plus argparse, json, and sys).

Usage examples:

    mnpy examples/sandbox/re_text_tool.py extract "Mail ann@example.com by 2026-10-29"
    mnpy examples/sandbox/re_text_tool.py redact "card 4111 1111 1111 1111, pin 1234"
    mnpy examples/sandbox/re_text_tool.py grep "err(or)?" "ok" "Error: disk" "warning" -i
    mnpy examples/sandbox/re_text_tool.py config "name = demo" "retries=3" "# comment"
"""

import argparse
import json
import re
import sys

EMAIL = re.compile(r"[\w.+-]+@[\w-]+(?:\.[\w-]+)+")
URL = re.compile(r"https?://[^\s<>\"']+")
DATE = re.compile(r"\b(?P<year>\d{4})-(?P<month>\d{2})-(?P<day>\d{2})\b")
CONFIG_LINE = re.compile(
    r"""
    ^\s*
    (?P<key>[A-Za-z_][\w.-]*)   # key
    \s*[=:]\s*
    (?P<value>.*?)              # value, without trailing spaces
    \s*$
    """,
    re.VERBOSE,
)


def extract(text):
    return {
        "emails": EMAIL.findall(text),
        "urls": [url.rstrip(".,;)") for url in URL.findall(text)],
        "dates": [match.groupdict() for match in DATE.finditer(text)],
        "words": len(re.findall(r"\b\w+\b", text)),
    }


def redact(text):
    def mask(match):
        digits = re.sub(r"\D", "", match.group())
        return "*" * (len(digits) - 4) + digits[-4:] if len(digits) > 4 else "****"

    redacted, count = re.subn(r"\b(?:\d[ -]?){4,19}\b", mask, text)
    return {"text": re.sub(r"\s+", " ", redacted).strip(), "redactions": count}


def grep(pattern, lines, ignore_case):
    try:
        compiled = re.compile(pattern, re.IGNORECASE if ignore_case else 0)
    except re.error as error:
        print(f"re_text_tool: bad pattern: {error}", file=sys.stderr)
        sys.exit(2)
    hits = []
    for number, line in enumerate(lines, 1):
        match = compiled.search(line)
        if match:
            hits.append({"line": number, "span": list(match.span()), "groups": list(match.groups())})
    return {"pattern": compiled.pattern, "flags": compiled.flags, "hits": hits}


def config(lines):
    entries = {}
    skipped = []
    for line in lines:
        if re.match(r"\s*(#|$)", line):
            continue
        match = CONFIG_LINE.match(line)
        if match is None:
            skipped.append(line)
            continue
        value = match["value"]
        if re.fullmatch(r"-?\d+", value):
            value = int(value)
        entries[match["key"]] = value
    return {"entries": entries, "skipped": skipped}


def main():
    parser = argparse.ArgumentParser(prog="re_text_tool", description="Regex text utilities.")
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("extract").add_argument("text")
    sub.add_parser("redact").add_argument("text")
    grep_parser = sub.add_parser("grep")
    grep_parser.add_argument("pattern")
    grep_parser.add_argument("lines", nargs="*")
    grep_parser.add_argument("-i", "--ignore-case", action="store_true")
    sub.add_parser("config").add_argument("lines", nargs="*")
    args = parser.parse_args()
    if args.command == "extract":
        result = extract(args.text)
    elif args.command == "redact":
        result = redact(args.text)
    elif args.command == "grep":
        result = grep(args.pattern, args.lines, args.ignore_case)
    else:
        result = config(args.lines)
    print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
