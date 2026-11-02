// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Modules/_csv.c (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! `_csv.reader`: the record parser.
//!
//! A record is parsed by a character state machine fed one input line at a
//! time, with an end-of-line event after each line, until the machine is
//! back at the start of a record. The states and transitions follow the
//! observable behavior of CPython 3.14.6's reader: quoted fields may span
//! lines, a stray line break inside an unquoted field is an error,
//! `strict` turns lenient recoveries into errors, and the `QUOTE_*` modes
//! decide which unquoted fields become floats or `None`.

use super::super::native_class::{c_slot_no_args, c_unpack_positional, native_instance};
use super::*;

const INPUT_FIELD: &str = "\0csv_input";
const DIALECT_FIELD: &str = "\0csv_dialect";
const LINE_NUM_FIELD: &str = "\0csv_line_num";
/// Counts `__next__` calls, so a call made from inside the input iterator
/// (re-entering the reader) is detected when the outer call resumes.
const GENERATION_FIELD: &str = "\0csv_generation";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    StartRecord,
    StartField,
    EscapedChar,
    InField,
    InQuotedField,
    EscapeInQuotedField,
    QuoteInQuotedField,
    EatCrNl,
    AfterEscapedCrNl,
}

/// The parse of one record.
struct Parser {
    state: State,
    field: String,
    field_len: i64,
    fields: Vec<Value>,
    unquoted: bool,
}

/// What a character (or the end of a line, `None`) does to the record.
enum Step {
    Continue,
    Error(String),
}

impl Vm {
    pub(super) fn csv_new_reader(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let [iterable, dialect] = c_unpack_positional("reader", args, 1, 2)?
            .try_into()
            .expect("two slots");
        let input = self.get_iter_propagating(iterable.expect("required"))?;
        let dialect = self.csv_call_dialect(dialect, keywords)?;
        let class = self.csv_state(STATE_READER_TYPE)?;
        native_instance(
            &class,
            vec![
                (INPUT_FIELD.to_string(), input),
                (DIALECT_FIELD.to_string(), dialect),
                (LINE_NUM_FIELD.to_string(), Value::Number(0)),
            ],
        )
    }

    pub(super) fn csv_reader_method(
        &mut self,
        method: &str,
        mut args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        if method == "__new__" {
            return Err(cannot_create("reader", args.first()));
        }
        if args.is_empty() {
            return Err(format!(
                "TypeError: descriptor '{method}' of '_csv.reader' object needs an argument"
            ));
        }
        let receiver = args.remove(0);
        match method {
            "__iter__" => {
                c_slot_no_args(&args, &keywords)?;
                Ok(receiver)
            }
            "__next__" => {
                c_slot_no_args(&args, &keywords)?;
                self.csv_reader_next(&receiver)
            }
            "dialect" => Ok(field(&receiver, DIALECT_FIELD).unwrap_or(Value::None)),
            "line_num" => Ok(field(&receiver, LINE_NUM_FIELD).unwrap_or(Value::Number(0))),
            "__setattr__" | "__delattr__" => {
                let name = args.first().and_then(string_of).unwrap_or_default();
                if matches!(name.as_str(), "dialect" | "line_num") {
                    return Err("AttributeError: readonly attribute".to_string());
                }
                Err(no_new_attribute(
                    "_csv.reader",
                    &name,
                    method == "__delattr__",
                ))
            }
            "__reduce_ex__" => Err(csv_pickle_error("reader", &receiver)),
            "__getattr__" => Err(missing_attribute("_csv.reader", &args)),
            _ => Err(format!("unknown builtin: _csv.reader.{method}")),
        }
    }

    /// `reader.__next__`: parses lines until a whole record is read.
    fn csv_reader_next(&mut self, receiver: &Value) -> Result<Value, String> {
        let dialect = field(receiver, DIALECT_FIELD).unwrap_or(Value::None);
        let Some(config) = dialect_config(&dialect) else {
            return Err("SystemError: csv reader has no dialect".to_string());
        };
        let Some(mut input) = field(receiver, INPUT_FIELD) else {
            return Err("SystemError: csv reader has no input".to_string());
        };
        let limit = self.csv_field_limit()?;
        let generation = match field(receiver, GENERATION_FIELD) {
            Some(Value::Number(generation)) => generation + 1,
            _ => 1,
        };
        set_field(receiver, GENERATION_FIELD, Value::Number(generation));
        let mut parser = Parser {
            state: State::StartRecord,
            field: String::new(),
            field_len: 0,
            fields: Vec::new(),
            unquoted: true,
        };
        loop {
            let line = match self.advance_owned_iterator_capturing(&mut input)? {
                Ok(IteratorAdvance::Yield(line)) => line,
                Ok(IteratorAdvance::Complete(_)) | Ok(IteratorAdvance::Raised) => {
                    if parser.field_len != 0 || parser.state == State::InQuotedField {
                        if config.strict {
                            return self.csv_error("unexpected end of data");
                        }
                        self.csv_save_field(&mut parser, &config)?;
                        break;
                    }
                    return Err("StopIteration".to_string());
                }
                Err(exception) => return Err(self.propagate_captured_exception(exception)),
            };
            if field(receiver, GENERATION_FIELD) != Some(Value::Number(generation)) {
                return self.csv_error("iterator has already advanced the reader");
            }
            let Some(text) = string_of(&line) else {
                return self.csv_error(format!(
                    "iterator should return strings, not {} (the file should be opened in text mode)",
                    type_name(&line)
                ));
            };
            let line_num = match field(receiver, LINE_NUM_FIELD) {
                Some(Value::Number(number)) => number,
                _ => 0,
            };
            set_field(receiver, LINE_NUM_FIELD, Value::Number(line_num + 1));
            for c in text.chars() {
                self.csv_process(&mut parser, &config, Some(c), limit)?;
            }
            self.csv_process(&mut parser, &config, None, limit)?;
            if parser.state == State::StartRecord {
                break;
            }
        }
        self.consume_allocation_budget(
            parser
                .fields
                .len()
                .saturating_mul(std::mem::size_of::<Value>()),
        )?;
        Ok(list_value(parser.fields))
    }

    fn csv_process(
        &mut self,
        parser: &mut Parser,
        config: &Config,
        c: Option<char>,
        limit: i64,
    ) -> Result<(), String> {
        match parse_step(self, parser, config, c, limit)? {
            Step::Continue => Ok(()),
            Step::Error(message) => self.csv_error(message),
        }
    }

    /// Ends the current field, converting it as the quoting mode asks.
    fn csv_save_field(&mut self, parser: &mut Parser, config: &Config) -> Result<(), String> {
        let quoting = config.quoting;
        let value = if parser.unquoted
            && parser.field_len == 0
            && (quoting == QUOTE_NOTNULL || quoting == QUOTE_STRINGS)
        {
            Value::None
        } else {
            let text = std::mem::take(&mut parser.field);
            let unquoted_number = parser.unquoted
                && parser.field_len != 0
                && (quoting == QUOTE_NONNUMERIC || quoting == QUOTE_STRINGS);
            parser.field_len = 0;
            if unquoted_number {
                self.call_value(builtin_type_value("float"), vec![Value::String(text)])?
            } else {
                Value::String(text)
            }
        };
        parser.fields.push(value);
        parser.unquoted = true;
        Ok(())
    }
}

fn add_char(parser: &mut Parser, c: char, limit: i64) -> Option<Step> {
    if parser.field_len >= limit {
        return Some(Step::Error(format!(
            "field larger than field limit ({limit})"
        )));
    }
    parser.field.push(c);
    parser.field_len += 1;
    None
}

/// One step of the state machine; `c` is `None` at the end of a line.
fn parse_step(
    vm: &mut Vm,
    parser: &mut Parser,
    config: &Config,
    c: Option<char>,
    limit: i64,
) -> Result<Step, String> {
    let is_newline = matches!(c, Some('\n' | '\r'));
    let quoting_enabled = config.quoting != QUOTE_NONE;
    let is_quote = c.is_some() && c == config.quotechar && quoting_enabled;
    let is_escape = c.is_some() && c == config.escapechar;
    let is_delimiter = c == Some(config.delimiter);
    macro_rules! add {
        ($c:expr) => {
            if let Some(step) = add_char(parser, $c, limit) {
                return Ok(step);
            }
        };
    }
    macro_rules! end_line {
        () => {{
            vm.csv_save_field(parser, config)?;
            parser.state = if c.is_none() {
                State::StartRecord
            } else {
                State::EatCrNl
            };
        }};
    }
    let mut state = parser.state;
    if state == State::StartRecord {
        match c {
            None => return Ok(Step::Continue),
            Some('\n' | '\r') => {
                parser.state = State::EatCrNl;
                return Ok(Step::Continue);
            }
            Some(_) => state = State::StartField,
        }
    }
    if state == State::AfterEscapedCrNl {
        if c.is_none() {
            return Ok(Step::Continue);
        }
        state = State::InField;
    }
    match state {
        State::StartField => {
            if is_newline || c.is_none() {
                end_line!();
            } else if is_quote {
                parser.unquoted = false;
                parser.state = State::InQuotedField;
            } else if is_escape {
                parser.state = State::EscapedChar;
            } else if c == Some(' ') && config.skipinitialspace {
                parser.state = State::StartField;
            } else if is_delimiter {
                vm.csv_save_field(parser, config)?;
                parser.state = State::StartField;
            } else {
                add!(c.expect("a character"));
                parser.state = State::InField;
            }
        }
        State::EscapedChar => {
            if is_newline {
                add!(c.expect("a character"));
                parser.state = State::AfterEscapedCrNl;
            } else {
                add!(c.unwrap_or('\n'));
                parser.state = State::InField;
            }
        }
        State::InField => {
            if is_newline || c.is_none() {
                end_line!();
            } else if is_escape {
                parser.state = State::EscapedChar;
            } else if is_delimiter {
                vm.csv_save_field(parser, config)?;
                parser.state = State::StartField;
            } else {
                // The state is left as it was: after an escaped line break
                // it stays `AfterEscapedCrNl`, which ignores line ends.
                add!(c.expect("a character"));
            }
        }
        State::InQuotedField => {
            if c.is_none() {
            } else if is_escape {
                parser.state = State::EscapeInQuotedField;
            } else if is_quote {
                parser.state = if config.doublequote {
                    State::QuoteInQuotedField
                } else {
                    State::InField
                };
            } else {
                add!(c.expect("a character"));
            }
        }
        State::EscapeInQuotedField => {
            add!(c.unwrap_or('\n'));
            parser.state = State::InQuotedField;
        }
        State::QuoteInQuotedField => {
            if is_quote {
                add!(c.expect("a character"));
                parser.state = State::InQuotedField;
            } else if is_delimiter {
                vm.csv_save_field(parser, config)?;
                parser.state = State::StartField;
            } else if is_newline || c.is_none() {
                end_line!();
            } else if !config.strict {
                add!(c.expect("a character"));
                parser.state = State::InField;
            } else {
                return Ok(Step::Error(format!(
                    "'{}' expected after '{}'",
                    config.delimiter,
                    config.quotechar.unwrap_or('"')
                )));
            }
        }
        State::EatCrNl => {
            if is_newline {
            } else if c.is_none() {
                parser.state = State::StartRecord;
            } else {
                return Ok(Step::Error(
                    "new-line character seen in unquoted field - do you need to open the file with newline=''?"
                        .to_string(),
                ));
            }
        }
        State::StartRecord | State::AfterEscapedCrNl => {
            unreachable!("handled before the match")
        }
    }
    Ok(Step::Continue)
}
