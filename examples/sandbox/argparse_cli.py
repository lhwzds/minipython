# SPDX-License-Identifier: MIT

"""A skill-shaped command-line tool: argparse + json + sys only.

Run it like any CPython script; `--help`, usage errors (exit status 2 with
usage and the error on stderr), and subcommands behave identically:

    mnpy examples/sandbox/argparse_cli.py --help
    mnpy examples/sandbox/argparse_cli.py stats '[3, 1, 4, 1, 5]' --round 2
    mnpy examples/sandbox/argparse_cli.py stats '[]' --round many
"""

import argparse
import json
import sys


def positive_int(text):
    value = int(text)
    if value < 0:
        raise argparse.ArgumentTypeError(f"{text!r} is negative")
    return value


def build_parser():
    parser = argparse.ArgumentParser(
        description=(
            "Inspect JSON documents passed on the command line. Every "
            "subcommand prints its result to stdout and reports problems "
            "on stderr with a non-zero exit status."
        ),
        epilog="Example: argparse_cli.py keys '{\"b\": 1, \"a\": 2}' --sort",
    )
    parser.add_argument(
        "-v", "--verbose", action="count", default=0, help="explain what is happening"
    )
    parser.add_argument(
        "--indent", type=positive_int, default=None, metavar="N",
        help="pretty-print JSON output with N spaces (default: compact)",
    )
    commands = parser.add_subparsers(dest="command", required=True, metavar="COMMAND")

    stats = commands.add_parser("stats", help="summarize a JSON array of numbers")
    stats.add_argument("numbers", type=json.loads, help="JSON array, e.g. '[1, 2, 3]'")
    stats.add_argument("--round", type=positive_int, default=3, help="decimal places (default: %(default)s)")

    keys = commands.add_parser("keys", aliases=["k"], help="list the keys of a JSON object")
    keys.add_argument("document", type=json.loads)
    order = keys.add_mutually_exclusive_group()
    order.add_argument("--sort", action="store_true", help="sort keys alphabetically")
    order.add_argument("--reverse", action="store_true", help="reverse insertion order")

    pick = commands.add_parser("pick", help="select fields from a JSON object")
    pick.add_argument("document", type=json.loads)
    pick.add_argument("fields", nargs="+", metavar="FIELD")
    pick.add_argument("--missing", choices=["skip", "null", "error"], default="skip")
    return parser


def main(argv=None):
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.verbose:
        print(f"command={args.command} verbose={args.verbose}", file=sys.stderr)
    if args.command == "stats":
        numbers = args.numbers
        if not isinstance(numbers, list) or not numbers:
            parser.error("stats needs a non-empty JSON array")
        total = sum(numbers)
        result = {
            "count": len(numbers),
            "total": total,
            "mean": round(total / len(numbers), args.round),
            "min": min(numbers),
            "max": max(numbers),
        }
    elif args.command in ("keys", "k"):
        keys = list(args.document)
        if args.sort:
            keys.sort()
        elif args.reverse:
            keys.reverse()
        result = keys
    else:
        result = {}
        for field in args.fields:
            if field in args.document:
                result[field] = args.document[field]
            elif args.missing == "null":
                result[field] = None
            elif args.missing == "error":
                print(f"missing field: {field}", file=sys.stderr)
                return 1
    print(json.dumps(result, indent=args.indent))
    return 0


if __name__ == "__main__":
    sys.exit(main())
