# SPDX-License-Identifier: MIT

"""A skill-shaped CSV tool built on csv, io, argparse, and json.

Agent skills often ship small data tables and read them with
csv.DictReader. The sandbox has no file access yet, so this tool takes the
CSV text itself as an argument and works entirely in memory:

    summary  group rows by one column and total another
    convert  re-emit the rows as JSON or tab-separated values
    sniff    detect the dialect and whether the first row is a header
    check    parse strictly and report malformed input
"""

import argparse
import csv
import io
import json
import sys


def read_rows(text):
    return list(csv.DictReader(io.StringIO(text, newline="")))


def summary(args):
    totals = {}
    counts = {}
    for row in read_rows(args.csv):
        key = row.get(args.by) or "(missing)"
        try:
            value = float(row[args.sum])
        except (KeyError, TypeError, ValueError):
            print(f"skipping row {row!r}", file=sys.stderr)
            continue
        totals[key] = totals.get(key, 0.0) + value
        counts[key] = counts.get(key, 0) + 1
    writer = csv.writer(sys.stdout, lineterminator="\n")
    writer.writerow([args.by, "rows", "total", "mean"])
    for key in sorted(totals):
        writer.writerow([key, counts[key], round(totals[key], 2), round(totals[key] / counts[key], 3)])


def convert(args):
    rows = read_rows(args.csv)
    if args.to == "json":
        print(json.dumps(rows, indent=1, ensure_ascii=False))
        return
    if not rows:
        return
    out = io.StringIO()
    writer = csv.DictWriter(out, fieldnames=list(rows[0]), dialect="excel-tab", extrasaction="ignore")
    writer.writeheader()
    writer.writerows(rows)
    sys.stdout.write(out.getvalue().replace("\r\n", "\n"))


def sniff(args):
    sniffer = csv.Sniffer()
    try:
        dialect = sniffer.sniff(args.sample)
    except csv.Error as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"delimiter={dialect.delimiter!r} quotechar={dialect.quotechar!r} "
          f"doublequote={dialect.doublequote} header={sniffer.has_header(args.sample)}")
    for row in csv.reader(args.sample.splitlines(), dialect):
        print(row)
    return 0


def check(args):
    reader = csv.reader(io.StringIO(args.csv, newline=""), strict=True)
    try:
        widths = {len(row) for row in reader}
    except csv.Error as error:
        print(f"line {reader.line_num}: {error}", file=sys.stderr)
        return 1
    print(f"ok: {reader.line_num} lines, widths {sorted(widths)}")
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(prog="csv_report_tool", description="In-memory CSV reports.")
    commands = parser.add_subparsers(dest="command", required=True)
    p = commands.add_parser("summary", help="total a column per group")
    p.add_argument("csv")
    p.add_argument("--by", required=True)
    p.add_argument("--sum", required=True)
    p.set_defaults(run=summary)
    p = commands.add_parser("convert", help="convert rows to JSON or TSV")
    p.add_argument("csv")
    p.add_argument("--to", choices=["json", "tsv"], default="json")
    p.set_defaults(run=convert)
    p = commands.add_parser("sniff", help="detect the dialect of a sample")
    p.add_argument("sample")
    p.set_defaults(run=sniff)
    p = commands.add_parser("check", help="parse strictly")
    p.add_argument("csv")
    p.set_defaults(run=check)
    args = parser.parse_args(argv)
    return args.run(args) or 0


if __name__ == "__main__":
    sys.exit(main())
