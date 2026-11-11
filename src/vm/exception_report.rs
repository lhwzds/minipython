// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Lib/traceback.py (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! CPython-shaped reports for the exception that ends a program.
//!
//! The layout follows what CPython 3.14 writes to stderr for an uncaught
//! exception: chained causes and contexts oldest first, joined by CPython's
//! linking sentences; a `Traceback (most recent call last):` header for every
//! exception that was actually raised; and the `format_exception_only` line
//! followed by the exception's notes. Exception groups use CPython's boxed
//! layout. MiniPython does not record per-frame line numbers yet, so the
//! `File "...", line N, in name` lines CPython prints under each header are
//! omitted rather than invented.

use super::*;

const TRACEBACK_HEADER: &str = "Traceback (most recent call last):\n";
const GROUP_TRACEBACK_HEADER: &str = "Exception Group Traceback (most recent call last):\n";
const CAUSE_SEPARATOR: &str =
    "\nThe above exception was the direct cause of the following exception:\n\n";
const CONTEXT_SEPARATOR: &str =
    "\nDuring handling of the above exception, another exception occurred:\n\n";
/// CPython's default `max_group_width` and `max_group_depth`.
const MAX_GROUP_WIDTH: usize = 15;
const MAX_GROUP_DEPTH: usize = 10;

/// The uncaught exception that ended a run, already rendered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VmUncaughtException {
    /// `type(exc).__name__`.
    pub type_name: String,
    /// `str(exc)`.
    pub message: String,
    /// The complete stderr report, ending with a newline.
    pub traceback: String,
}

/// Indentation state shared while rendering nested exception groups.
#[derive(Default)]
struct ReportContext {
    seen: Vec<*const ExceptionIdentity>,
    /// `str(exc)` of the uncaught exception, computed once so a `__str__`
    /// with side effects runs once, as in CPython.
    top_message: Option<(*const ExceptionIdentity, Option<String>)>,
    group_depth: usize,
    need_close: bool,
}

impl ReportContext {
    fn indent(&self) -> String {
        " ".repeat(2 * self.group_depth)
    }

    /// Prefixes every line of `text` with the current group margin.
    fn emit(&self, out: &mut String, text: &str, margin: char) {
        let mut prefix = self.indent();
        if self.group_depth > 0 {
            prefix.push(margin);
            prefix.push(' ');
        }
        for line in text.split_inclusive('\n') {
            out.push_str(&prefix);
            out.push_str(line);
        }
    }
}

impl Vm {
    /// Renders the uncaught exception behind `result`, or `None` when the
    /// run succeeded or failed with a sandbox error that has no exception
    /// object.
    pub(super) fn uncaught_exception(
        &mut self,
        result: &Result<Value, String>,
    ) -> Option<VmUncaughtException> {
        let Err(message) = result else {
            return None;
        };
        let exception = matching_exception(self.in_flight_exception.take(), message)?;
        let class = exception_type_value(&exception);
        let type_name = self
            .report_string_attribute(&class, "__name__")
            .unwrap_or_else(|| exception.type_name.clone());
        let message = self.report_exception_str(&exception);
        let mut context = ReportContext {
            top_message: Some((Rc::as_ptr(&exception.identity), message.clone())),
            ..ReportContext::default()
        };
        let message = message.unwrap_or_else(|| "<exception str() failed>".to_string());
        let mut traceback = String::new();
        self.format_exception_chain(&exception, &mut context, &mut traceback);
        Some(VmUncaughtException {
            type_name,
            message,
            traceback,
        })
    }

    /// One exception with its `__cause__`/`__context__` chain, oldest first.
    fn format_exception_chain(
        &mut self,
        exception: &MiniException,
        context: &mut ReportContext,
        out: &mut String,
    ) {
        let mut chain: Vec<(Option<&'static str>, MiniException)> = Vec::new();
        let mut current = Some(exception.clone());
        while let Some(exception) = current {
            context.seen.push(Rc::as_ptr(&exception.identity));
            let unseen = |linked: &Option<Box<MiniException>>| {
                linked
                    .as_deref()
                    .filter(|linked| !context.seen.contains(&Rc::as_ptr(&linked.identity)))
                    .cloned()
            };
            let (separator, next) = if let Some(cause) = unseen(&exception.cause) {
                (Some(CAUSE_SEPARATOR), Some(cause))
            } else if !exception.suppress_context
                && let Some(linked) = unseen(&exception.context)
            {
                (Some(CONTEXT_SEPARATOR), Some(linked))
            } else {
                (None, None)
            };
            chain.push((separator, exception));
            current = next;
        }

        for (separator, exception) in chain.into_iter().rev() {
            if let Some(separator) = separator {
                context.emit(out, separator, '|');
            }
            match &exception.exceptions {
                None => {
                    if exception_was_raised(&exception) {
                        context.emit(out, TRACEBACK_HEADER, '|');
                    }
                    let only = self.format_exception_only(&exception, context);
                    context.emit(out, &only, '|');
                }
                Some(members) => self.format_exception_group(&exception, members, context, out),
            }
        }
    }

    fn format_exception_group(
        &mut self,
        exception: &MiniException,
        members: &[MiniException],
        context: &mut ReportContext,
        out: &mut String,
    ) {
        if context.group_depth > MAX_GROUP_DEPTH {
            context.emit(
                out,
                &format!("... (max_group_depth is {MAX_GROUP_DEPTH})\n"),
                '|',
            );
            return;
        }
        let top_level = context.group_depth == 0;
        if top_level {
            context.group_depth += 1;
        }
        if exception_was_raised(exception) {
            let margin = if top_level { '+' } else { '|' };
            context.emit(out, GROUP_TRACEBACK_HEADER, margin);
        }
        let only = self.format_exception_only(exception, context);
        context.emit(out, &only, '|');

        let shown = members.len().min(MAX_GROUP_WIDTH + 1);
        context.need_close = false;
        for index in 0..shown {
            let last = index + 1 == shown;
            if last {
                context.need_close = true;
            }
            let truncated = index >= MAX_GROUP_WIDTH;
            let title = if truncated {
                "...".to_string()
            } else {
                (index + 1).to_string()
            };
            out.push_str(&context.indent());
            out.push_str(if index == 0 { "+-" } else { "  " });
            out.push_str(&format!("+---------------- {title} ----------------\n"));
            context.group_depth += 1;
            if truncated {
                let remaining = members.len() - MAX_GROUP_WIDTH;
                let plural = if remaining > 1 { "s" } else { "" };
                context.emit(
                    out,
                    &format!("and {remaining} more exception{plural}\n"),
                    '|',
                );
            } else {
                self.format_exception_chain(&members[index], context, out);
            }
            if last && context.need_close {
                out.push_str(&context.indent());
                out.push_str("+------------------------------------\n");
                context.need_close = false;
            }
            context.group_depth -= 1;
        }
        if top_level {
            context.group_depth = 0;
        }
    }

    /// `traceback.format_exception_only` for one exception: the
    /// module-qualified type name, `str(exc)` when it is not empty, and the
    /// exception's notes.
    fn format_exception_only(
        &mut self,
        exception: &MiniException,
        context: &ReportContext,
    ) -> String {
        let class = exception_type_value(exception);
        let qualname = self
            .report_string_attribute(&class, "__qualname__")
            .unwrap_or_else(|| exception.type_name.clone());
        let type_name = match self
            .report_string_attribute(&class, "__module__")
            .as_deref()
        {
            Some("builtins" | "__main__") => qualname,
            Some(module) => format!("{module}.{qualname}"),
            None => format!("<unknown>.{qualname}"),
        };
        let mut out = if exception
            .type_hierarchy
            .iter()
            .any(|name| name == "SyntaxError")
        {
            self.format_syntax_error_only(exception, &type_name)
        } else {
            let message = match &context.top_message {
                Some((identity, message)) if *identity == Rc::as_ptr(&exception.identity) => {
                    message.clone()
                }
                _ => self.report_exception_str(exception),
            }
            .unwrap_or_else(|| "<exception str() failed>".to_string());
            if message.is_empty() {
                type_name
            } else {
                format!("{type_name}: {message}")
            }
        };
        out.push('\n');
        for note in self.report_exception_notes(exception) {
            out.push_str(&note);
            out.push('\n');
        }
        out
    }

    /// `traceback.TracebackException._format_syntax_error`: the location
    /// line, the stripped source line with carets under the error range,
    /// and `Type: msg` (without the trailing newline).
    fn format_syntax_error_only(&mut self, exception: &MiniException, type_name: &str) -> String {
        let value = value_from_exception(exception);
        let mut attribute = |name: &str| {
            self.load_attribute_catching(value.clone(), name)
                .ok()
                .and_then(Result::ok)
                .unwrap_or(Value::None)
        };
        let filename = attribute("filename");
        let lineno = attribute("lineno");
        let text = attribute("text");
        let offset = attribute("offset");
        let end_lineno = attribute("end_lineno");
        let end_offset = attribute("end_offset");
        let msg = attribute("msg");
        let integer = |value: &Value| match value {
            Value::Number(number) => Some(*number),
            Value::Bool(flag) => Some(i64::from(*flag)),
            _ => None,
        };

        let mut out = String::new();
        let mut filename_suffix = String::new();
        let filename_text = report_text(filename.clone());
        if !matches!(lineno, Value::None) {
            let shown = filename_text
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "<string>".to_string());
            out.push_str(&format!(
                "  File \"{shown}\", line {}\n",
                self.str_value(&lineno).unwrap_or_default()
            ));
        } else if !matches!(filename, Value::None) {
            filename_suffix = format!(" ({})", self.str_value(&filename).unwrap_or_default());
        }

        if let Some(text) = report_text(text) {
            let rtext = text.trim_end_matches('\n');
            let ltext = rtext.trim_start_matches([' ', '\n', '\x0c']);
            let spaces = (rtext.chars().count() - ltext.chars().count()) as i64;
            let text_len = text.chars().count() as i64;
            let rtext_len = rtext.chars().count() as i64;
            match integer(&offset) {
                None if matches!(offset, Value::None) => {
                    out.push_str(&format!("    {ltext}\n"));
                }
                None => {}
                Some(mut start) => {
                    let mut end = if integer(&lineno) == integer(&end_lineno) {
                        match integer(&end_offset) {
                            Some(end) if end != 0 => end,
                            _ => start,
                        }
                    } else {
                        rtext_len + 1
                    };
                    if text_len > 0 && start > text_len {
                        start = rtext_len + 1;
                    }
                    if text_len > 0 && end > text_len {
                        end = rtext_len + 1;
                    }
                    if start >= end || end < 0 {
                        end = start + 1;
                    }
                    let column = start - 1 - spaces;
                    let end_column = end - 1 - spaces;
                    out.push_str(&format!("    {ltext}\n"));
                    if column >= 0 {
                        let caret_space = ltext
                            .chars()
                            .take(column as usize)
                            .map(|ch| if ch.is_whitespace() { ch } else { ' ' })
                            .collect::<String>();
                        out.push_str(&format!(
                            "    {caret_space}{}\n",
                            "^".repeat((end_column - column).max(0) as usize)
                        ));
                    }
                }
            }
        }

        let msg = match &msg {
            Value::None => String::new(),
            value => self.str_value(value).unwrap_or_default(),
        };
        let msg = if msg.is_empty() {
            "<no detail available>".to_string()
        } else {
            msg
        };
        out.push_str(&format!("{type_name}: {msg}{filename_suffix}"));
        out
    }

    /// `str(exc)`, honoring a `__str__` defined on the exception's class.
    /// `None` when `__str__` raises or returns a non-string.
    fn report_exception_str(&mut self, exception: &MiniException) -> Option<String> {
        let value = value_from_exception(exception);
        if let Some(Value::Class { attrs, bases, .. }) = exception.type_object.as_deref()
            && let Some(method) = find_class_attr(attrs, bases, "__str__")
        {
            let result = self
                .call_value_catching(bind_method(method, value), Vec::new())
                .ok()?
                .ok()?;
            return report_text(result);
        }
        self.str_value(&value).ok()
    }

    /// The lines CPython prints for `exc.__notes__`: each item of a sequence
    /// through `str()`, or the `repr()` of any other non-`None` value.
    fn report_exception_notes(&mut self, exception: &MiniException) -> Vec<String> {
        let Some(notes) = exception_notes_value(&exception.identity) else {
            return Vec::new();
        };
        let items = match &notes {
            Value::None => return Vec::new(),
            Value::List(items) => items.borrow().clone(),
            Value::Tuple(items) => items.as_ref().clone(),
            Value::String(text) => text
                .chars()
                .map(|ch| Value::String(ch.to_string()))
                .collect(),
            other => {
                return vec![
                    self.repr_value(other)
                        .unwrap_or_else(|_| "<__notes__ repr() failed>".to_string()),
                ];
            }
        };
        items
            .iter()
            .map(|note| {
                self.str_value(note)
                    .unwrap_or_else(|_| "<note str() failed>".to_string())
            })
            .collect()
    }

    fn report_string_attribute(&mut self, object: &Value, name: &str) -> Option<String> {
        report_text(
            self.load_attribute_catching(object.clone(), name)
                .ok()?
                .ok()?,
        )
    }
}

/// Whether the exception carries a traceback, meaning it was raised rather
/// than only constructed (for example the target of `raise ... from X()`).
fn exception_was_raised(exception: &MiniException) -> bool {
    !matches!(exception_traceback_value(&exception.attrs), Value::None)
}

fn report_text(value: Value) -> Option<String> {
    match value {
        Value::String(text) | Value::IdentityString { value: text, .. } => Some(text),
        value => str_subclass_string(&value),
    }
}
