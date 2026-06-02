use std::collections::HashSet;

const INVENTORY: &str = include_str!("cpython_grammar_inventory.md");

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
