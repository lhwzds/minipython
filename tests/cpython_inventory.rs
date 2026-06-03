use std::collections::{BTreeSet, HashSet};

const INVENTORY: &str = include_str!("cpython_grammar_inventory.md");
const CPYTHON_GRAMMAR_SOURCE: &str = "/Volumes/samsung/GitHub/cpython/Grammar/python.gram";

#[derive(Debug)]
struct InventoryRow<'a> {
    rule: &'a str,
    kind: &'a str,
    status: &'a str,
}

#[test]
fn cpython_grammar_inventory_summary_matches_rows() {
    let rows = inventory_rows();

    assert_eq!(
        rows.len(),
        summary_count("Total CPython grammar rules"),
        "inventory summary total must match the number of rule rows"
    );
    assert_eq!(
        rows.iter().filter(|row| row.kind == "normal").count(),
        summary_count("Normal grammar rules"),
        "normal-rule summary must match the rule rows"
    );
    assert_eq!(
        rows.iter().filter(|row| row.kind == "invalid").count(),
        summary_count("Invalid/error grammar rules"),
        "invalid-rule summary must match the rule rows"
    );
    assert_eq!(
        rows.iter().filter(|row| row.status == "missing").count(),
        summary_count("`missing` rows"),
        "missing-row summary must match the rule rows"
    );
    assert_eq!(
        rows.iter().filter(|row| row.status == "partial").count(),
        summary_count("`partial` rows"),
        "partial-row summary must match the rule rows"
    );
    assert_eq!(
        rows.iter().filter(|row| row.status == "supported").count(),
        summary_count("`supported` rows"),
        "supported-row summary must match the rule rows"
    );
}

#[test]
fn cpython_grammar_inventory_rows_are_well_formed() {
    let rows = inventory_rows();
    let mut seen = HashSet::new();

    for row in rows {
        assert!(
            matches!(row.kind, "normal" | "invalid"),
            "unknown inventory kind for `{}`: `{}`",
            row.rule,
            row.kind
        );
        assert!(
            matches!(row.status, "supported" | "partial" | "planned" | "missing"),
            "unknown inventory status for `{}`: `{}`",
            row.rule,
            row.status
        );
        assert!(
            seen.insert(row.rule),
            "duplicate inventory row: `{}`",
            row.rule
        );
    }
}

#[test]
fn cpython_grammar_inventory_rules_match_current_cpython_grammar() {
    let inventory_rules = inventory_rows()
        .into_iter()
        .map(|row| row.rule)
        .collect::<BTreeSet<_>>();
    let grammar = std::fs::read_to_string(CPYTHON_GRAMMAR_SOURCE)
        .unwrap_or_else(|error| panic!("failed to read {CPYTHON_GRAMMAR_SOURCE}: {error}"));
    let grammar_rules = cpython_grammar_rules(&grammar);

    assert_eq!(
        inventory_rules.len(),
        grammar_rules.len(),
        "grammar inventory and CPython grammar rule counts differ"
    );

    let missing = grammar_rules
        .difference(&inventory_rules)
        .copied()
        .collect::<Vec<_>>();
    let extra = inventory_rules
        .difference(&grammar_rules)
        .copied()
        .collect::<Vec<_>>();

    assert!(
        missing.is_empty() && extra.is_empty(),
        "grammar inventory drifted from CPython grammar; missing={missing:?}; extra={extra:?}"
    );
}

fn inventory_rows() -> Vec<InventoryRow<'static>> {
    INVENTORY
        .lines()
        .filter_map(|line| {
            if !line.starts_with("| `") {
                return None;
            }

            let cells = table_cells(line);
            if cells.len() < 3 {
                return None;
            }

            let rule = strip_backticks(cells[0])?;
            let kind = strip_backticks(cells[1])?;
            let status = strip_backticks(cells[2])?;
            Some(InventoryRow { rule, kind, status })
        })
        .collect()
}

fn cpython_grammar_rules(source: &str) -> BTreeSet<&str> {
    source
        .lines()
        .filter_map(|line| {
            let first = line.as_bytes().first()?;
            if !first.is_ascii_lowercase() {
                return None;
            }

            let colon = line.find(':')?;
            let header = &line[..colon];
            let name_end = header
                .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                .unwrap_or(header.len());
            let name = &header[..name_end];
            if name.is_empty() { None } else { Some(name) }
        })
        .collect()
}

fn summary_count(metric: &str) -> usize {
    INVENTORY
        .lines()
        .find_map(|line| {
            let cells = table_cells(line);
            if cells.len() == 2 && cells[0] == metric {
                cells[1].parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("missing summary metric: {metric}"))
}

fn table_cells(line: &str) -> Vec<&str> {
    line.trim_matches('|').split('|').map(str::trim).collect()
}

fn strip_backticks(cell: &str) -> Option<&str> {
    cell.strip_prefix('`')?.strip_suffix('`')
}
