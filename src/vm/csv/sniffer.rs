// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Lib/csv.py (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! `csv.Sniffer`: dialect and header detection.
//!
//! The heuristics reproduce CPython 3.14.6's observable results:
//! quote-and-delimiter detection runs regular expressions through the
//! native `re` module, the delimiter fallback builds per-line character
//! frequency tables over chunks of ten lines, and `has_header` votes on the
//! column types of up to twenty-one rows after the first.

use super::super::native_class::NativeClass;
use super::*;

/// The quote-and-delimiter patterns, tried in order, with the group
/// numbers of `quote`, `delim`, and `space`.
const QUOTE_PATTERNS: &[(&str, usize, Option<usize>, Option<usize>)] = &[
    (
        r#"(?P<delim>[^\w\n"'])(?P<space> ?)(?P<quote>["']).*?(?P=quote)(?P=delim)"#,
        3,
        Some(1),
        Some(2),
    ),
    (
        r#"(?:^|\n)(?P<quote>["']).*?(?P=quote)(?P<delim>[^\w\n"'])(?P<space> ?)"#,
        1,
        Some(2),
        Some(3),
    ),
    (
        r#"(?P<delim>[^\w\n"'])(?P<space> ?)(?P<quote>["']).*?(?P=quote)(?:$|\n)"#,
        3,
        Some(1),
        Some(2),
    ),
    (
        r#"(?:^|\n)(?P<quote>["']).*?(?P=quote)(?:$|\n)"#,
        1,
        None,
        None,
    ),
];

const RE_MULTILINE: i64 = 8;
const RE_DOTALL: i64 = 16;

/// A column type in `has_header`'s vote.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ColumnType {
    Complex,
    Length(usize),
}

/// Counts of `key` in insertion order (a Python dict of counters), with a
/// hash index so counting stays linear in the number of updates.
struct Counter<K> {
    entries: Vec<(K, i64)>,
    index: HashMap<K, usize>,
}

impl<K: Clone + Eq + std::hash::Hash> Counter<K> {
    fn new() -> Self {
        Counter {
            entries: Vec::new(),
            index: HashMap::new(),
        }
    }

    fn bump(&mut self, key: K) {
        match self.index.get(&key) {
            Some(&position) => self.entries[position].1 += 1,
            None => {
                self.index.insert(key.clone(), self.entries.len());
                self.entries.push((key, 1));
            }
        }
    }

    fn get(&self, key: &K) -> i64 {
        self.index
            .get(key)
            .map_or(0, |&position| self.entries[position].1)
    }
}

/// `max(counts, key=counts.get)`: the first key with the largest count.
fn first_max<K: Clone>(counts: &[(K, i64)]) -> Option<K> {
    let mut best: Option<&(K, i64)> = None;
    for entry in counts {
        if best.is_none_or(|best| entry.1 > best.1) {
            best = Some(entry);
        }
    }
    best.map(|(key, _)| key.clone())
}

/// A `has_header` column's state: not yet typed, typed, or dropped after
/// a type change (`del columnTypes[col]`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum ColumnState {
    Untyped,
    Typed(ColumnType),
    Dropped,
}

impl Vm {
    pub(super) fn csv_sniffer_method(
        &mut self,
        method: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let sig = |qualname: &'static str, params: &'static [&'static str], required| Sig {
            qualname,
            params,
            required,
            kwonly: &[],
            varargs: false,
            varkw: false,
        };
        match method {
            "__init__" => {
                let bound = bind_args(&sig("Sniffer.__init__", &["self"], 1), args, keywords)?;
                let preferred = list_value(
                    [",", "\t", ";", " ", ":"]
                        .iter()
                        .map(|text| Value::String((*text).to_string()))
                        .collect(),
                );
                self.store_attribute_value(bound.arg(0), "preferred", preferred)?;
                Ok(Value::None)
            }
            "sniff" => {
                let bound = bind_args(
                    &sig("Sniffer.sniff", &["self", "sample", "delimiters"], 2),
                    args,
                    keywords,
                )?;
                self.csv_sniff(bound.arg(0), bound.arg(1), bound.arg(2))
            }
            "_guess_quote_and_delimiter" => {
                let bound = bind_args(
                    &sig(
                        "Sniffer._guess_quote_and_delimiter",
                        &["self", "data", "delimiters"],
                        3,
                    ),
                    args,
                    keywords,
                )?;
                self.csv_guess_quote_and_delimiter(bound.arg(1), bound.arg(2))
            }
            "_guess_delimiter" => {
                let bound = bind_args(
                    &sig(
                        "Sniffer._guess_delimiter",
                        &["self", "data", "delimiters"],
                        3,
                    ),
                    args,
                    keywords,
                )?;
                self.csv_guess_delimiter(bound.arg(0), bound.arg(1), bound.arg(2))
            }
            "has_header" => {
                let bound = bind_args(
                    &sig("Sniffer.has_header", &["self", "sample"], 2),
                    args,
                    keywords,
                )?;
                self.csv_has_header(bound.arg(0), bound.arg(1))
            }
            _ => Err(format!("unknown builtin: csv.Sniffer.{method}")),
        }
    }

    /// Unpacks a returned tuple into exactly `count` items, with Python's
    /// unpacking errors.
    fn csv_unpack(&mut self, value: Value, count: usize) -> Result<Vec<Value>, String> {
        let items = self.collect_iterable_values_propagating(value)?;
        if items.len() > count {
            return Err(format!(
                "ValueError: too many values to unpack (expected {count}, got {})",
                items.len()
            ));
        }
        if items.len() < count {
            return Err(format!(
                "ValueError: not enough values to unpack (expected {count}, got {})",
                items.len()
            ));
        }
        Ok(items)
    }

    fn csv_sniff(
        &mut self,
        receiver: Value,
        sample: Value,
        delimiters: Value,
    ) -> Result<Value, String> {
        let guess = self.load_attribute_value(receiver.clone(), "_guess_quote_and_delimiter")?;
        let result = self.call_value(guess, vec![sample.clone(), delimiters.clone()])?;
        let [quotechar, doublequote, mut delimiter, mut skipinitialspace]: [Value; 4] =
            self.csv_unpack(result, 4)?.try_into().expect("four values");
        if !self.truth_value(delimiter.clone())? {
            let guess = self.load_attribute_value(receiver, "_guess_delimiter")?;
            let result = self.call_value(guess, vec![sample, delimiters])?;
            let [found, skip]: [Value; 2] =
                self.csv_unpack(result, 2)?.try_into().expect("two values");
            delimiter = found;
            skipinitialspace = skip;
        }
        if !self.truth_value(delimiter.clone())? {
            return self.csv_error("Could not determine delimiter");
        }
        let quotechar = if self.truth_value(quotechar.clone())? {
            quotechar
        } else {
            Value::String("\"".to_string())
        };
        let base = self.csv_state(STATE_PY_DIALECT)?;
        let class = NativeClass {
            name: "dialect",
            module: "csv",
            doc: None,
            bases: vec![base],
            attrs: vec![
                ("_name".to_string(), Value::String("sniffed".to_string())),
                (
                    "lineterminator".to_string(),
                    Value::String("\r\n".to_string()),
                ),
                ("quoting".to_string(), Value::Number(QUOTE_MINIMAL)),
                ("doublequote".to_string(), doublequote),
                ("delimiter".to_string(), delimiter),
                ("quotechar".to_string(), quotechar),
                ("skipinitialspace".to_string(), skipinitialspace),
            ],
            hidden: Vec::new(),
        }
        .build();
        if let Value::Class { attrs, .. } = &class {
            let qualname = Value::String("Sniffer.sniff.<locals>.dialect".to_string());
            let mut attrs = attrs.borrow_mut();
            attrs.insert("__qualname__".to_string(), qualname.clone());
            attrs.insert(CLASS_QUALNAME_ATTR.to_string(), qualname);
        }
        Ok(class)
    }

    fn csv_re(&mut self) -> Result<Value, String> {
        if let Some(module @ Value::Module { .. }) = lookup_string_key(&self.module_cache, "re") {
            return Ok(module);
        }
        self.load_stdlib_module("re")
    }

    fn csv_re_call(&mut self, function: &str, args: Vec<Value>) -> Result<Value, String> {
        let re = self.csv_re()?;
        let function = self.load_attribute_value(re, function)?;
        self.call_value(function, args)
    }

    /// Whether `key` is in the user's `delimiters` (always, when `None`).
    fn csv_allowed(&mut self, delimiters: &Value, key: &str) -> Result<bool, String> {
        if matches!(delimiters, Value::None) {
            return Ok(true);
        }
        self.contains_value(Value::String(key.to_string()), delimiters.clone())
    }

    fn csv_guess_quote_and_delimiter(
        &mut self,
        data: Value,
        delimiters: Value,
    ) -> Result<Value, String> {
        let mut matches = Vec::new();
        let mut groups = (1, None, None);
        for (pattern, quote, delim, space) in QUOTE_PATTERNS {
            let regexp = self.csv_re_call(
                "compile",
                vec![
                    Value::String((*pattern).to_string()),
                    Value::Number(RE_DOTALL | RE_MULTILINE),
                ],
            )?;
            let findall = self.load_attribute_value(regexp, "findall")?;
            let found = self.call_value(findall, vec![data.clone()])?;
            matches = self.collect_iterable_values_propagating(found)?;
            groups = (*quote, *delim, *space);
            if !matches.is_empty() {
                break;
            }
        }
        if matches.is_empty() {
            return Ok(tuple_value(vec![
                Value::String(String::new()),
                Value::Bool(false),
                Value::None,
                Value::Number(0),
            ]));
        }
        let (quote_group, delim_group, space_group) = groups;
        let group = |m: &Value, index: usize| -> String {
            match m {
                Value::Tuple(items) => items.get(index - 1).and_then(string_of).unwrap_or_default(),
                // A single group: findall returns the strings themselves,
                // and `m[0]` is their first character.
                other => string_of(other)
                    .and_then(|text| text.chars().next())
                    .map(|c| c.to_string())
                    .unwrap_or_default(),
            }
        };
        let mut quotes: Counter<String> = Counter::new();
        let mut delims: Counter<String> = Counter::new();
        let mut spaces = 0;
        for m in &matches {
            let key = group(m, quote_group);
            if !key.is_empty() {
                quotes.bump(key);
            }
            let Some(delim_group) = delim_group else {
                continue;
            };
            let key = group(m, delim_group);
            if !key.is_empty() && self.csv_allowed(&delimiters, &key)? {
                delims.bump(key);
            }
            if let Some(space_group) = space_group
                && !group(m, space_group).is_empty()
            {
                spaces += 1;
            }
        }
        let Some(quotechar) = first_max(&quotes.entries) else {
            return Err("ValueError: max() iterable argument is empty".to_string());
        };
        let (delim, skipinitialspace) = match first_max(&delims.entries) {
            Some(delim) => {
                let count = delims.get(&delim);
                let skip = Value::Bool(count == spaces);
                (if delim == "\n" { String::new() } else { delim }, skip)
            }
            None => (String::new(), Value::Number(0)),
        };
        // An extra quote between delimiters means a doubled-quote format.
        let escaped = self.csv_re_call("escape", vec![Value::String(delim.clone())])?;
        let escaped = string_of(&escaped).unwrap_or_default();
        let pattern = format!(
            "(({escaped})|^)\\W*{quotechar}[^{escaped}\\n]*{quotechar}[^{escaped}\\n]*{quotechar}\\W*(({escaped})|$)"
        );
        let regexp = self.csv_re_call(
            "compile",
            vec![Value::String(pattern), Value::Number(RE_MULTILINE)],
        )?;
        let search = self.load_attribute_value(regexp, "search")?;
        let found = self.call_value(search, vec![data])?;
        let doublequote = !matches!(found, Value::None);
        Ok(tuple_value(vec![
            Value::String(quotechar),
            Value::Bool(doublequote),
            Value::String(delim),
            skipinitialspace,
        ]))
    }

    fn csv_guess_delimiter(
        &mut self,
        receiver: Value,
        data: Value,
        delimiters: Value,
    ) -> Result<Value, String> {
        let Some(text) = string_of(&data) else {
            return Err(format!(
                "AttributeError: '{}' object has no attribute 'split'",
                type_name(&data)
            ));
        };
        // `list(filter(None, data.split('\n')))`, walked lazily: lines are
        // counted chunk by chunk, so no per-line table is ever stored.
        let nonempty = || text.split('\n').filter(|line| !line.is_empty());
        let mut steps = 0u64;
        self.tick_native_steps(&mut steps, text.len() as u64)?;
        let line_count = nonempty().count();
        let first = nonempty().next().unwrap_or("");
        let skip_for = |delim: &str| -> Value {
            let spaced = format!("{delim} ");
            Value::Bool(first.matches(delim).count() == first.matches(spaced.as_str()).count())
        };

        let chunk_length = line_count.min(10);
        let mut iteration = 0usize;
        // charFrequency: for each character, frequency -> rows, in
        // insertion order.
        let mut frequencies: Vec<Counter<usize>> = (0..127).map(|_| Counter::new()).collect();
        // modes, in insertion order.
        let mut modes: Vec<(u8, (usize, i64))> = Vec::new();
        let mut delims: Vec<(u8, (usize, i64))> = Vec::new();
        let mut lines = nonempty();
        let (mut start, mut end) = (0usize, chunk_length);
        while start < line_count {
            iteration += 1;
            for line in lines.by_ref().take(end.min(line_count) - start) {
                // Counts of each 7-bit ASCII character (0..127).
                let mut line_counts = [0usize; 127];
                for c in line.chars() {
                    if (c as u32) < 127 {
                        line_counts[c as usize] += 1;
                    }
                }
                self.tick_native_steps(&mut steps, line.len() as u64 + 127)?;
                for (frequency, count) in frequencies.iter_mut().zip(line_counts) {
                    frequency.bump(count);
                }
            }
            for (c, counter) in frequencies.iter().enumerate() {
                let items = &counter.entries;
                self.tick_native_steps(&mut steps, items.len() as u64)?;
                if items.len() == 1 && items[0].0 == 0 {
                    continue;
                }
                let mode = if items.len() > 1 {
                    let (freq, rows) = first_max(
                        &items
                            .iter()
                            .map(|(f, r)| ((*f, *r), *r))
                            .collect::<Vec<_>>(),
                    )
                    .expect("non-empty");
                    let others: i64 = items
                        .iter()
                        .filter(|(f, _)| *f != freq)
                        .map(|(_, r)| *r)
                        .sum();
                    (freq, rows - others)
                } else {
                    items[0]
                };
                match modes.iter_mut().find(|(key, _)| *key as usize == c) {
                    Some(entry) => entry.1 = mode,
                    None => modes.push((c as u8, mode)),
                }
            }
            let total = (chunk_length * iteration).min(line_count) as f64;
            let mut consistency = 1.0f64;
            let threshold = 0.9f64;
            while delims.is_empty() && consistency >= threshold {
                for (c, mode) in &modes {
                    if mode.0 > 0
                        && mode.1 > 0
                        && (mode.1 as f64 / total) >= consistency
                        && self.csv_allowed(&delimiters, &(*c as char).to_string())?
                    {
                        match delims.iter_mut().find(|(key, _)| key == c) {
                            Some(entry) => entry.1 = *mode,
                            None => delims.push((*c, *mode)),
                        }
                    }
                }
                consistency -= 0.01;
            }
            if delims.len() == 1 {
                self.flush_native_steps(&mut steps)?;
                let delim = (delims[0].0 as char).to_string();
                let skip = skip_for(&delim);
                return Ok(tuple_value(vec![Value::String(delim), skip]));
            }
            start = end;
            end += chunk_length;
        }
        self.flush_native_steps(&mut steps)?;
        if delims.is_empty() {
            return Ok(tuple_value(vec![
                Value::String(String::new()),
                Value::Number(0),
            ]));
        }
        // More than one candidate: fall back to the preferred list.
        let preferred = self.load_attribute_value(receiver, "preferred")?;
        for d in self.collect_iterable_values_propagating(preferred)? {
            let Some(d_text) = string_of(&d) else {
                continue;
            };
            if delims
                .iter()
                .any(|(key, _)| (*key as char).to_string() == d_text)
            {
                let skip = skip_for(&d_text);
                return Ok(tuple_value(vec![d, skip]));
            }
        }
        // Otherwise the candidate with the largest (frequency, rows), then
        // the largest character.
        let (delim, _) = delims
            .iter()
            .max_by(|(left_key, left), (right_key, right)| {
                left.cmp(right).then(left_key.cmp(right_key))
            })
            .expect("non-empty");
        let delim = (*delim as char).to_string();
        let skip = skip_for(&delim);
        Ok(tuple_value(vec![Value::String(delim), skip]))
    }

    fn csv_has_header(&mut self, receiver: Value, sample: Value) -> Result<Value, String> {
        let module = self.csv_module_for_globals()?;
        let string_io = self.load_attribute_value(module.clone(), "StringIO")?;
        let stream = self.call_value(string_io, vec![sample.clone()])?;
        let sniff = self.load_attribute_value(receiver, "sniff")?;
        let dialect = self.call_value(sniff, vec![sample])?;
        let reader_fn = self.load_attribute_value(module, "reader")?;
        let reader = self.call_value(reader_fn, vec![stream, dialect])?;
        let header = self.call_value(Value::Builtin("next".to_string()), vec![reader.clone()])?;
        let header = self.collect_iterable_values_propagating(header)?;
        let columns = header.len();
        // columnTypes, indexed by column (dropped columns stay as markers so
        // updates are O(1) and the survivors keep ascending order).
        let mut column_types = vec![ColumnState::Untyped; columns];
        let mut steps = 0u64;
        let complex = builtin_type_value("complex");
        let mut checked = 0;
        loop {
            let row = match self.advance_owned_iterator_capturing(&mut reader.clone())? {
                Ok(IteratorAdvance::Yield(row)) => row,
                Ok(_) => break,
                Err(exception) => return Err(self.propagate_captured_exception(exception)),
            };
            if checked > 20 {
                break;
            }
            checked += 1;
            let row = self.collect_iterable_values_propagating(row)?;
            if row.len() != columns {
                continue;
            }
            for column in 0..columns {
                if column_types[column] == ColumnState::Dropped {
                    continue;
                }
                self.tick_native_steps(&mut steps, 1)?;
                let value = row[column].clone();
                let this_type =
                    match self.call_value_catching(complex.clone(), vec![value.clone()])? {
                        Ok(_) => ColumnType::Complex,
                        Err(exception)
                            if exception.type_name == "ValueError"
                                || exception.type_name == "OverflowError" =>
                        {
                            ColumnType::Length(self.len_value(value)?)
                        }
                        Err(exception) => return Err(self.propagate_captured_exception(exception)),
                    };
                match column_types[column] {
                    ColumnState::Typed(current) if current == this_type => {}
                    ColumnState::Untyped => column_types[column] = ColumnState::Typed(this_type),
                    _ => column_types[column] = ColumnState::Dropped,
                }
            }
        }
        self.flush_native_steps(&mut steps)?;
        let mut votes: i64 = 0;
        for (column, column_type) in column_types.into_iter().enumerate() {
            match column_type {
                ColumnState::Dropped => {}
                ColumnState::Typed(ColumnType::Length(length)) => {
                    if self.len_value(header[column].clone())? != length {
                        votes += 1;
                    } else {
                        votes -= 1;
                    }
                }
                ColumnState::Typed(ColumnType::Complex) => {
                    match self.call_value_catching(complex.clone(), vec![header[column].clone()])? {
                        Ok(_) => votes -= 1,
                        Err(exception)
                            if exception.type_name == "ValueError"
                                || exception.type_name == "TypeError" =>
                        {
                            votes += 1
                        }
                        Err(exception) => return Err(self.propagate_captured_exception(exception)),
                    }
                }
                // `None(header[col])` raises TypeError.
                ColumnState::Untyped => votes += 1,
            }
        }
        Ok(Value::Bool(votes > 0))
    }
}
