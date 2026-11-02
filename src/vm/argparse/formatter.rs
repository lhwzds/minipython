// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Lib/argparse.py (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! `HelpFormatter` and its subclasses. Formatting state lives in instance
//! attributes and every internal step is a method looked up on the
//! formatter, as in CPython, so subclasses can override any of them.

use super::super::*;
use super::textwrap;
use super::*;

/// Width CPython computes with `shutil.get_terminal_size()` when `COLUMNS`
/// is unset and stdout is not a terminal (80 columns), minus 2. The sandbox
/// never has a terminal.
const DEFAULT_WIDTH: i64 = 80 - 2;

/// Field names of the (always uncolored) `_colorize` argparse theme.
const THEME_FIELDS: &[&str] = &[
    "usage",
    "prog",
    "prog_extra",
    "heading",
    "summary_long_option",
    "summary_short_option",
    "summary_label",
    "summary_action",
    "long_option",
    "short_option",
    "label",
    "action",
    "reset",
];

/// The argparse theme without colors: every field is an empty string.
pub(super) fn no_color_theme() -> Value {
    Value::SimpleNamespace {
        fields: Rc::new(RefCell::new(DictStorage::new(
            THEME_FIELDS
                .iter()
                .map(|name| (str_value(*name), str_value("")))
                .collect(),
        ))),
    }
}

pub(super) fn indented_subactions_class() -> Value {
    let scope = new_scope();
    {
        let mut values = scope.borrow_mut();
        values.insert("__module__".to_string(), str_value("argparse"));
        values.insert(
            "__qualname__".to_string(),
            str_value("HelpFormatter._iter_indented_subactions"),
        );
        for method in ["__iter__", "__next__"] {
            values.insert(
                method.to_string(),
                Value::Builtin(format!("{ARGPARSE_PREFIX}_IndentedSubactions.{method}")),
            );
        }
    }
    Value::Class {
        name: "generator".to_string(),
        type_params: Vec::new(),
        metaclass: None,
        bases: vec![builtin_type_value("object")],
        attrs: scope,
    }
}

fn spaces(count: i64) -> String {
    " ".repeat(count.unsigned_abs() as usize)
}

fn char_count(text: &str) -> i64 {
    text.chars().count() as i64
}

impl Vm {
    fn ap_int_attr(&mut self, object: &Value, name: &str) -> Result<i64, String> {
        let value = self.ap_get(object, name)?;
        self.index_i64(value, name)
    }

    fn ap_add_int_attr(&mut self, object: &Value, name: &str, delta: Value) -> Result<i64, String> {
        let current = self.ap_get(object, name)?;
        let updated = self.add_values(current, delta)?;
        self.ap_set(object, name, updated.clone())?;
        self.index_i64(updated, name)
    }

    pub(super) fn argparse_formatter_method(
        &mut self,
        class: &str,
        method: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        match (class, method) {
            ("HelpFormatter", "__init__") => self.method_help_formatter_init(args, keywords),
            ("HelpFormatter", "_set_color") => self.method_help_formatter_set_color(args, keywords),
            ("HelpFormatter", "_indent") | ("HelpFormatter", "_dedent") => {
                self.method_help_formatter_dedent(method, args, keywords)
            }
            ("HelpFormatter", "_add_item") => self.method_help_formatter_add_item(args, keywords),
            ("HelpFormatter", "start_section") => {
                self.method_help_formatter_start_section(args, keywords)
            }
            ("HelpFormatter", "end_section") => {
                self.method_help_formatter_end_section(args, keywords)
            }
            ("HelpFormatter", "add_text") => self.method_help_formatter_add_text(args, keywords),
            ("HelpFormatter", "add_usage") => self.method_help_formatter_add_usage(args, keywords),
            ("HelpFormatter", "add_argument") => {
                self.method_help_formatter_add_argument(args, keywords)
            }
            ("HelpFormatter", "add_arguments") => {
                self.method_help_formatter_add_arguments(args, keywords)
            }
            ("HelpFormatter", "format_help") => {
                self.method_help_formatter_format_help(args, keywords)
            }
            ("HelpFormatter", "_join_parts") => {
                self.method_help_formatter_join_parts(args, keywords)
            }
            ("HelpFormatter", "_format_usage") => {
                self.method_help_formatter_format_usage(args, keywords)
            }
            ("HelpFormatter", "_is_long_option") => {
                self.method_help_formatter_is_long_option(args, keywords)
            }
            ("HelpFormatter", "_get_actions_usage_parts") => {
                self.method_help_formatter_get_actions_usage_parts(args, keywords)
            }
            ("HelpFormatter", "_format_text") => {
                self.method_help_formatter_format_text(args, keywords)
            }
            ("HelpFormatter", "_format_action") => {
                self.method_help_formatter_format_action(args, keywords)
            }
            ("HelpFormatter", "_format_action_invocation") => {
                self.method_help_formatter_format_action_invocation(args, keywords)
            }
            ("HelpFormatter", "_metavar_formatter") => {
                self.method_help_formatter_metavar_formatter(args, keywords)
            }
            ("HelpFormatter", "_format_args") => {
                self.method_help_formatter_format_args(args, keywords)
            }
            ("HelpFormatter", "_expand_help") => {
                self.method_help_formatter_expand_help(args, keywords)
            }
            ("HelpFormatter", "_iter_indented_subactions") => {
                self.method_help_formatter_iter_indented_subactions(args, keywords)
            }
            ("HelpFormatter", "_split_lines") => {
                self.method_help_formatter_split_lines(args, keywords)
            }
            ("HelpFormatter", "_fill_text") => self.method_help_formatter_fill_text(args, keywords),
            ("HelpFormatter", "_get_help_string") => {
                self.method_help_formatter_get_help_string(args, keywords)
            }
            ("HelpFormatter", "_get_default_metavar_for_optional") => {
                self.method_help_formatter_get_default_metavar_for_optional(args, keywords)
            }
            ("HelpFormatter", "_get_default_metavar_for_positional") => {
                self.method_help_formatter_get_default_metavar_for_positional(args, keywords)
            }
            ("HelpFormatter._Section", "__init__") => {
                self.method_help_formatter_section_init(args, keywords)
            }
            ("HelpFormatter._Section", "format_help") => {
                self.method_help_formatter_section_format_help(args, keywords)
            }
            ("RawDescriptionHelpFormatter", "_fill_text") => {
                self.method_raw_description_help_formatter_fill_text(args, keywords)
            }
            ("RawTextHelpFormatter", "_split_lines") => {
                self.method_raw_text_help_formatter_split_lines(args, keywords)
            }
            ("ArgumentDefaultsHelpFormatter", "_get_help_string") => {
                self.method_argument_defaults_help_formatter_get_help_string(args, keywords)
            }
            ("MetavarTypeHelpFormatter", "_get_default_metavar_for_optional")
            | ("MetavarTypeHelpFormatter", "_get_default_metavar_for_positional") => self
                .method_metavar_type_help_formatter_get_default_metavar_for_positional(
                    method, args, keywords,
                ),
            _ => Err(format!("unknown builtin: argparse.{class}.{method}")),
        }
    }

    #[inline(never)]
    fn method_help_formatter_init(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "HelpFormatter.__init__",
                params: &[
                    "self",
                    "prog",
                    "indent_increment",
                    "max_help_position",
                    "width",
                    "color",
                ],
                required: 2,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        self.ap_formatter_init(bound)?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_help_formatter_set_color(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "HelpFormatter._set_color",
                params: &["self", "color"],
                required: 2,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        // The sandbox never writes to a terminal, so CPython's
        // `can_colorize()` is always false: no color theme applies.
        let receiver = bound.arg(0);
        // Like `_colorize.get_theme(force_no_color=True)`, every
        // formatter shares one theme object.
        let module = self.ap_module()?;
        let theme = match &module {
            Value::Module { attrs, .. } => attrs.borrow().get("\0_theme").cloned(),
            _ => None,
        }
        .unwrap_or_else(no_color_theme);
        self.ap_set(&receiver, "_theme", theme)?;
        let identity = self.ap_class("_identity")?; // module function
        self.ap_set(&receiver, "_decolor", identity)?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_help_formatter_dedent(
        &mut self,
        method: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let receiver = self.ap_bind_self(
            if method == "_indent" {
                "HelpFormatter._indent"
            } else {
                "HelpFormatter._dedent"
            },
            args,
            keywords,
        )?;
        if method == "_indent" {
            let increment = self.ap_get(&receiver, "_indent_increment")?;
            self.ap_add_int_attr(&receiver, "_current_indent", increment)?;
            self.ap_add_int_attr(&receiver, "_level", Value::Number(1))?;
        } else {
            let increment = self.ap_get(&receiver, "_indent_increment")?;
            let current = self.ap_get(&receiver, "_current_indent")?;
            let updated = self.subtract_values(current, increment)?;
            self.ap_set(&receiver, "_current_indent", updated.clone())?;
            if self.index_i64(updated, "_current_indent")? < 0 {
                return self.ap_error("AssertionError", "Indent decreased below 0.");
            }
            self.ap_add_int_attr(&receiver, "_level", Value::Number(-1))?;
        }
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_help_formatter_add_item(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "HelpFormatter._add_item",
                params: &["self", "func", "args"],
                required: 3,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let section = self.ap_get(&bound.arg(0), "_current_section")?;
        let items = self.ap_get(&section, "items")?;
        self.ap_method(
            &items,
            "append",
            vec![tuple_value(vec![bound.arg(1), bound.arg(2)])],
        )?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_help_formatter_start_section(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "HelpFormatter.start_section",
                params: &["self", "heading"],
                required: 2,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let receiver = bound.arg(0);
        self.ap_method(&receiver, "_indent", Vec::new())?;
        let section_class = self.ap_get(&receiver, "_Section")?;
        let current = self.ap_get(&receiver, "_current_section")?;
        let section =
            self.call_value(section_class, vec![receiver.clone(), current, bound.arg(1)])?;
        let format_help = self.ap_get(&section, "format_help")?;
        self.ap_method(
            &receiver,
            "_add_item",
            vec![format_help, list_value(Vec::new())],
        )?;
        self.ap_set(&receiver, "_current_section", section)?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_help_formatter_end_section(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let receiver = self.ap_bind_self("HelpFormatter.end_section", args, keywords)?;
        let current = self.ap_get(&receiver, "_current_section")?;
        let parent = self.ap_get(&current, "parent")?;
        self.ap_set(&receiver, "_current_section", parent)?;
        self.ap_method(&receiver, "_dedent", Vec::new())?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_help_formatter_add_text(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "HelpFormatter.add_text",
                params: &["self", "text"],
                required: 2,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let receiver = bound.arg(0);
        let text = bound.arg(1);
        if !is_suppress(&text) && !matches!(text, Value::None) {
            let format_text = self.ap_get(&receiver, "_format_text")?;
            self.ap_method(
                &receiver,
                "_add_item",
                vec![format_text, list_value(vec![text])],
            )?;
        }
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_help_formatter_add_usage(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "HelpFormatter.add_usage",
                params: &["self", "usage", "actions", "groups", "prefix"],
                required: 4,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let receiver = bound.arg(0);
        let usage = bound.arg(1);
        if !is_suppress(&usage) {
            let format_usage = self.ap_get(&receiver, "_format_usage")?;
            let item_args = tuple_value(vec![usage, bound.arg(2), bound.arg(3), bound.arg(4)]);
            self.ap_method(&receiver, "_add_item", vec![format_usage, item_args])?;
        }
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_help_formatter_add_argument(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let (receiver, action) =
            self.ap_bind_self_action("HelpFormatter.add_argument", args, keywords)?;
        self.ap_formatter_add_argument(&receiver, &action)?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_help_formatter_add_arguments(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "HelpFormatter.add_arguments",
                params: &["self", "actions"],
                required: 2,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let receiver = bound.arg(0);
        for action in self.ap_iter(&bound.arg(1))? {
            self.ap_method(&receiver, "add_argument", vec![action])?;
        }
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_help_formatter_format_help(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let receiver = self.ap_bind_self("HelpFormatter.format_help", args, keywords)?;
        let root = self.ap_get(&receiver, "_root_section")?;
        let help = self.ap_method(&root, "format_help", Vec::new())?;
        if !self.ap_truth(&help)? {
            return Ok(help);
        }
        let help = self.ap_text(&help)?;
        let help = collapse_long_breaks(&help);
        let trimmed = help.trim_matches('\n');
        Ok(str_value(format!("{trimmed}\n")))
    }

    #[inline(never)]
    fn method_help_formatter_join_parts(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "HelpFormatter._join_parts",
                params: &["self", "part_strings"],
                required: 2,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let mut result = String::new();
        for part in self.ap_iter(&bound.arg(1))? {
            if !self.ap_truth(&part)? || is_suppress(&part) {
                continue;
            }
            match string_of(&part) {
                Some(text) => result.push_str(text),
                None => {
                    return self.ap_error(
                        "TypeError",
                        format!(
                            "sequence item 0: expected str instance, {} found",
                            type_name(&part)
                        ),
                    );
                }
            }
        }
        Ok(str_value(result))
    }

    #[inline(never)]
    fn method_help_formatter_format_usage(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "HelpFormatter._format_usage",
                params: &["self", "usage", "actions", "groups", "prefix"],
                required: 5,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        self.ap_format_usage(
            &bound.arg(0),
            bound.arg(1),
            bound.arg(2),
            bound.arg(3),
            bound.arg(4),
        )
    }

    #[inline(never)]
    fn method_help_formatter_is_long_option(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "HelpFormatter._is_long_option",
                params: &["self", "string"],
                required: 2,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        Ok(Value::Bool(self.ap_len(&bound.arg(1))? > 2))
    }

    #[inline(never)]
    fn method_help_formatter_get_actions_usage_parts(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "HelpFormatter._get_actions_usage_parts",
                params: &["self", "actions", "groups"],
                required: 3,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let (parts, pos_start) =
            self.ap_actions_usage_parts(&bound.arg(0), &bound.arg(1), &bound.arg(2))?;
        Ok(tuple_value(vec![
            list_value(parts.into_iter().map(str_value).collect()),
            Value::Number(pos_start as i64),
        ]))
    }

    #[inline(never)]
    fn method_help_formatter_format_text(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "HelpFormatter._format_text",
                params: &["self", "text"],
                required: 2,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let receiver = bound.arg(0);
        let mut text = bound.arg(1);
        if self.ap_contains(&text, &str_value("%(prog)"))? {
            let prog = self.ap_get(&receiver, "_prog")?;
            let format = self.ap_text(&text)?;
            text =
                str_value(self.ap_percent(&format, dict_value(vec![(str_value("prog"), prog)]))?);
        }
        let width = self.ap_int_attr(&receiver, "_width")?;
        let current_indent = self.ap_int_attr(&receiver, "_current_indent")?;
        let text_width = (width - current_indent).max(11);
        let indent = spaces(current_indent.max(0));
        let filled = self.ap_method(
            &receiver,
            "_fill_text",
            vec![text, Value::Number(text_width), str_value(indent)],
        )?;
        self.add_values(filled, str_value("\n\n"))
    }

    #[inline(never)]
    fn method_help_formatter_format_action(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let (receiver, action) =
            self.ap_bind_self_action("HelpFormatter._format_action", args, keywords)?;
        self.ap_format_action(&receiver, &action)
    }

    #[inline(never)]
    fn method_help_formatter_format_action_invocation(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let (receiver, action) =
            self.ap_bind_self_action("HelpFormatter._format_action_invocation", args, keywords)?;
        self.ap_format_action_invocation(&receiver, &action)
    }

    #[inline(never)]
    fn method_help_formatter_metavar_formatter(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "HelpFormatter._metavar_formatter",
                params: &["self", "action", "default_metavar"],
                required: 3,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let action = bound.arg(1);
        let metavar = self.ap_get(&action, "metavar")?;
        let result = if !matches!(metavar, Value::None) {
            metavar
        } else {
            let choices = self.ap_get(&action, "choices")?;
            if !matches!(choices, Value::None) {
                let mut names = Vec::new();
                for choice in self.ap_iter(&choices)? {
                    names.push(self.ap_str(&choice)?);
                }
                str_value(format!("{{{}}}", names.join(",")))
            } else {
                bound.arg(2)
            }
        };
        Ok(Value::Partial {
            function: Box::new(Value::Builtin(format!("{ARGPARSE_PREFIX}_metavar_format"))),
            args: vec![result],
            keywords: Rc::new(RefCell::new(DictStorage::new(Vec::new()))),
            attrs: new_scope(),
            identity: Rc::new(()),
        })
    }

    #[inline(never)]
    fn method_help_formatter_format_args(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "HelpFormatter._format_args",
                params: &["self", "action", "default_metavar"],
                required: 3,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        self.ap_format_args(&bound.arg(0), &bound.arg(1), bound.arg(2))
    }

    #[inline(never)]
    fn method_help_formatter_expand_help(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let (receiver, action) =
            self.ap_bind_self_action("HelpFormatter._expand_help", args, keywords)?;
        self.ap_expand_help(&receiver, &action)
    }

    #[inline(never)]
    fn method_help_formatter_iter_indented_subactions(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let (receiver, action) =
            self.ap_bind_self_action("HelpFormatter._iter_indented_subactions", args, keywords)?;
        let module = self.ap_module()?;
        let class = match &module {
            Value::Module { attrs, .. } => attrs
                .borrow()
                .get("\0_IndentedSubactions")
                .cloned()
                .unwrap_or_else(indented_subactions_class),
            _ => indented_subactions_class(),
        };
        let Value::Class {
            name, attrs, bases, ..
        } = class
        else {
            return Err("argparse iterator class is missing".to_string());
        };
        let fields = new_scope();
        {
            let mut values = fields.borrow_mut();
            values.insert("\0formatter".to_string(), receiver);
            values.insert("\0action".to_string(), action);
            values.insert("\0stage".to_string(), Value::Number(0));
            values.insert("\0items".to_string(), list_value(Vec::new()));
            values.insert("\0index".to_string(), Value::Number(0));
        }
        Ok(Value::Instance {
            class_name: name,
            fields,
            class_attrs: attrs,
            class_bases: bases,
        })
    }

    #[inline(never)]
    fn method_help_formatter_split_lines(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "HelpFormatter._split_lines",
                params: &["self", "text", "width"],
                required: 3,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let text = self.ap_text(&bound.arg(1))?;
        let width = self.index_i64(bound.arg(2), "width")?;
        let text = textwrap::collapse_whitespace(&text);
        let lines = textwrap::wrap(&text, width)?;
        Ok(list_value(lines.into_iter().map(str_value).collect()))
    }

    #[inline(never)]
    fn method_help_formatter_fill_text(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "HelpFormatter._fill_text",
                params: &["self", "text", "width", "indent"],
                required: 4,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let text = self.ap_text(&bound.arg(1))?;
        let width = self.index_i64(bound.arg(2), "width")?;
        let indent = self.ap_text(&bound.arg(3))?;
        let text = textwrap::collapse_whitespace(&text);
        Ok(str_value(textwrap::fill(&text, width, &indent)?))
    }

    #[inline(never)]
    fn method_help_formatter_get_help_string(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let (_, action) =
            self.ap_bind_self_action("HelpFormatter._get_help_string", args, keywords)?;
        self.ap_get(&action, "help")
    }

    #[inline(never)]
    fn method_help_formatter_get_default_metavar_for_optional(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let (_, action) = self.ap_bind_self_action(
            "HelpFormatter._get_default_metavar_for_optional",
            args,
            keywords,
        )?;
        let dest = self.ap_get(&action, "dest")?;
        self.ap_method(&dest, "upper", Vec::new())
    }

    #[inline(never)]
    fn method_help_formatter_get_default_metavar_for_positional(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let (_, action) = self.ap_bind_self_action(
            "HelpFormatter._get_default_metavar_for_positional",
            args,
            keywords,
        )?;
        self.ap_get(&action, "dest")
    }

    #[inline(never)]
    fn method_help_formatter_section_init(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "HelpFormatter._Section.__init__",
                params: &["self", "formatter", "parent", "heading"],
                required: 3,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let receiver = bound.arg(0);
        self.ap_set(&receiver, "formatter", bound.arg(1))?;
        self.ap_set(&receiver, "parent", bound.arg(2))?;
        self.ap_set(&receiver, "heading", bound.arg(3))?;
        self.ap_set(&receiver, "items", list_value(Vec::new()))?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_help_formatter_section_format_help(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let receiver = self.ap_bind_self("HelpFormatter._Section.format_help", args, keywords)?;
        self.ap_section_format_help(&receiver)
    }

    #[inline(never)]
    fn method_raw_description_help_formatter_fill_text(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "RawDescriptionHelpFormatter._fill_text",
                params: &["self", "text", "width", "indent"],
                required: 4,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let indent = self.ap_text(&bound.arg(3))?;
        let lines = self.ap_method_kw(
            &bound.arg(1),
            "splitlines",
            Vec::new(),
            vec![("keepends".to_string(), Value::Bool(true))],
        )?;
        let mut result = String::new();
        for line in self.ap_iter(&lines)? {
            result.push_str(&indent);
            result.push_str(&self.ap_text(&line)?);
        }
        Ok(str_value(result))
    }

    #[inline(never)]
    fn method_raw_text_help_formatter_split_lines(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "RawTextHelpFormatter._split_lines",
                params: &["self", "text", "width"],
                required: 3,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        self.ap_method(&bound.arg(1), "splitlines", Vec::new())
    }

    #[inline(never)]
    fn method_argument_defaults_help_formatter_get_help_string(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let (_, action) = self.ap_bind_self_action(
            "ArgumentDefaultsHelpFormatter._get_help_string",
            args,
            keywords,
        )?;
        let mut help = self.ap_get(&action, "help")?;
        if matches!(help, Value::None) {
            help = str_value("");
        }
        if !self.ap_contains(&help, &str_value("%(default)"))? {
            let default = self.ap_get(&action, "default")?;
            let required = self.ap_get(&action, "required")?;
            if !is_suppress(&default) && !self.ap_truth(&required)? {
                let option_strings = self.ap_get(&action, "option_strings")?;
                let nargs = self.ap_get(&action, "nargs")?;
                let defaulting = self.ap_truth(&option_strings)?
                    || matches!(nargs_kind(&nargs), Nargs::Optional | Nargs::ZeroOrMore);
                if defaulting {
                    help = self.add_values(help, str_value(" (default: %(default)s)"))?;
                }
            }
        }
        Ok(help)
    }

    #[inline(never)]
    fn method_metavar_type_help_formatter_get_default_metavar_for_positional(
        &mut self,
        method: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let (_, action) = self.ap_bind_self_action(
            if method == "_get_default_metavar_for_optional" {
                "MetavarTypeHelpFormatter._get_default_metavar_for_optional"
            } else {
                "MetavarTypeHelpFormatter._get_default_metavar_for_positional"
            },
            args,
            keywords,
        )?;
        let type_value = self.ap_get(&action, "type")?;
        self.ap_get(&type_value, "__name__")
    }

    /// `HelpFormatter.__init__`.
    fn ap_formatter_init(&mut self, bound: Bound) -> Result<(), String> {
        let receiver = bound.arg(0);
        let indent_increment = bound.or(2, Value::Number(2));
        let max_help_position = bound.or(3, Value::Number(24));
        let width = match bound.arg(4) {
            Value::None => Value::Number(DEFAULT_WIDTH),
            width => width,
        };
        self.ap_method(
            &receiver,
            "_set_color",
            vec![bound.or(5, Value::Bool(true))],
        )?;
        self.ap_set(&receiver, "_prog", bound.arg(1))?;
        self.ap_set(&receiver, "_indent_increment", indent_increment.clone())?;
        let width_int = self.index_i64(width.clone(), "width")?;
        let increment_int = self.index_i64(indent_increment, "indent_increment")?;
        let max_help_int = self.index_i64(max_help_position.clone(), "max_help_position")?;
        let limit = (width_int - 20).max(increment_int * 2);
        let max_help = if max_help_int <= limit {
            max_help_position
        } else {
            Value::Number(limit)
        };
        self.ap_set(&receiver, "_max_help_position", max_help)?;
        self.ap_set(&receiver, "_width", width)?;
        self.ap_set(&receiver, "_current_indent", Value::Number(0))?;
        self.ap_set(&receiver, "_level", Value::Number(0))?;
        self.ap_set(&receiver, "_action_max_length", Value::Number(0))?;
        let section_class = self.ap_get(&receiver, "_Section")?;
        let root = self.call_value(section_class, vec![receiver.clone(), Value::None])?;
        self.ap_set(&receiver, "_root_section", root.clone())?;
        self.ap_set(&receiver, "_current_section", root)?;
        Ok(())
    }

    /// `_metavar_formatter(...)(tuple_size)`.
    pub(super) fn ap_metavar_format(
        &mut self,
        result: Value,
        tuple_size: Value,
    ) -> Result<Value, String> {
        if matches!(result, Value::Tuple(_)) {
            return Ok(result);
        }
        let size = self.index_i64(tuple_size, "tuple_size")?.max(0) as usize;
        Ok(tuple_value(vec![result; size]))
    }

    /// Calls the formatter's `_metavar_formatter(action, default)(size)`.
    fn ap_get_metavar(
        &mut self,
        receiver: &Value,
        action: &Value,
        default: Value,
        size: i64,
    ) -> Result<Value, String> {
        let formatter = self.ap_method(
            receiver,
            "_metavar_formatter",
            vec![action.clone(), default],
        )?;
        self.call_value(formatter, vec![Value::Number(size)])
    }

    /// `HelpFormatter._format_args`.
    fn ap_format_args(
        &mut self,
        receiver: &Value,
        action: &Value,
        default: Value,
    ) -> Result<Value, String> {
        let nargs = self.ap_get(action, "nargs")?;
        let text = match nargs_kind(&nargs) {
            Nargs::Single => {
                let metavar = self.ap_get_metavar(receiver, action, default, 1)?;
                self.ap_percent("%s", metavar)?
            }
            Nargs::Optional => {
                let metavar = self.ap_get_metavar(receiver, action, default, 1)?;
                self.ap_percent("[%s]", metavar)?
            }
            Nargs::ZeroOrMore => {
                let metavar = self.ap_get_metavar(receiver, action, default, 1)?;
                if self.ap_len(&metavar)? == 2 {
                    self.ap_percent("[%s [%s ...]]", metavar)?
                } else {
                    self.ap_percent("[%s ...]", metavar)?
                }
            }
            Nargs::OneOrMore => {
                let metavar = self.ap_get_metavar(receiver, action, default, 2)?;
                self.ap_percent("%s [%s ...]", metavar)?
            }
            Nargs::Remainder => "...".to_string(),
            Nargs::Parser => {
                let metavar = self.ap_get_metavar(receiver, action, default, 1)?;
                self.ap_percent("%s ...", metavar)?
            }
            Nargs::Suppress => String::new(),
            Nargs::Count(count) => {
                let metavar = self.ap_get_metavar(receiver, action, default, count)?;
                let format = vec!["%s"; count.max(0) as usize].join(" ");
                self.ap_percent(&format, metavar)?
            }
            Nargs::Other => return self.ap_error("ValueError", "invalid nargs value"),
        };
        Ok(str_value(text))
    }

    /// `HelpFormatter._format_action_invocation`.
    fn ap_format_action_invocation(
        &mut self,
        receiver: &Value,
        action: &Value,
    ) -> Result<Value, String> {
        let option_strings = self.ap_get(action, "option_strings")?;
        if !self.ap_truth(&option_strings)? {
            let default = self.ap_method(
                receiver,
                "_get_default_metavar_for_positional",
                vec![action.clone()],
            )?;
            let metavar = self.ap_get_metavar(receiver, action, default, 1)?;
            let separator = str_value(" ");
            return self.ap_method(&separator, "join", vec![metavar]);
        }
        let mut names = Vec::new();
        for option_string in self.ap_iter(&option_strings)? {
            names.push(self.ap_str(&option_string)?);
        }
        let joined = names.join(", ");
        let nargs = self.ap_get(action, "nargs")?;
        if self.ap_eq(&nargs, &Value::Number(0))? {
            return Ok(str_value(joined));
        }
        let default = self.ap_method(
            receiver,
            "_get_default_metavar_for_optional",
            vec![action.clone()],
        )?;
        let args_string =
            self.ap_method(receiver, "_format_args", vec![action.clone(), default])?;
        let args_string = self.ap_str(&args_string)?;
        Ok(str_value(format!("{joined} {args_string}")))
    }

    /// The subactions of `action` when it has `_get_subactions`.
    fn ap_iter_indented(&mut self, receiver: &Value, action: &Value) -> Result<Value, String> {
        let iterator =
            self.ap_method(receiver, "_iter_indented_subactions", vec![action.clone()])?;
        Ok(iterator)
    }

    /// `HelpFormatter.add_argument`.
    fn ap_formatter_add_argument(
        &mut self,
        receiver: &Value,
        action: &Value,
    ) -> Result<(), String> {
        let help = self.ap_get(action, "help")?;
        if is_suppress(&help) {
            return Ok(());
        }
        let current_indent = self.ap_int_attr(receiver, "_current_indent")?;
        let invocation = self.ap_invocation_length(receiver, action)?;
        let mut lengths = vec![invocation + current_indent];
        let iterator = self.ap_iter_indented(receiver, action)?;
        let mut iterator = self.get_iter(iterator)?;
        loop {
            match self.advance_owned_iterator(&mut iterator)? {
                IteratorAdvance::Yield(subaction) => {
                    let length = self.ap_invocation_length(receiver, &subaction)?;
                    let indent = self.ap_int_attr(receiver, "_current_indent")?;
                    lengths.push(length + indent);
                }
                IteratorAdvance::Complete(_) | IteratorAdvance::Raised => break,
            }
        }
        let action_length = lengths.into_iter().max().unwrap_or(0);
        let max_length = self.ap_int_attr(receiver, "_action_max_length")?;
        self.ap_set(
            receiver,
            "_action_max_length",
            Value::Number(max_length.max(action_length)),
        )?;
        let format_action = self.ap_get(receiver, "_format_action")?;
        self.ap_method(
            receiver,
            "_add_item",
            vec![format_action, list_value(vec![action.clone()])],
        )?;
        Ok(())
    }

    fn ap_invocation_length(&mut self, receiver: &Value, action: &Value) -> Result<i64, String> {
        let invocation =
            self.ap_method(receiver, "_format_action_invocation", vec![action.clone()])?;
        let decolored = self.ap_method(receiver, "_decolor", vec![invocation])?;
        Ok(self.ap_len(&decolored)? as i64)
    }

    /// `HelpFormatter._format_action`.
    fn ap_format_action(&mut self, receiver: &Value, action: &Value) -> Result<Value, String> {
        let action_max_length = self.ap_int_attr(receiver, "_action_max_length")?;
        let max_help_position = self.ap_int_attr(receiver, "_max_help_position")?;
        let width = self.ap_int_attr(receiver, "_width")?;
        let current_indent = self.ap_int_attr(receiver, "_current_indent")?;
        let help_position = (action_max_length + 2).min(max_help_position);
        let help_width = (width - help_position).max(11);
        let action_width = help_position - current_indent - 2;
        let header = self.ap_method(receiver, "_format_action_invocation", vec![action.clone()])?;
        let header_plain = self.ap_method(receiver, "_decolor", vec![header.clone()])?;
        let header_text = self.ap_str(&header)?;
        let header_plain_text = self.ap_str(&header_plain)?;
        let help = self.ap_get(action, "help")?;
        let indent_first;
        let action_header = if !self.ap_truth(&help)? {
            indent_first = 0;
            format!("{}{header_text}\n", spaces(current_indent))
        } else if char_count(&header_plain_text) <= action_width {
            indent_first = 0;
            let padding = (action_width - char_count(&header_plain_text)).max(0);
            let padded = format!(
                "{}{header_plain_text}{}  ",
                spaces(current_indent),
                spaces(padding)
            );
            if header_plain_text.is_empty() {
                padded
            } else {
                padded.replace(&header_plain_text, &header_text)
            }
        } else {
            indent_first = help_position;
            format!("{}{header_text}\n", spaces(current_indent))
        };
        let mut parts = vec![str_value(action_header.clone())];
        let help_present = self.ap_truth(&help)? && {
            let stripped = self.ap_method(&help, "strip", Vec::new())?;
            self.ap_truth(&stripped)?
        };
        if help_present {
            let help_text = self.ap_method(receiver, "_expand_help", vec![action.clone()])?;
            if self.ap_truth(&help_text)? {
                let lines = self.ap_method(
                    receiver,
                    "_split_lines",
                    vec![help_text, Value::Number(help_width)],
                )?;
                let lines = self.ap_list_items(&lines)?;
                let Some(first) = lines.first() else {
                    return self.ap_error("IndexError", "list index out of range");
                };
                let first = self.ap_str(first)?;
                parts.push(str_value(format!("{}{first}\n", spaces(indent_first))));
                for line in &lines[1..] {
                    let line = self.ap_str(line)?;
                    parts.push(str_value(format!("{}{line}\n", spaces(help_position))));
                }
            }
        } else if !action_header.ends_with('\n') {
            parts.push(str_value("\n"));
        }
        let iterator = self.ap_iter_indented(receiver, action)?;
        let mut iterator = self.get_iter(iterator)?;
        loop {
            match self.advance_owned_iterator(&mut iterator)? {
                IteratorAdvance::Yield(subaction) => {
                    let formatted = self.ap_method(receiver, "_format_action", vec![subaction])?;
                    parts.push(formatted);
                }
                IteratorAdvance::Complete(_) | IteratorAdvance::Raised => break,
            }
        }
        self.ap_method(receiver, "_join_parts", vec![list_value(parts)])
    }

    /// `HelpFormatter._expand_help`.
    fn ap_expand_help(&mut self, receiver: &Value, action: &Value) -> Result<Value, String> {
        let help_string = self.ap_method(receiver, "_get_help_string", vec![action.clone()])?;
        if !self.ap_contains(&help_string, &str_value("%"))? {
            return Ok(help_string);
        }
        let mut params = self.ap_instance_dict_items(action)?;
        let prog = self.ap_get(receiver, "_prog")?;
        match params
            .iter_mut()
            .find(|(key, _)| matches!(string_of(key), Some("prog")))
        {
            Some(entry) => entry.1 = prog,
            None => params.push((str_value("prog"), prog)),
        }
        let mut expanded = Vec::new();
        for (name, value) in params {
            if is_suppress(&value) {
                continue;
            }
            match self.ap_get_opt(&value, "__name__")? {
                Some(name_value) => expanded.push((name, name_value)),
                None => expanded.push((name, value)),
            }
        }
        if let Some(entry) = expanded
            .iter_mut()
            .find(|(key, _)| matches!(string_of(key), Some("choices")))
            && !matches!(entry.1, Value::None)
        {
            let mut names = Vec::new();
            for choice in self.ap_iter(&entry.1.clone())? {
                names.push(self.ap_str(&choice)?);
            }
            entry.1 = str_value(names.join(", "));
        }
        let format = self.ap_text(&help_string)?;
        Ok(str_value(self.ap_percent(&format, dict_value(expanded))?))
    }

    /// `HelpFormatter._Section.format_help`.
    fn ap_section_format_help(&mut self, receiver: &Value) -> Result<Value, String> {
        let formatter = self.ap_get(receiver, "formatter")?;
        let parent = self.ap_get(receiver, "parent")?;
        let has_parent = !matches!(parent, Value::None);
        if has_parent {
            self.ap_method(&formatter, "_indent", Vec::new())?;
        }
        let join = self.ap_get(&formatter, "_join_parts")?;
        let items = self.ap_get(receiver, "items")?;
        let mut rendered = Vec::new();
        for item in self.ap_list_items(&items)? {
            let pair = self.ap_list_items(&item)?;
            let [func, item_args] = pair.as_slice() else {
                return self.ap_error("ValueError", "too many values to unpack (expected 2)");
            };
            let item_args = self.ap_list_items(item_args)?;
            rendered.push(self.ap_invoke(func.clone(), item_args, Vec::new())?);
        }
        let item_help = self.ap_invoke(join.clone(), vec![list_value(rendered)], Vec::new())?;
        if has_parent {
            self.ap_method(&formatter, "_dedent", Vec::new())?;
        }
        if !self.ap_truth(&item_help)? {
            return Ok(str_value(""));
        }
        let heading = self.ap_get(receiver, "heading")?;
        let heading_text = if !is_suppress(&heading) && !matches!(heading, Value::None) {
            let current_indent = self.ap_int_attr(&formatter, "_current_indent")?;
            let heading = self.ap_str(&heading)?;
            format!("{}{heading}:\n", spaces(current_indent))
        } else {
            String::new()
        };
        self.ap_invoke(
            join,
            vec![list_value(vec![
                str_value("\n"),
                str_value(heading_text),
                item_help,
                str_value("\n"),
            ])],
            Vec::new(),
        )
    }

    /// `HelpFormatter._format_usage`.
    fn ap_format_usage(
        &mut self,
        receiver: &Value,
        usage: Value,
        actions: Value,
        groups: Value,
        prefix: Value,
    ) -> Result<Value, String> {
        let prefix = match prefix {
            Value::None => "usage: ".to_string(),
            prefix => self.ap_str(&prefix)?,
        };
        let prog_value = self.ap_get(receiver, "_prog")?;
        let usage_text = if !matches!(usage, Value::None) {
            let prog = self.ap_str(&prog_value)?;
            let format = self.ap_text(&usage)?;
            self.ap_percent(
                &format,
                dict_value(vec![(str_value("prog"), str_value(prog))]),
            )?
        } else if !self.ap_truth(&actions)? {
            self.ap_str(&prog_value)?
        } else {
            let prog = self.ap_str(&prog_value)?;
            let result =
                self.ap_method(receiver, "_get_actions_usage_parts", vec![actions, groups])?;
            let pair = self.ap_list_items(&result)?;
            let [parts_value, pos_start_value] = pair.as_slice() else {
                return self.ap_error("ValueError", "too many values to unpack (expected 2)");
            };
            let mut parts = Vec::new();
            for part in self.ap_list_items(parts_value)? {
                parts.push(self.ap_text(&part)?);
            }
            let pos_start = (self.index_i64(pos_start_value.clone(), "pos_start")?.max(0) as usize)
                .min(parts.len());
            let mut usage = std::iter::once(prog.clone())
                .chain(parts.iter().cloned())
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            let width = self.ap_int_attr(receiver, "_width")?;
            let current_indent = self.ap_int_attr(receiver, "_current_indent")?;
            let text_width = width - current_indent;
            if char_count(&prefix) + char_count(&usage) > text_width {
                let opt_parts = &parts[..pos_start];
                let pos_parts = &parts[pos_start..];
                let get_lines =
                    |parts: &[String], indent: &str, prefix: Option<&str>| -> Vec<String> {
                        let mut lines = Vec::new();
                        let mut line: Vec<&str> = Vec::new();
                        let indent_length = char_count(indent);
                        let mut line_len = match prefix {
                            Some(prefix) => char_count(prefix) - 1,
                            None => indent_length - 1,
                        };
                        for part in parts {
                            let part_len = char_count(part);
                            if line_len + 1 + part_len > text_width && !line.is_empty() {
                                lines.push(format!("{indent}{}", line.join(" ")));
                                line.clear();
                                line_len = indent_length - 1;
                            }
                            line.push(part);
                            line_len += part_len + 1;
                        }
                        if !line.is_empty() {
                            lines.push(format!("{indent}{}", line.join(" ")));
                        }
                        if prefix.is_some()
                            && let Some(first) = lines.first_mut()
                        {
                            *first = first.chars().skip(indent_length.max(0) as usize).collect();
                        }
                        lines
                    };
                let prog_len = char_count(&prog);
                let lines = if (char_count(&prefix) + prog_len) as f64 <= 0.75 * text_width as f64 {
                    let indent = spaces(char_count(&prefix) + prog_len + 1);
                    if !opt_parts.is_empty() {
                        let mut first = vec![prog.clone()];
                        first.extend(opt_parts.iter().cloned());
                        let mut lines = get_lines(&first, &indent, Some(&prefix));
                        lines.extend(get_lines(pos_parts, &indent, None));
                        lines
                    } else if !pos_parts.is_empty() {
                        let mut first = vec![prog.clone()];
                        first.extend(pos_parts.iter().cloned());
                        get_lines(&first, &indent, Some(&prefix))
                    } else {
                        vec![prog.clone()]
                    }
                } else {
                    let indent = spaces(char_count(&prefix));
                    let all_parts = parts.clone();
                    let mut lines = get_lines(&all_parts, &indent, None);
                    if lines.len() > 1 {
                        lines = get_lines(opt_parts, &indent, None);
                        lines.extend(get_lines(pos_parts, &indent, None));
                    }
                    let mut with_prog = vec![prog.clone()];
                    with_prog.extend(lines);
                    with_prog
                };
                usage = lines.join("\n");
            }
            let rest = usage
                .strip_prefix(prog.as_str())
                .unwrap_or(&usage)
                .to_string();
            format!("{prog}{rest}")
        };
        Ok(str_value(format!("{prefix}{usage_text}\n\n")))
    }

    /// `HelpFormatter._get_actions_usage_parts`.
    fn ap_actions_usage_parts(
        &mut self,
        receiver: &Value,
        actions: &Value,
        groups: &Value,
    ) -> Result<(Vec<String>, usize), String> {
        let mut visible = Vec::new();
        for action in self.ap_iter(actions)? {
            let help = self.ap_get(&action, "help")?;
            if !is_suppress(&help) {
                visible.push(action);
            }
        }
        // action -> group (None when the action is in no mutex group).
        let mut action_groups: Vec<(Value, Option<Value>)> = Vec::new();
        for action in &visible {
            if !action_groups
                .iter()
                .any(|(key, _)| same_object(key, action))
            {
                action_groups.push((action.clone(), None));
            }
        }
        for group in self.ap_iter(groups)? {
            let group_actions = self.ap_get(&group, "_group_actions")?;
            for action in self.ap_list_items(&group_actions)? {
                if let Some(entry) = action_groups
                    .iter_mut()
                    .find(|(key, _)| same_object(key, &action))
                {
                    entry.1 = Some(group.clone());
                }
            }
        }
        fn pop(map: &mut Vec<(Value, Option<Value>)>, action: &Value) -> Option<Option<Value>> {
            let index = map.iter().position(|(key, _)| same_object(key, action))?;
            Some(map.remove(index).1)
        }

        let mut positionals: Vec<(Value, Vec<Value>)> = Vec::new();
        for action in &visible {
            let option_strings = self.ap_get(action, "option_strings")?;
            if self.ap_truth(&option_strings)? {
                continue;
            }
            let Some(group) = pop(&mut action_groups, action) else {
                let rendered = self.ap_repr(action)?;
                return self.ap_error("KeyError", rendered);
            };
            match group {
                Some(group) => {
                    let mut group_actions = Vec::new();
                    let members = self.ap_get(&group, "_group_actions")?;
                    for member in self.ap_list_items(&members)? {
                        let member_options = self.ap_get(&member, "option_strings")?;
                        if self.ap_truth(&member_options)?
                            && pop(&mut action_groups, &member).flatten().is_some()
                        {
                            group_actions.push(member);
                        }
                    }
                    group_actions.push(action.clone());
                    let required = self.ap_get(&group, "required")?;
                    positionals.push((required, group_actions));
                }
                None => positionals.push((Value::None, vec![action.clone()])),
            }
        }
        let mut optionals: Vec<(Value, Vec<Value>)> = Vec::new();
        for action in &visible {
            let option_strings = self.ap_get(action, "option_strings")?;
            if !self.ap_truth(&option_strings)? {
                continue;
            }
            let Some(group) = pop(&mut action_groups, action) else {
                continue;
            };
            match group {
                Some(group) => {
                    let mut group_actions = vec![action.clone()];
                    let members = self.ap_get(&group, "_group_actions")?;
                    for member in self.ap_list_items(&members)? {
                        let member_options = self.ap_get(&member, "option_strings")?;
                        if self.ap_truth(&member_options)?
                            && pop(&mut action_groups, &member).flatten().is_some()
                        {
                            group_actions.push(member);
                        }
                    }
                    let required = self.ap_get(&group, "required")?;
                    optionals.push((required, group_actions));
                }
                None => optionals.push((Value::None, vec![action.clone()])),
            }
        }

        let optional_count = optionals.len();
        let mut parts: Vec<String> = Vec::new();
        let mut pos_start = None;
        for (index, (required, group)) in optionals.into_iter().chain(positionals).enumerate() {
            let start = parts.len();
            if index == optional_count {
                pos_start = Some(start);
            }
            let required = self.ap_truth(&required)?;
            let in_group = group.len() > 1;
            for action in &group {
                let option_strings = self.ap_get(action, "option_strings")?;
                let part = if !self.ap_truth(&option_strings)? {
                    let default = self.ap_method(
                        receiver,
                        "_get_default_metavar_for_positional",
                        vec![action.clone()],
                    )?;
                    let part =
                        self.ap_method(receiver, "_format_args", vec![action.clone(), default])?;
                    let mut part = self.ap_text(&part)?;
                    if in_group {
                        if part.is_empty() {
                            return self.ap_error("IndexError", "string index out of range");
                        }
                        if part.starts_with('[') && part.ends_with(']') {
                            let characters = part.chars().collect::<Vec<_>>();
                            part = if characters.len() >= 2 {
                                characters[1..characters.len() - 1].iter().collect()
                            } else {
                                String::new()
                            };
                        }
                    }
                    part
                } else {
                    let option_string =
                        self.load_subscript_value(option_strings, Value::Number(0))?;
                    let option_string = self.ap_str(&option_string)?;
                    let nargs = self.ap_get(action, "nargs")?;
                    let mut part = if self.ap_eq(&nargs, &Value::Number(0))? {
                        let usage = self.ap_method(action, "format_usage", Vec::new())?;
                        self.ap_str(&usage)?
                    } else {
                        let default = self.ap_method(
                            receiver,
                            "_get_default_metavar_for_optional",
                            vec![action.clone()],
                        )?;
                        let args_string = self.ap_method(
                            receiver,
                            "_format_args",
                            vec![action.clone(), default],
                        )?;
                        let args_string = self.ap_str(&args_string)?;
                        format!("{option_string} {args_string}")
                    };
                    let action_required = self.ap_get(action, "required")?;
                    if !(self.ap_truth(&action_required)? || required || in_group) {
                        part = format!("[{part}]");
                    }
                    part
                };
                parts.push(part);
            }
            if in_group {
                let opener = if required { "(" } else { "[" };
                parts[start] = format!("{opener}{}", parts[start]);
                let last = parts.len() - 1;
                for part in parts.iter_mut().take(last).skip(start) {
                    part.push_str(" |");
                }
                parts[last].push_str(if required { ")" } else { "]" });
            }
        }
        let pos_start = pos_start.unwrap_or(parts.len());
        Ok((parts, pos_start))
    }

    /// `__next__` of the `_iter_indented_subactions` generator.
    pub(super) fn ap_indented_subactions_next(
        &mut self,
        receiver: &Value,
    ) -> Result<Value, String> {
        let Value::Instance { fields, .. } = receiver else {
            return self.ap_error("TypeError", "descriptor requires an iterator");
        };
        let field = |name: &str| fields.borrow().get(name).cloned().unwrap_or(Value::None);
        let stage = match field("\0stage") {
            Value::Number(stage) => stage,
            _ => 2,
        };
        if stage == 2 {
            return Err("StopIteration".to_string());
        }
        let formatter = field("\0formatter");
        if stage == 0 {
            let action = field("\0action");
            let Some(get_subactions) = self.ap_get_opt(&action, "_get_subactions")? else {
                fields
                    .borrow_mut()
                    .insert("\0stage".to_string(), Value::Number(2));
                return Err("StopIteration".to_string());
            };
            self.ap_method(&formatter, "_indent", Vec::new())?;
            let subactions = self.call_value(get_subactions, Vec::new())?;
            let items = self.ap_iter(&subactions)?;
            let mut values = fields.borrow_mut();
            values.insert("\0items".to_string(), list_value(items));
            values.insert("\0stage".to_string(), Value::Number(1));
        }
        let items = match field("\0items") {
            Value::List(items) => items.borrow().clone(),
            _ => Vec::new(),
        };
        let index = match field("\0index") {
            Value::Number(index) => index.max(0) as usize,
            _ => 0,
        };
        if let Some(item) = items.get(index) {
            fields
                .borrow_mut()
                .insert("\0index".to_string(), Value::Number(index as i64 + 1));
            return Ok(item.clone());
        }
        self.ap_method(&formatter, "_dedent", Vec::new())?;
        fields
            .borrow_mut()
            .insert("\0stage".to_string(), Value::Number(2));
        Err("StopIteration".to_string())
    }
}

/// `re.compile(r'\n\n\n+').sub('\n\n', text)`.
fn collapse_long_breaks(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut newlines = 0usize;
    for ch in text.chars() {
        if ch == '\n' {
            newlines += 1;
            continue;
        }
        if newlines > 0 {
            result.push_str(if newlines >= 3 {
                "\n\n"
            } else {
                &"\n\n"[..newlines]
            });
            newlines = 0;
        }
        result.push(ch);
    }
    if newlines > 0 {
        result.push_str(if newlines >= 3 {
            "\n\n"
        } else {
            &"\n\n"[..newlines]
        });
    }
    result
}
