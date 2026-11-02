// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Lib/argparse.py, Lib/difflib.py (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! `ArgumentParser`: construction, the command-line parsing algorithm,
//! value conversion, help printing, and exiting.

use super::super::*;
use super::actions::{set_keyword, take_keyword};
use super::container::keyword_dict;
use super::*;
use std::collections::BTreeMap;

const PARSER_INIT_PARAMS: &[&str] = &[
    "self",
    "prog",
    "usage",
    "description",
    "epilog",
    "parents",
    "formatter_class",
    "prefix_chars",
    "fromfile_prefix_chars",
    "argument_default",
    "conflict_handler",
    "add_help",
    "allow_abbrev",
    "exit_on_error",
];

/// One parsed option candidate: `(action, option_string, sep, explicit_arg)`.
struct OptionTuple {
    action: Value,
    option_string: Value,
    sep: Value,
    explicit_arg: Value,
}

impl Vm {
    pub(super) fn argparse_parser_method(
        &mut self,
        method: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        match method {
            "__init__" => self.method_argument_parser_init(args, keywords),
            "_get_kwargs" => self.method_argument_parser_get_kwargs(args, keywords),
            "add_subparsers" => self.method_argument_parser_add_subparsers(args, keywords),
            "_add_action" => self.method_argument_parser_add_action(args, keywords),
            "_get_optional_actions" | "_get_positional_actions" => {
                self.method_argument_parser_get_optional_actions(method, args, keywords)
            }
            "parse_args" | "parse_intermixed_args" => {
                self.method_argument_parser_parse_args(method, args, keywords)
            }
            "parse_known_args" => self.method_argument_parser_parse_known_args(args, keywords),
            "parse_known_intermixed_args" => {
                self.method_argument_parser_parse_known_intermixed_args(args, keywords)
            }
            "_parse_known_args2" => self.method_argument_parser_parse_known_args2(args, keywords),
            "_parse_known_args" => self.method_argument_parser_parse_known_args_(args, keywords),
            "_read_args_from_files" => {
                self.method_argument_parser_read_args_from_files(args, keywords)
            }
            "convert_arg_line_to_args" => {
                self.method_argument_parser_convert_arg_line_to_args(args, keywords)
            }
            "_match_argument" => self.method_argument_parser_match_argument(args, keywords),
            "_match_arguments_partial" => {
                self.method_argument_parser_match_arguments_partial(args, keywords)
            }
            "_parse_optional" => self.method_argument_parser_parse_optional(args, keywords),
            "_get_option_tuples" => self.method_argument_parser_get_option_tuples(args, keywords),
            "_get_nargs_pattern" => self.method_argument_parser_get_nargs_pattern(args, keywords),
            "_get_values" => self.method_argument_parser_get_values(args, keywords),
            "_get_value" => self.method_argument_parser_get_value(args, keywords),
            "_check_value" => self.method_argument_parser_check_value(args, keywords),
            "format_usage" => self.method_argument_parser_format_usage(args, keywords),
            "format_help" => self.method_argument_parser_format_help(args, keywords),
            "_get_formatter" => self.method_argument_parser_get_formatter(args, keywords),
            "_get_validation_formatter" => {
                self.method_argument_parser_get_validation_formatter(args, keywords)
            }
            "print_usage" | "print_help" => {
                self.method_argument_parser_print_usage(method, args, keywords)
            }
            "_print_message" => self.method_argument_parser_print_message(args, keywords),
            "exit" => self.method_argument_parser_exit(args, keywords),
            "error" => self.method_argument_parser_error(args, keywords),
            "_warning" => self.method_argument_parser_warning(args, keywords),
            _ => Err(format!("unknown builtin: argparse.ArgumentParser.{method}")),
        }
    }

    #[inline(never)]
    fn method_argument_parser_init(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "ArgumentParser.__init__",
                params: PARSER_INIT_PARAMS,
                required: 1,
                kwonly: &["suggest_on_error", "color"],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        self.ap_parser_init(bound)?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_argument_parser_get_kwargs(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let receiver = self.ap_bind_self("ArgumentParser._get_kwargs", args, keywords)?;
        let mut items = Vec::new();
        for name in [
            "prog",
            "usage",
            "description",
            "formatter_class",
            "conflict_handler",
            "add_help",
        ] {
            let value = self.ap_get(&receiver, name)?;
            items.push(tuple_value(vec![str_value(name), value]));
        }
        Ok(list_value(items))
    }

    #[inline(never)]
    fn method_argument_parser_add_subparsers(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "ArgumentParser.add_subparsers",
                params: &["self"],
                required: 1,
                kwonly: &[],
                varargs: false,
                varkw: true,
            },
            args,
            keywords,
        )?;
        self.ap_add_subparsers(&bound.arg(0), bound.extra_kwargs)
    }

    #[inline(never)]
    fn method_argument_parser_add_action(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let (receiver, action) =
            self.ap_bind_self_action("ArgumentParser._add_action", args, keywords)?;
        let option_strings = self.ap_get(&action, "option_strings")?;
        let group = if self.ap_truth(&option_strings)? {
            self.ap_get(&receiver, "_optionals")?
        } else {
            self.ap_get(&receiver, "_positionals")?
        };
        self.ap_method(&group, "_add_action", vec![action.clone()])?;
        Ok(action)
    }

    #[inline(never)]
    fn method_argument_parser_get_optional_actions(
        &mut self,
        method: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let receiver = self.ap_bind_self(
            if method == "_get_optional_actions" {
                "ArgumentParser._get_optional_actions"
            } else {
                "ArgumentParser._get_positional_actions"
            },
            args,
            keywords,
        )?;
        let want_optional = method == "_get_optional_actions";
        Ok(list_value(
            self.ap_filter_actions(&receiver, want_optional)?,
        ))
    }

    #[inline(never)]
    fn method_argument_parser_parse_args(
        &mut self,
        method: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: if method == "parse_args" {
                    "ArgumentParser.parse_args"
                } else {
                    "ArgumentParser.parse_intermixed_args"
                },
                params: &["self", "args", "namespace"],
                required: 1,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let receiver = bound.arg(0);
        let known = if method == "parse_args" {
            "parse_known_args"
        } else {
            "parse_known_intermixed_args"
        };
        let result = self.ap_method(&receiver, known, vec![bound.arg(1), bound.arg(2)])?;
        let (namespace, argv) = self.ap_unpack_pair(&result)?;
        if self.ap_truth(&argv)? {
            let separator = str_value(" ");
            let joined = self.ap_method(&separator, "join", vec![argv])?;
            let joined = self.ap_text(&joined)?;
            let message = format!("unrecognized arguments: {joined}");
            let exit_on_error = self.ap_get(&receiver, "exit_on_error")?;
            if self.ap_truth(&exit_on_error)? {
                self.ap_method(&receiver, "error", vec![str_value(message)])?;
            } else {
                return self.ap_raise_argument_error(Value::None, message);
            }
        }
        Ok(namespace)
    }

    #[inline(never)]
    fn method_argument_parser_parse_known_args(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "ArgumentParser.parse_known_args",
                params: &["self", "args", "namespace"],
                required: 1,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        self.ap_method(
            &bound.arg(0),
            "_parse_known_args2",
            vec![bound.arg(1), bound.arg(2), Value::Bool(false)],
        )
    }

    #[inline(never)]
    fn method_argument_parser_parse_known_intermixed_args(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "ArgumentParser.parse_known_intermixed_args",
                params: &["self", "args", "namespace"],
                required: 1,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let receiver = bound.arg(0);
        let positionals = self.ap_method(&receiver, "_get_positional_actions", Vec::new())?;
        for action in self.ap_iter(&positionals)? {
            let nargs = self.ap_get(&action, "nargs")?;
            if matches!(nargs_kind(&nargs), Nargs::Parser | Nargs::Remainder) {
                let rendered = self.ap_str(&nargs)?;
                return self.ap_error(
                    "TypeError",
                    format!("parse_intermixed_args: positional arg with nargs={rendered}"),
                );
            }
        }
        self.ap_method(
            &receiver,
            "_parse_known_args2",
            vec![bound.arg(1), bound.arg(2), Value::Bool(true)],
        )
    }

    #[inline(never)]
    fn method_argument_parser_parse_known_args2(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "ArgumentParser._parse_known_args2",
                params: &["self", "args", "namespace", "intermixed"],
                required: 4,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        self.ap_parse_known_args2(&bound.arg(0), bound.arg(1), bound.arg(2), bound.arg(3))
    }

    #[inline(never)]
    fn method_argument_parser_parse_known_args_(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "ArgumentParser._parse_known_args",
                params: &["self", "arg_strings", "namespace", "intermixed"],
                required: 4,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let intermixed = self.ap_truth(&bound.arg(3))?;
        let (namespace, extras) =
            self.ap_parse_known_args(&bound.arg(0), bound.arg(1), bound.arg(2), intermixed)?;
        Ok(tuple_value(vec![namespace, list_value(extras)]))
    }

    #[inline(never)]
    fn method_argument_parser_read_args_from_files(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "ArgumentParser._read_args_from_files",
                params: &["self", "arg_strings"],
                required: 2,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        self.ap_read_args_from_files(&bound.arg(0), &bound.arg(1))
    }

    #[inline(never)]
    fn method_argument_parser_convert_arg_line_to_args(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "ArgumentParser.convert_arg_line_to_args",
                params: &["self", "arg_line"],
                required: 2,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        Ok(list_value(vec![bound.arg(1)]))
    }

    #[inline(never)]
    fn method_argument_parser_match_argument(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "ArgumentParser._match_argument",
                params: &["self", "action", "arg_strings_pattern"],
                required: 3,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let pattern = self.ap_text(&bound.arg(2))?;
        let count = self.ap_match_argument_native(&bound.arg(0), &bound.arg(1), &pattern)?;
        Ok(Value::Number(count as i64))
    }

    #[inline(never)]
    fn method_argument_parser_match_arguments_partial(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "ArgumentParser._match_arguments_partial",
                params: &["self", "actions", "arg_strings_pattern"],
                required: 3,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let actions = self.ap_list_items(&bound.arg(1))?;
        let pattern = self.ap_text(&bound.arg(2))?;
        let counts = self.ap_match_arguments_partial_native(&bound.arg(0), &actions, &pattern)?;
        Ok(list_value(
            counts
                .into_iter()
                .map(|count| Value::Number(count as i64))
                .collect(),
        ))
    }

    #[inline(never)]
    fn method_argument_parser_parse_optional(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "ArgumentParser._parse_optional",
                params: &["self", "arg_string"],
                required: 2,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        self.ap_parse_optional(&bound.arg(0), &bound.arg(1))
    }

    #[inline(never)]
    fn method_argument_parser_get_option_tuples(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "ArgumentParser._get_option_tuples",
                params: &["self", "option_string"],
                required: 2,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let tuples = self.ap_get_option_tuples(&bound.arg(0), &bound.arg(1))?;
        Ok(list_value(tuples))
    }

    #[inline(never)]
    fn method_argument_parser_get_nargs_pattern(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let (_, action) =
            self.ap_bind_self_action("ArgumentParser._get_nargs_pattern", args, keywords)?;
        self.ap_get_nargs_pattern(&action)
    }

    #[inline(never)]
    fn method_argument_parser_get_values(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "ArgumentParser._get_values",
                params: &["self", "action", "arg_strings"],
                required: 3,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        self.ap_get_values(&bound.arg(0), &bound.arg(1), &bound.arg(2))
    }

    #[inline(never)]
    fn method_argument_parser_get_value(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "ArgumentParser._get_value",
                params: &["self", "action", "arg_string"],
                required: 3,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        self.ap_get_value(&bound.arg(0), &bound.arg(1), bound.arg(2))
    }

    #[inline(never)]
    fn method_argument_parser_check_value(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "ArgumentParser._check_value",
                params: &["self", "action", "value"],
                required: 3,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        self.ap_check_value(&bound.arg(0), &bound.arg(1), &bound.arg(2))?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_argument_parser_format_usage(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let receiver = self.ap_bind_self("ArgumentParser.format_usage", args, keywords)?;
        let formatter = self.ap_method(&receiver, "_get_formatter", Vec::new())?;
        let usage = self.ap_get(&receiver, "usage")?;
        let actions = self.ap_get(&receiver, "_actions")?;
        let groups = self.ap_get(&receiver, "_mutually_exclusive_groups")?;
        self.ap_method(&formatter, "add_usage", vec![usage, actions, groups])?;
        self.ap_method(&formatter, "format_help", Vec::new())
    }

    #[inline(never)]
    fn method_argument_parser_format_help(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let receiver = self.ap_bind_self("ArgumentParser.format_help", args, keywords)?;
        self.ap_parser_format_help(&receiver)
    }

    #[inline(never)]
    fn method_argument_parser_get_formatter(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let receiver = self.ap_bind_self("ArgumentParser._get_formatter", args, keywords)?;
        let formatter_class = self.ap_get(&receiver, "formatter_class")?;
        let prog = self.ap_get(&receiver, "prog")?;
        let formatter = self.call_value_with_keywords(
            formatter_class,
            Vec::new(),
            vec![("prog".to_string(), prog)],
        )?;
        let color = self.ap_get(&receiver, "color")?;
        self.ap_method(&formatter, "_set_color", vec![color])?;
        Ok(formatter)
    }

    #[inline(never)]
    fn method_argument_parser_get_validation_formatter(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let receiver =
            self.ap_bind_self("ArgumentParser._get_validation_formatter", args, keywords)?;
        let cached = self.ap_get(&receiver, "_cached_formatter")?;
        if !matches!(cached, Value::None) {
            return Ok(cached);
        }
        let formatter = self.ap_method(&receiver, "_get_formatter", Vec::new())?;
        self.ap_set(&receiver, "_cached_formatter", formatter.clone())?;
        Ok(formatter)
    }

    #[inline(never)]
    fn method_argument_parser_print_usage(
        &mut self,
        method: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: if method == "print_usage" {
                    "ArgumentParser.print_usage"
                } else {
                    "ArgumentParser.print_help"
                },
                params: &["self", "file"],
                required: 1,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let receiver = bound.arg(0);
        let file = match bound.arg(1) {
            Value::None => self.ap_sys_attr("stdout")?,
            file => file,
        };
        let format = if method == "print_usage" {
            "format_usage"
        } else {
            "format_help"
        };
        let text = self.ap_method(&receiver, format, Vec::new())?;
        self.ap_method(&receiver, "_print_message", vec![text, file])?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_argument_parser_print_message(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "ArgumentParser._print_message",
                params: &["self", "message", "file"],
                required: 2,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        self.ap_print_message(bound.arg(1), bound.arg(2))?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_argument_parser_exit(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "ArgumentParser.exit",
                params: &["self", "status", "message"],
                required: 1,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let receiver = bound.arg(0);
        let message = bound.arg(2);
        if self.ap_truth(&message)? {
            let stderr = self.ap_sys_attr("stderr")?;
            self.ap_method(&receiver, "_print_message", vec![message, stderr])?;
        }
        let exit = self.ap_sys_attr("exit")?;
        self.call_value(exit, vec![bound.or(1, Value::Number(0))])?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_argument_parser_error(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "ArgumentParser.error",
                params: &["self", "message"],
                required: 2,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let receiver = bound.arg(0);
        let stderr = self.ap_sys_attr("stderr")?;
        self.ap_method(&receiver, "print_usage", vec![stderr])?;
        let prog = self.ap_get(&receiver, "prog")?;
        let text = self.ap_percent(
            "%(prog)s: error: %(message)s\n",
            dict_value(vec![
                (str_value("prog"), prog),
                (str_value("message"), bound.arg(1)),
            ]),
        )?;
        self.ap_method(&receiver, "exit", vec![Value::Number(2), str_value(text)])?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_argument_parser_warning(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "ArgumentParser._warning",
                params: &["self", "message"],
                required: 2,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let receiver = bound.arg(0);
        let prog = self.ap_get(&receiver, "prog")?;
        let text = self.ap_percent(
            "%(prog)s: warning: %(message)s\n",
            dict_value(vec![
                (str_value("prog"), prog),
                (str_value("message"), bound.arg(1)),
            ]),
        )?;
        let stderr = self.ap_sys_attr("stderr")?;
        self.ap_method(&receiver, "_print_message", vec![str_value(text), stderr])?;
        Ok(Value::None)
    }

    fn ap_unpack_pair(&mut self, value: &Value) -> Result<(Value, Value), String> {
        let items = self.ap_list_items(value)?;
        match <[Value; 2]>::try_from(items) {
            Ok([first, second]) => Ok((first, second)),
            Err(items) => {
                if items.len() > 2 {
                    self.ap_error("ValueError", "too many values to unpack (expected 2)")
                } else {
                    self.ap_error(
                        "ValueError",
                        format!(
                            "not enough values to unpack (expected 2, got {})",
                            items.len()
                        ),
                    )
                }
            }
        }
    }

    /// `ArgumentParser.__init__`.
    fn ap_parser_init(&mut self, bound: Bound) -> Result<(), String> {
        let receiver = bound.arg(0);
        let prefix_chars = bound.or(7, str_value("-"));
        let conflict_handler = bound.or(10, str_value("error"));
        self.ap_container_init(
            &receiver,
            bound.arg(3),
            prefix_chars.clone(),
            bound.arg(9),
            conflict_handler,
        )?;
        let prog = self.ap_prog_name(bound.arg(1))?;
        self.ap_set(&receiver, "prog", prog)?;
        self.ap_set(&receiver, "usage", bound.arg(2))?;
        self.ap_set(&receiver, "epilog", bound.arg(4))?;
        let formatter_class = match bound.get(6) {
            Some(class) => class,
            None => self.ap_class("HelpFormatter")?,
        };
        self.ap_set(&receiver, "formatter_class", formatter_class)?;
        self.ap_set(&receiver, "fromfile_prefix_chars", bound.arg(8))?;
        self.ap_set(&receiver, "add_help", bound.or(11, Value::Bool(true)))?;
        self.ap_set(&receiver, "allow_abbrev", bound.or(12, Value::Bool(true)))?;
        self.ap_set(&receiver, "exit_on_error", bound.or(13, Value::Bool(true)))?;
        self.ap_set(
            &receiver,
            "suggest_on_error",
            bound.or(14, Value::Bool(false)),
        )?;
        self.ap_set(&receiver, "color", bound.or(15, Value::Bool(true)))?;
        self.ap_set(&receiver, "_cached_formatter", Value::None)?;

        let add_group = self.ap_get(&receiver, "add_argument_group")?;
        let positionals =
            self.call_value(add_group.clone(), vec![str_value("positional arguments")])?;
        self.ap_set(&receiver, "_positionals", positionals)?;
        let optionals = self.call_value(add_group, vec![str_value("options")])?;
        self.ap_set(&receiver, "_optionals", optionals)?;
        self.ap_set(&receiver, "_subparsers", Value::None)?;

        let identity = Value::Builtin(format!("{ARGPARSE_PREFIX}_identity"));
        let identity = match self.ap_module() {
            Ok(module) => self.ap_get_opt(&module, "_identity")?.unwrap_or(identity),
            Err(_) => identity,
        };
        self.ap_method(
            &receiver,
            "register",
            vec![str_value("type"), Value::None, identity],
        )?;

        let default_prefix = if self.ap_contains(&prefix_chars, &str_value("-"))? {
            str_value("-")
        } else {
            self.load_subscript_value(prefix_chars, Value::Number(0))?
        };
        let add_help = self.ap_get(&receiver, "add_help")?;
        if self.ap_truth(&add_help)? {
            let prefix = self.ap_text(&default_prefix)?;
            self.ap_method_kw(
                &receiver,
                "add_argument",
                vec![
                    str_value(format!("{prefix}h")),
                    str_value(format!("{prefix}{prefix}help")),
                ],
                vec![
                    ("action".to_string(), str_value("help")),
                    ("default".to_string(), str_value(SUPPRESS)),
                    (
                        "help".to_string(),
                        str_value("show this help message and exit"),
                    ),
                ],
            )?;
        }

        let parents = bound.get(5).unwrap_or_else(|| list_value(Vec::new()));
        let parser_class = self.ap_class("ArgumentParser")?;
        for parent in self.ap_iter(&parents)? {
            if !self.ap_isinstance(&parent, &parser_class)? {
                return self.ap_error("TypeError", "parents must be a list of ArgumentParser");
            }
            self.ap_method(&receiver, "_add_container_actions", vec![parent.clone()])?;
            let defaults = self.ap_get(&parent, "_defaults")?;
            let own_defaults = self.ap_get(&receiver, "_defaults")?;
            self.ap_method(&own_defaults, "update", vec![defaults])?;
        }
        Ok(())
    }

    /// `ArgumentParser.add_subparsers(**kwargs)`.
    fn ap_add_subparsers(
        &mut self,
        receiver: &Value,
        mut kwargs: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let existing = self.ap_get(receiver, "_subparsers")?;
        if !matches!(existing, Value::None) {
            return self.ap_error("ValueError", "cannot have multiple subparser arguments");
        }
        if !kwargs.iter().any(|(key, _)| key == "parser_class") {
            let class = self.ap_get(receiver, "__class__")?;
            kwargs.push(("parser_class".to_string(), class));
        }
        let subparsers_group = if kwargs
            .iter()
            .any(|(key, _)| key == "title" || key == "description")
        {
            let title =
                take_keyword(&mut kwargs, "title").unwrap_or_else(|| str_value("subcommands"));
            let description = take_keyword(&mut kwargs, "description").unwrap_or(Value::None);
            self.ap_method(receiver, "add_argument_group", vec![title, description])?
        } else {
            self.ap_get(receiver, "_positionals")?
        };
        self.ap_set(receiver, "_subparsers", subparsers_group.clone())?;

        let prog_missing = matches!(
            kwargs
                .iter()
                .find(|(key, _)| key == "prog")
                .map(|(_, value)| value),
            None | Some(Value::None)
        );
        if prog_missing {
            let formatter_class = self.ap_get(receiver, "formatter_class")?;
            let prog = self.ap_get(receiver, "prog")?;
            let formatter = self.call_value_with_keywords(
                formatter_class,
                Vec::new(),
                vec![("prog".to_string(), prog)],
            )?;
            self.ap_method(&formatter, "_set_color", vec![Value::Bool(false)])?;
            let positionals = self.ap_method(receiver, "_get_positional_actions", Vec::new())?;
            let groups = self.ap_get(receiver, "_mutually_exclusive_groups")?;
            self.ap_method(
                &formatter,
                "add_usage",
                vec![Value::None, positionals, groups, str_value("")],
            )?;
            let help = self.ap_method(&formatter, "format_help", Vec::new())?;
            let prog = self.ap_method(&help, "strip", Vec::new())?;
            set_keyword(&mut kwargs, "prog", prog);
        }

        let kwargs_dict = keyword_dict(kwargs);
        let parsers_class = self.ap_method(
            receiver,
            "_pop_action_class",
            vec![kwargs_dict.clone(), str_value("parsers")],
        )?;
        let mut keywords = vec![("option_strings".to_string(), list_value(Vec::new()))];
        keywords.extend(self.ap_dict_keywords(&kwargs_dict)?);
        let action = self.call_value_with_keywords(parsers_class, Vec::new(), keywords)?;
        let color = self.ap_get(receiver, "color")?;
        self.ap_set(&action, "_color", color)?;
        self.ap_method(receiver, "_check_help", vec![action.clone()])?;
        self.ap_method(&subparsers_group, "_add_action", vec![action.clone()])?;
        Ok(action)
    }

    fn ap_filter_actions(
        &mut self,
        receiver: &Value,
        want_optional: bool,
    ) -> Result<Vec<Value>, String> {
        let actions = self.ap_get(receiver, "_actions")?;
        let mut selected = Vec::new();
        for action in self.ap_list_items(&actions)? {
            let option_strings = self.ap_get(&action, "option_strings")?;
            if self.ap_truth(&option_strings)? == want_optional {
                selected.push(action);
            }
        }
        Ok(selected)
    }

    /// `ArgumentParser._parse_known_args2`.
    fn ap_parse_known_args2(
        &mut self,
        receiver: &Value,
        args: Value,
        namespace: Value,
        intermixed: Value,
    ) -> Result<Value, String> {
        let args = if matches!(args, Value::None) {
            let argv = self.ap_sys_attr("argv")?;
            self.load_subscript_value(
                argv,
                Value::Slice {
                    start: Some(Box::new(Value::Number(1))),
                    stop: None,
                    step: None,
                    identity: Rc::new(()),
                },
            )?
        } else {
            self.call_value(Value::Builtin("list".to_string()), vec![args])?
        };
        let namespace = if matches!(namespace, Value::None) {
            let class = self.ap_class("Namespace")?;
            self.call_value(class, Vec::new())?
        } else {
            namespace
        };

        let actions = self.ap_get(receiver, "_actions")?;
        for action in self.ap_list_items(&actions)? {
            let dest = self.ap_get(&action, "dest")?;
            if is_suppress(&dest) {
                continue;
            }
            let dest = self.ap_attr_name(&dest)?;
            if !self.ap_hasattr(&namespace, &dest)? {
                let default = self.ap_get(&action, "default")?;
                if !is_suppress(&default) {
                    self.ap_set(&namespace, &dest, default)?;
                }
            }
        }
        let defaults = self.ap_get(receiver, "_defaults")?;
        for dest in self.ap_iter(&defaults)? {
            let name = self.ap_attr_name(&dest)?;
            if !self.ap_hasattr(&namespace, &name)? {
                let value = self.load_subscript_value(defaults.clone(), dest)?;
                self.ap_set(&namespace, &name, value)?;
            }
        }

        let exit_on_error = self.ap_get(receiver, "exit_on_error")?;
        let parse = self.ap_get(receiver, "_parse_known_args")?;
        let call_args = vec![args.clone(), namespace.clone(), intermixed];
        let (namespace, args) = if self.ap_truth(&exit_on_error)? {
            match self.ap_invoke_catching(parse, call_args)? {
                Ok(result) => self.ap_unpack_pair(&result)?,
                Err(exception) => {
                    if !self.ap_exception_matches_class(&exception, "ArgumentError")? {
                        return Err(self.remember_runtime_exception_error(exception));
                    }
                    let message = self.ap_str(&value_from_exception(&exception))?;
                    self.ap_method(receiver, "error", vec![str_value(message)])?;
                    (namespace, args)
                }
            }
        } else {
            let result = self.ap_invoke(parse, call_args, Vec::new())?;
            self.ap_unpack_pair(&result)?
        };

        if self.ap_hasattr(&namespace, UNRECOGNIZED_ARGS_ATTR)? {
            let extra = self.ap_get(&namespace, UNRECOGNIZED_ARGS_ATTR)?;
            self.ap_method(&args, "extend", vec![extra])?;
            self.ap_del(&namespace, UNRECOGNIZED_ARGS_ATTR)?;
        }
        Ok(tuple_value(vec![namespace, args]))
    }

    /// The command-line parsing algorithm (`ArgumentParser._parse_known_args`).
    fn ap_parse_known_args(
        &mut self,
        receiver: &Value,
        arg_strings: Value,
        namespace: Value,
        intermixed: bool,
    ) -> Result<(Value, Vec<Value>), String> {
        // Subparsers re-enter parsing once per nesting level; CPython's
        // argparse is Python code, so a deep chain ends in RecursionError.
        crate::stack::check_python_recursion()?;
        let fromfile = self.ap_get(receiver, "fromfile_prefix_chars")?;
        let arg_strings = if matches!(fromfile, Value::None) {
            arg_strings
        } else {
            self.ap_method(receiver, "_read_args_from_files", vec![arg_strings])?
        };
        let mut arg_strings = self.ap_list_items(&arg_strings)?;

        // Map every mutually exclusive action to the actions it conflicts with.
        let mut action_conflicts: Vec<(Value, Vec<Value>)> = Vec::new();
        let mutex_groups = self.ap_get(receiver, "_mutually_exclusive_groups")?;
        for group in self.ap_list_items(&mutex_groups)? {
            let group_actions = self.ap_get(&group, "_group_actions")?;
            let group_actions = self.ap_list_items(&group_actions)?;
            for (index, action) in group_actions.iter().enumerate() {
                let others = group_actions[..index]
                    .iter()
                    .chain(group_actions[index + 1..].iter())
                    .cloned();
                match action_conflicts
                    .iter_mut()
                    .find(|(key, _)| same_object(key, action))
                {
                    Some((_, conflicts)) => conflicts.extend(others),
                    None => action_conflicts.push((action.clone(), others.collect())),
                }
            }
        }

        // Classify every arg string: 'O' option, 'A' argument, '-' for '--'.
        let mut option_string_indices: BTreeMap<usize, Value> = BTreeMap::new();
        let mut pattern = String::new();
        let mut index = 0;
        while index < arg_strings.len() {
            let arg_string = arg_strings[index].clone();
            if matches!(string_of(&arg_string), Some("--")) {
                pattern.push('-');
                for _ in index + 1..arg_strings.len() {
                    pattern.push('A');
                }
                break;
            }
            let option_tuples = self.ap_method(receiver, "_parse_optional", vec![arg_string])?;
            if matches!(option_tuples, Value::None) {
                pattern.push('A');
            } else {
                option_string_indices.insert(index, option_tuples);
                pattern.push('O');
            }
            index += 1;
        }
        let mut arg_strings_pattern = pattern;

        let mut state = ParseState {
            namespace: namespace.clone(),
            seen_actions: Vec::new(),
            seen_non_default_actions: Vec::new(),
            warned: Vec::new(),
            action_conflicts,
            extras: Vec::new(),
            extras_pattern: String::new(),
        };

        let positionals = self.ap_method(receiver, "_get_positional_actions", Vec::new())?;
        let mut positionals_list = self.ap_list_items(&positionals)?;

        let max_option_string_index = option_string_indices
            .keys()
            .next_back()
            .map(|index| *index as i64)
            .unwrap_or(-1);
        let mut start_index: usize = 0;
        while (start_index as i64) <= max_option_string_index {
            let mut next_option_string_index = start_index;
            while (next_option_string_index as i64) <= max_option_string_index {
                if option_string_indices.contains_key(&next_option_string_index) {
                    break;
                }
                next_option_string_index += 1;
            }
            if !intermixed && start_index != next_option_string_index {
                let positionals_end_index = self.ap_consume_positionals(
                    receiver,
                    &mut state,
                    &mut positionals_list,
                    &arg_strings,
                    &arg_strings_pattern,
                    start_index,
                )?;
                if positionals_end_index > start_index {
                    start_index = positionals_end_index;
                    continue;
                }
                start_index = positionals_end_index;
            }
            if !option_string_indices.contains_key(&start_index) {
                let end = next_option_string_index.min(arg_strings.len());
                if start_index < end {
                    state
                        .extras
                        .extend(arg_strings[start_index..end].iter().cloned());
                    state
                        .extras_pattern
                        .push_str(&arg_strings_pattern[start_index..end]);
                }
                start_index = next_option_string_index;
            }
            start_index = self.ap_consume_optional(
                receiver,
                &mut state,
                &option_string_indices,
                &arg_strings,
                &arg_strings_pattern,
                start_index,
            )?;
        }

        if !intermixed {
            let stop_index = self.ap_consume_positionals(
                receiver,
                &mut state,
                &mut positionals_list,
                &arg_strings,
                &arg_strings_pattern,
                start_index,
            )?;
            if stop_index < arg_strings.len() {
                state
                    .extras
                    .extend(arg_strings[stop_index..].iter().cloned());
            }
        } else {
            if start_index < arg_strings.len() {
                state
                    .extras
                    .extend(arg_strings[start_index..].iter().cloned());
                state
                    .extras_pattern
                    .push_str(&arg_strings_pattern[start_index..]);
            }
            let extras_pattern = state.extras_pattern.clone();
            let mut extras: Vec<Option<Value>> = state.extras.drain(..).map(Some).collect();
            arg_strings = extras
                .iter()
                .zip(extras_pattern.chars())
                .filter(|(_, kind)| *kind != 'O')
                .filter_map(|(value, _)| value.clone())
                .collect();
            arg_strings_pattern = extras_pattern.replace('O', "");
            let mut stop_index = self.ap_consume_positionals(
                receiver,
                &mut state,
                &mut positionals_list,
                &arg_strings,
                &arg_strings_pattern,
                0,
            )?;
            for (index, kind) in extras_pattern.chars().enumerate() {
                if stop_index == 0 {
                    break;
                }
                if kind != 'O' {
                    stop_index -= 1;
                    extras[index] = None;
                }
            }
            state.extras = extras.into_iter().flatten().collect();
        }

        // Required actions, and string defaults converted after parsing.
        let mut required_actions = Vec::new();
        let actions = self.ap_get(receiver, "_actions")?;
        for action in self.ap_list_items(&actions)? {
            if state
                .seen_actions
                .iter()
                .any(|seen| same_object(seen, &action))
            {
                continue;
            }
            let required = self.ap_get(&action, "required")?;
            if self.ap_truth(&required)? {
                required_actions.push(self.ap_action_name(&action)?);
                continue;
            }
            let default = self.ap_get(&action, "default")?;
            if let Some(default_text) = string_of(&default) {
                let dest = self.ap_get(&action, "dest")?;
                let dest = self.ap_attr_name(&dest)?;
                if let Some(current) = self.ap_get_opt(&state.namespace, &dest)?
                    && string_of(&current) == Some(default_text)
                {
                    let converted = self.ap_method(
                        receiver,
                        "_get_value",
                        vec![action.clone(), default.clone()],
                    )?;
                    self.ap_set(&state.namespace, &dest, converted)?;
                }
            }
        }
        if !required_actions.is_empty() {
            let mut names = Vec::new();
            for (index, name) in required_actions.iter().enumerate() {
                match string_of(name) {
                    Some(name) => names.push(name.to_string()),
                    None => {
                        return self.ap_error(
                            "TypeError",
                            format!(
                                "sequence item {index}: expected str instance, {} found",
                                type_name(name)
                            ),
                        );
                    }
                }
            }
            return self.ap_raise_argument_error(
                Value::None,
                format!("the following arguments are required: {}", names.join(", ")),
            );
        }

        for group in self.ap_list_items(&mutex_groups)? {
            let required = self.ap_get(&group, "required")?;
            if !self.ap_truth(&required)? {
                continue;
            }
            let group_actions = self.ap_get(&group, "_group_actions")?;
            let group_actions = self.ap_list_items(&group_actions)?;
            let satisfied = group_actions.iter().any(|action| {
                state
                    .seen_non_default_actions
                    .iter()
                    .any(|seen| same_object(seen, action))
            });
            if satisfied {
                continue;
            }
            let mut names = Vec::new();
            for action in &group_actions {
                let help = self.ap_get(action, "help")?;
                if is_suppress(&help) {
                    continue;
                }
                let name = self.ap_action_name(action)?;
                names.push(match string_of(&name) {
                    Some(name) => name.to_string(),
                    None => self.ap_str(&name)?,
                });
            }
            return self.ap_raise_argument_error(
                Value::None,
                format!("one of the arguments {} is required", names.join(" ")),
            );
        }

        Ok((state.namespace, state.extras))
    }

    /// The `take_action` closure of `_parse_known_args`.
    fn ap_take_action(
        &mut self,
        receiver: &Value,
        state: &mut ParseState,
        action: &Value,
        argument_strings: Vec<Value>,
        option_string: Value,
    ) -> Result<(), String> {
        if !state
            .seen_actions
            .iter()
            .any(|seen| same_object(seen, action))
        {
            state.seen_actions.push(action.clone());
        }
        let has_arguments = !argument_strings.is_empty();
        let argument_values = self.ap_method(
            receiver,
            "_get_values",
            vec![action.clone(), list_value(argument_strings)],
        )?;
        let option_strings = self.ap_get(action, "option_strings")?;
        if self.ap_truth(&option_strings)? || has_arguments {
            if !state
                .seen_non_default_actions
                .iter()
                .any(|seen| same_object(seen, action))
            {
                state.seen_non_default_actions.push(action.clone());
            }
            let conflicts = state
                .action_conflicts
                .iter()
                .find(|(key, _)| same_object(key, action))
                .map(|(_, conflicts)| conflicts.clone())
                .unwrap_or_default();
            for conflict_action in conflicts {
                if state
                    .seen_non_default_actions
                    .iter()
                    .any(|seen| same_object(seen, &conflict_action))
                {
                    let name = self.ap_action_name(&conflict_action)?;
                    let name = self.ap_str(&name)?;
                    return self.ap_raise_argument_error(
                        action.clone(),
                        format!("not allowed with argument {name}"),
                    );
                }
            }
        }
        if !is_suppress(&argument_values) {
            self.ap_call_object(
                action.clone(),
                vec![
                    receiver.clone(),
                    state.namespace.clone(),
                    argument_values,
                    option_string,
                ],
            )?;
        }
        Ok(())
    }

    /// The `consume_optional` closure of `_parse_known_args`.
    fn ap_consume_optional(
        &mut self,
        receiver: &Value,
        state: &mut ParseState,
        option_string_indices: &BTreeMap<usize, Value>,
        arg_strings: &[Value],
        arg_strings_pattern: &str,
        start_index: usize,
    ) -> Result<usize, String> {
        let option_tuples = option_string_indices
            .get(&start_index)
            .cloned()
            .unwrap_or(Value::None);
        let option_tuples = self.ap_option_tuples(&option_tuples)?;
        if option_tuples.len() > 1 {
            let mut names = Vec::new();
            for tuple in &option_tuples {
                names.push(self.ap_text(&tuple.option_string)?);
            }
            let option = self.ap_str(&arg_strings[start_index])?;
            return self.ap_raise_argument_error(
                Value::None,
                format!(
                    "ambiguous option: {option} could match {}",
                    names.join(", ")
                ),
            );
        }
        let Some(first) = option_tuples.into_iter().next() else {
            return self.ap_error("IndexError", "list index out of range");
        };
        let OptionTuple {
            mut action,
            mut option_string,
            mut sep,
            mut explicit_arg,
        } = first;

        let chars = self.ap_get(receiver, "prefix_chars")?;
        let mut action_tuples: Vec<(Value, Vec<Value>, Value)> = Vec::new();
        let stop;
        loop {
            if matches!(action, Value::None) {
                state.extras.push(arg_strings[start_index].clone());
                state.extras_pattern.push('O');
                return Ok(start_index + 1);
            }
            if !matches!(explicit_arg, Value::None) {
                let arg_count = self.ap_match_argument(receiver, &action, "A")?;
                let option_text = self.ap_text(&option_string)?;
                let second_is_prefix = match option_text.chars().nth(1) {
                    Some(ch) => self.ap_contains(&chars, &str_value(ch.to_string()))?,
                    None => {
                        return self.ap_error("IndexError", "string index out of range");
                    }
                };
                let explicit_text = self.ap_text(&explicit_arg)?;
                if arg_count == 0 && !second_is_prefix && !explicit_text.is_empty() {
                    let first_char = explicit_text
                        .chars()
                        .next()
                        .expect("explicit arg is not empty");
                    let sep_truthy = self.ap_truth(&sep)?;
                    if sep_truthy || self.ap_contains(&chars, &str_value(first_char.to_string()))? {
                        let rendered = self.ap_repr(&explicit_arg)?;
                        return self.ap_raise_argument_error(
                            action,
                            format!("ignored explicit argument {rendered}"),
                        );
                    }
                    action_tuples.push((action.clone(), Vec::new(), option_string.clone()));
                    let prefix_char = option_text
                        .chars()
                        .next()
                        .expect("option string is not empty");
                    let new_option = format!("{prefix_char}{first_char}");
                    option_string = str_value(new_option.clone());
                    let optionals = self.ap_get(receiver, "_option_string_actions")?;
                    if self.ap_contains(&optionals, &option_string)? {
                        action = self.load_subscript_value(optionals, option_string.clone())?;
                        let rest: String = explicit_text.chars().skip(1).collect();
                        if rest.is_empty() {
                            sep = Value::None;
                            explicit_arg = Value::None;
                        } else if let Some(tail) = rest.strip_prefix('=') {
                            sep = str_value("=");
                            explicit_arg = str_value(tail);
                        } else {
                            sep = str_value("");
                            explicit_arg = str_value(rest);
                        }
                    } else {
                        state
                            .extras
                            .push(str_value(format!("{prefix_char}{explicit_text}")));
                        state.extras_pattern.push('O');
                        stop = start_index + 1;
                        break;
                    }
                } else if arg_count == 1 {
                    stop = start_index + 1;
                    action_tuples.push((
                        action.clone(),
                        vec![explicit_arg.clone()],
                        option_string.clone(),
                    ));
                    break;
                } else {
                    let rendered = self.ap_repr(&explicit_arg)?;
                    return self.ap_raise_argument_error(
                        action,
                        format!("ignored explicit argument {rendered}"),
                    );
                }
            } else {
                let start = start_index + 1;
                let selected_patterns = arg_strings_pattern.get(start..).unwrap_or("");
                let arg_count = self.ap_match_argument(receiver, &action, selected_patterns)?;
                stop = start + arg_count;
                let args = arg_strings
                    .get(start..stop.min(arg_strings.len()))
                    .unwrap_or(&[])
                    .to_vec();
                action_tuples.push((action.clone(), args, option_string.clone()));
                break;
            }
        }

        for (action, args, option_string) in action_tuples {
            let deprecated = self.ap_get(&action, "deprecated")?;
            if self.ap_truth(&deprecated)?
                && !state
                    .warned
                    .iter()
                    .any(|warned| self.ap_values_equal_text(warned, &option_string))
            {
                let option = self.ap_str(&option_string)?;
                self.ap_method(
                    receiver,
                    "_warning",
                    vec![str_value(format!("option '{option}' is deprecated"))],
                )?;
                state.warned.push(option_string.clone());
            }
            self.ap_take_action(receiver, state, &action, args, option_string)?;
        }
        Ok(stop)
    }

    fn ap_values_equal_text(&self, left: &Value, right: &Value) -> bool {
        match (string_of(left), string_of(right)) {
            (Some(left), Some(right)) => left == right,
            _ => same_object(left, right),
        }
    }

    /// The `consume_positionals` closure of `_parse_known_args`.
    fn ap_consume_positionals(
        &mut self,
        receiver: &Value,
        state: &mut ParseState,
        positionals: &mut Vec<Value>,
        arg_strings: &[Value],
        arg_strings_pattern: &str,
        mut start_index: usize,
    ) -> Result<usize, String> {
        let selected_pattern = arg_strings_pattern.get(start_index..).unwrap_or("");
        let counts = self.ap_method(
            receiver,
            "_match_arguments_partial",
            vec![list_value(positionals.clone()), str_value(selected_pattern)],
        )?;
        let counts = self.ap_list_items(&counts)?;
        let mut arg_counts = Vec::new();
        for count in &counts {
            arg_counts.push(self.index_i64(count.clone(), "count")?.max(0) as usize);
        }
        let consumed = arg_counts.len().min(positionals.len());
        let current = positionals.clone();
        for (action, arg_count) in current.iter().zip(arg_counts.iter()) {
            let end = (start_index + arg_count).min(arg_strings.len());
            let mut args = arg_strings.get(start_index..end).unwrap_or(&[]).to_vec();
            let nargs = self.ap_get(action, "nargs")?;
            match nargs_kind(&nargs) {
                Nargs::Parser => {
                    if arg_strings_pattern.as_bytes().get(start_index) == Some(&b'-') {
                        if let Some(position) = args
                            .iter()
                            .position(|arg| matches!(string_of(arg), Some("--")))
                        {
                            args.remove(position);
                        }
                    }
                }
                Nargs::Remainder => {}
                _ => {
                    let window_end = (start_index + arg_count).min(arg_strings_pattern.len());
                    let window = arg_strings_pattern
                        .get(start_index..window_end)
                        .unwrap_or("");
                    if window.contains('-') {
                        match args
                            .iter()
                            .position(|arg| matches!(string_of(arg), Some("--")))
                        {
                            Some(position) => {
                                args.remove(position);
                            }
                            None => {
                                return self
                                    .ap_error("ValueError", "list.remove(x): x not in list");
                            }
                        }
                    }
                }
            }
            start_index += arg_count;
            if !args.is_empty() {
                let deprecated = self.ap_get(action, "deprecated")?;
                if self.ap_truth(&deprecated)? {
                    let dest = self.ap_get(action, "dest")?;
                    if !state
                        .warned
                        .iter()
                        .any(|warned| self.ap_values_equal_text(warned, &dest))
                    {
                        let name = self.ap_str(&dest)?;
                        self.ap_method(
                            receiver,
                            "_warning",
                            vec![str_value(format!("argument '{name}' is deprecated"))],
                        )?;
                        state.warned.push(dest);
                    }
                }
            }
            self.ap_take_action(receiver, state, action, args, Value::None)?;
        }
        positionals.drain(..consumed);
        Ok(start_index)
    }

    fn ap_option_tuples(&mut self, value: &Value) -> Result<Vec<OptionTuple>, String> {
        let mut tuples = Vec::new();
        for item in self.ap_list_items(value)? {
            let parts = self.ap_list_items(&item)?;
            let Ok([action, option_string, sep, explicit_arg]) = <[Value; 4]>::try_from(parts)
            else {
                return self.ap_error("ValueError", "too many values to unpack (expected 4)");
            };
            tuples.push(OptionTuple {
                action,
                option_string,
                sep,
                explicit_arg,
            });
        }
        Ok(tuples)
    }

    /// `self._match_argument(action, pattern)` through attribute lookup.
    fn ap_match_argument(
        &mut self,
        receiver: &Value,
        action: &Value,
        pattern: &str,
    ) -> Result<usize, String> {
        let count = self.ap_method(
            receiver,
            "_match_argument",
            vec![action.clone(), str_value(pattern)],
        )?;
        Ok(self.index_i64(count, "count")?.max(0) as usize)
    }

    /// `ArgumentParser._match_argument`.
    fn ap_match_argument_native(
        &mut self,
        receiver: &Value,
        action: &Value,
        arg_strings_pattern: &str,
    ) -> Result<usize, String> {
        let nargs_pattern = self.ap_method(receiver, "_get_nargs_pattern", vec![action.clone()])?;
        let nargs_pattern = self.ap_text(&nargs_pattern)?;
        let regex = match NargsRegex::parse(&nargs_pattern) {
            Ok(regex) => regex,
            Err(message) => return self.ap_error("re.PatternError", message),
        };
        if let Some(found) = regex.match_prefix(arg_strings_pattern) {
            return Ok(found
                .groups
                .first()
                .copied()
                .flatten()
                .map_or(0, |(start, end)| end - start));
        }
        let nargs = self.ap_get(action, "nargs")?;
        let message = match nargs_kind(&nargs) {
            Nargs::Single => "expected one argument".to_string(),
            Nargs::Optional => "expected at most one argument".to_string(),
            Nargs::OneOrMore => "expected at least one argument".to_string(),
            Nargs::Count(1) => "expected 1 argument".to_string(),
            _ => {
                let rendered = self.ap_str(&nargs)?;
                format!("expected {rendered} arguments")
            }
        };
        self.ap_raise_argument_error(action.clone(), message)
    }

    /// `ArgumentParser._match_arguments_partial`.
    fn ap_match_arguments_partial_native(
        &mut self,
        receiver: &Value,
        actions: &[Value],
        arg_strings_pattern: &str,
    ) -> Result<Vec<usize>, String> {
        let mut patterns = Vec::new();
        for action in actions {
            let pattern = self.ap_method(receiver, "_get_nargs_pattern", vec![action.clone()])?;
            patterns.push(self.ap_text(&pattern)?);
        }
        for count in (1..=actions.len()).rev() {
            let pattern = patterns[..count].concat();
            let regex = match NargsRegex::parse(&pattern) {
                Ok(regex) => regex,
                Err(message) => return self.ap_error("re.PatternError", message),
            };
            if let Some(found) = regex.match_prefix(arg_strings_pattern) {
                let mut result = Vec::new();
                for group in &found.groups {
                    match group {
                        Some((start, end)) => result.push(end - start),
                        None => {
                            return self
                                .ap_error("TypeError", "object of type 'NoneType' has no len()");
                        }
                    }
                }
                if found.end < arg_strings_pattern.len()
                    && arg_strings_pattern.as_bytes()[found.end] == b'O'
                {
                    while result.last() == Some(&0) {
                        result.pop();
                    }
                }
                return Ok(result);
            }
        }
        Ok(Vec::new())
    }

    /// `ArgumentParser._parse_optional`.
    fn ap_parse_optional(&mut self, receiver: &Value, arg_string: &Value) -> Result<Value, String> {
        if !self.ap_truth(arg_string)? {
            return Ok(Value::None);
        }
        let prefix_chars = self.ap_get(receiver, "prefix_chars")?;
        let first = self.load_subscript_value(arg_string.clone(), Value::Number(0))?;
        if !self.ap_contains(&prefix_chars, &first)? {
            return Ok(Value::None);
        }
        let optionals = self.ap_get(receiver, "_option_string_actions")?;
        if self.ap_contains(&optionals, arg_string)? {
            let action = self.load_subscript_value(optionals, arg_string.clone())?;
            return Ok(list_value(vec![tuple_value(vec![
                action,
                arg_string.clone(),
                Value::None,
                Value::None,
            ])]));
        }
        if self.ap_len(arg_string)? == 1 {
            return Ok(Value::None);
        }
        let text = self.ap_text(arg_string)?;
        if let Some((option_string, explicit_arg)) = text.split_once('=') {
            let option_value = str_value(option_string);
            if self.ap_contains(&optionals, &option_value)? {
                let action = self.load_subscript_value(optionals, option_value.clone())?;
                return Ok(list_value(vec![tuple_value(vec![
                    action,
                    option_value,
                    str_value("="),
                    str_value(explicit_arg),
                ])]));
            }
        }
        let option_tuples =
            self.ap_method(receiver, "_get_option_tuples", vec![arg_string.clone()])?;
        if self.ap_truth(&option_tuples)? {
            return Ok(option_tuples);
        }
        if super::container::looks_like_negative_number(&text) {
            let flags = self.ap_get(receiver, "_has_negative_number_optionals")?;
            if !self.ap_truth(&flags)? {
                return Ok(Value::None);
            }
        }
        if text.contains(' ') {
            return Ok(Value::None);
        }
        Ok(list_value(vec![tuple_value(vec![
            Value::None,
            arg_string.clone(),
            Value::None,
            Value::None,
        ])]))
    }

    /// `ArgumentParser._get_option_tuples`.
    fn ap_get_option_tuples(
        &mut self,
        receiver: &Value,
        option_string: &Value,
    ) -> Result<Vec<Value>, String> {
        let chars = self.ap_get(receiver, "prefix_chars")?;
        let chars_text = self.ap_text(&chars)?;
        let text = self.ap_text(option_string)?;
        let mut characters = text.chars();
        let first = characters.next();
        let second = characters.next();
        let in_chars = |ch: Option<char>| ch.is_some_and(|ch| chars_text.contains(ch));
        let allow_abbrev = self.ap_get(receiver, "allow_abbrev")?;
        let allow_abbrev = self.ap_truth(&allow_abbrev)?;
        let optionals = self.ap_get(receiver, "_option_string_actions")?;
        let keys = self.ap_iter(&optionals)?;
        let mut result = Vec::new();
        if in_chars(first) && in_chars(second) {
            if allow_abbrev {
                let (option_prefix, sep, explicit_arg) = match text.split_once('=') {
                    Some((prefix, arg)) => (prefix.to_string(), str_value("="), str_value(arg)),
                    None => (text.clone(), Value::None, Value::None),
                };
                for key in keys {
                    let key_text = self.ap_text(&key)?;
                    if key_text.starts_with(&option_prefix) {
                        let action = self.load_subscript_value(optionals.clone(), key.clone())?;
                        result.push(tuple_value(vec![
                            action,
                            key,
                            sep.clone(),
                            explicit_arg.clone(),
                        ]));
                    }
                }
            }
        } else if in_chars(first) && second.is_some() {
            let (option_prefix, sep, explicit_arg) = match text.split_once('=') {
                Some((prefix, arg)) => (prefix.to_string(), str_value("="), str_value(arg)),
                None => (text.clone(), Value::None, Value::None),
            };
            let short_option_prefix: String = text.chars().take(2).collect();
            let short_explicit_arg: String = text.chars().skip(2).collect();
            for key in keys {
                let key_text = self.ap_text(&key)?;
                if key_text == short_option_prefix {
                    let action = self.load_subscript_value(optionals.clone(), key.clone())?;
                    result.push(tuple_value(vec![
                        action,
                        key,
                        str_value(""),
                        str_value(short_explicit_arg.clone()),
                    ]));
                } else if allow_abbrev && key_text.starts_with(&option_prefix) {
                    let action = self.load_subscript_value(optionals.clone(), key.clone())?;
                    result.push(tuple_value(vec![
                        action,
                        key,
                        sep.clone(),
                        explicit_arg.clone(),
                    ]));
                }
            }
        } else if second.is_none() && in_chars(first) {
            return self.ap_error("IndexError", "string index out of range");
        } else {
            return self
                .ap_raise_argument_error(Value::None, format!("unexpected option string: {text}"));
        }
        Ok(result)
    }

    /// `ArgumentParser._get_nargs_pattern`.
    fn ap_get_nargs_pattern(&mut self, action: &Value) -> Result<Value, String> {
        let nargs = self.ap_get(action, "nargs")?;
        let option_strings = self.ap_get(action, "option_strings")?;
        let option = self.ap_truth(&option_strings)?;
        let pattern = match nargs_kind(&nargs) {
            Nargs::Single => if option { "([A])" } else { "(-*A-*)" }.to_string(),
            Nargs::Optional => if option { "(A?)" } else { "(-*A?-*)" }.to_string(),
            Nargs::ZeroOrMore => if option { "(A*)" } else { "(-*[A-]*)" }.to_string(),
            Nargs::OneOrMore => if option { "(A+)" } else { "(-*A[A-]*)" }.to_string(),
            Nargs::Remainder => if option { "([AO]*)" } else { "(.*)" }.to_string(),
            Nargs::Parser => if option { "(A[AO]*)" } else { "(-*A[-AO]*)" }.to_string(),
            Nargs::Suppress => if option { "()" } else { "(-*)" }.to_string(),
            _ => {
                if option {
                    self.ap_percent("([AO]{%d})", nargs)?
                } else {
                    self.ap_percent("((?:-*A){%d}-*)", nargs)?
                }
            }
        };
        Ok(str_value(pattern))
    }

    /// `ArgumentParser._get_values`.
    fn ap_get_values(
        &mut self,
        receiver: &Value,
        action: &Value,
        arg_strings: &Value,
    ) -> Result<Value, String> {
        let nargs = self.ap_get(action, "nargs")?;
        let kind = nargs_kind(&nargs);
        let strings = self.ap_list_items(arg_strings)?;
        let option_strings = self.ap_get(action, "option_strings")?;
        let has_options = self.ap_truth(&option_strings)?;
        if strings.is_empty() && kind == Nargs::Optional {
            let mut value = if has_options {
                self.ap_get(action, "const")?
            } else {
                self.ap_get(action, "default")?
            };
            if string_of(&value).is_some() && !is_suppress(&value) {
                value = self.ap_method(receiver, "_get_value", vec![action.clone(), value])?;
            }
            return Ok(value);
        }
        if strings.is_empty() && kind == Nargs::ZeroOrMore && !has_options {
            let default = self.ap_get(action, "default")?;
            return Ok(if matches!(default, Value::None) {
                list_value(Vec::new())
            } else {
                default
            });
        }
        if strings.len() == 1 && matches!(kind, Nargs::Single | Nargs::Optional) {
            let value = self.ap_method(
                receiver,
                "_get_value",
                vec![action.clone(), strings[0].clone()],
            )?;
            self.ap_method(
                receiver,
                "_check_value",
                vec![action.clone(), value.clone()],
            )?;
            return Ok(value);
        }
        if kind == Nargs::Suppress {
            return Ok(str_value(SUPPRESS));
        }
        let mut values = Vec::new();
        for string in strings {
            values.push(self.ap_method(receiver, "_get_value", vec![action.clone(), string])?);
        }
        match kind {
            Nargs::Remainder => {}
            Nargs::Parser => {
                let Some(first) = values.first().cloned() else {
                    return self.ap_error("IndexError", "list index out of range");
                };
                self.ap_method(receiver, "_check_value", vec![action.clone(), first])?;
            }
            _ => {
                for value in &values {
                    self.ap_method(
                        receiver,
                        "_check_value",
                        vec![action.clone(), value.clone()],
                    )?;
                }
            }
        }
        Ok(list_value(values))
    }

    /// `ArgumentParser._get_value`.
    fn ap_get_value(
        &mut self,
        receiver: &Value,
        action: &Value,
        arg_string: Value,
    ) -> Result<Value, String> {
        let type_value = self.ap_get(action, "type")?;
        let type_func = self.ap_method(
            receiver,
            "_registry_get",
            vec![str_value("type"), type_value.clone(), type_value.clone()],
        )?;
        if !self.ap_callable(&type_func)? {
            let rendered = self.ap_repr(&type_func)?;
            return self.ap_error("TypeError", format!("{rendered} is not callable"));
        }
        match self.call_value_catching(type_func, vec![arg_string.clone()])? {
            Ok(result) => Ok(result),
            Err(exception) => {
                if self.ap_exception_matches_class(&exception, "ArgumentTypeError")? {
                    let message = self.ap_str(&value_from_exception(&exception))?;
                    return self.ap_raise_argument_error(action.clone(), message);
                }
                if exception_is(&exception, "TypeError") || exception_is(&exception, "ValueError") {
                    let name = match self.ap_get_opt(&type_value, "__name__")? {
                        Some(name) => name,
                        None => str_value(self.ap_repr(&type_value)?),
                    };
                    let message = self.ap_percent(
                        "invalid %(type)s value: %(value)r",
                        dict_value(vec![
                            (str_value("type"), name),
                            (str_value("value"), arg_string),
                        ]),
                    )?;
                    return self.ap_raise_argument_error(action.clone(), message);
                }
                Err(self.remember_runtime_exception_error(exception))
            }
        }
    }

    /// `ArgumentParser._check_value`.
    fn ap_check_value(
        &mut self,
        receiver: &Value,
        action: &Value,
        value: &Value,
    ) -> Result<(), String> {
        let choices = self.ap_get(action, "choices")?;
        if matches!(choices, Value::None) {
            return Ok(());
        }
        let contained = if string_of(&choices).is_some() {
            let mut found = false;
            for item in self.ap_iter(&choices)? {
                if self.ap_eq(&item, value)? {
                    found = true;
                    break;
                }
            }
            found
        } else {
            self.ap_contains(&choices, value)?
        };
        if contained {
            return Ok(());
        }
        let value_text = self.ap_str(value)?;
        let mut rendered_choices = Vec::new();
        let choice_items = self.ap_iter(&choices)?;
        for choice in &choice_items {
            let text = self.ap_str(choice)?;
            rendered_choices.push(self.ap_repr(&str_value(text))?);
        }
        let value_repr = self.ap_repr(&str_value(value_text.clone()))?;
        let mut message = format!(
            "invalid choice: {value_repr} (choose from {})",
            rendered_choices.join(", ")
        );
        let suggest = self.ap_get(receiver, "suggest_on_error")?;
        if self.ap_truth(&suggest)?
            && let Some(word) = string_of(value)
        {
            let mut all_strings = Vec::new();
            for choice in &choice_items {
                match string_of(choice) {
                    Some(text) => all_strings.push(text.to_string()),
                    None => {
                        all_strings.clear();
                        break;
                    }
                }
            }
            if all_strings.len() == choice_items.len()
                && let Some(closest) = close_match(word, &all_strings)
            {
                let closest = self.ap_repr(&str_value(closest))?;
                message = format!(
                    "invalid choice: {value_repr}, maybe you meant {closest}? (choose from {})",
                    rendered_choices.join(", ")
                );
            }
        }
        self.ap_raise_argument_error(action.clone(), message)
    }

    /// `ArgumentParser._read_args_from_files`.
    fn ap_read_args_from_files(
        &mut self,
        receiver: &Value,
        arg_strings: &Value,
    ) -> Result<Value, String> {
        let prefix_chars = self.ap_get(receiver, "fromfile_prefix_chars")?;
        let mut new_arg_strings = Vec::new();
        for arg_string in self.ap_iter(arg_strings)? {
            let is_file = if self.ap_truth(&arg_string)? {
                let first = self.load_subscript_value(arg_string.clone(), Value::Number(0))?;
                self.ap_contains(&prefix_chars, &first)?
            } else {
                false
            };
            if !is_file {
                new_arg_strings.push(arg_string);
                continue;
            }
            let path = self.load_subscript_value(
                arg_string,
                Value::Slice {
                    start: Some(Box::new(Value::Number(1))),
                    stop: None,
                    step: None,
                    identity: Rc::new(()),
                },
            )?;
            // The sandbox's own `open` decides; host files are never read.
            let open = self.ap_builtin_open()?;
            let opened = self.call_value_with_keywords_catching(
                open,
                vec![path],
                vec![
                    ("encoding".to_string(), str_value("utf-8")),
                    ("errors".to_string(), str_value("surrogateescape")),
                ],
            )?;
            let file = match opened {
                Ok(file) => file,
                Err(exception) if exception_is(&exception, "OSError") => {
                    let message = self.ap_str(&value_from_exception(&exception))?;
                    return self.ap_raise_argument_error(Value::None, message);
                }
                Err(exception) => return Err(self.remember_runtime_exception_error(exception)),
            };
            let content = self.ap_method(&file, "read", Vec::new())?;
            let _ = self
                .ap_get_opt(&file, "close")?
                .map(|close| self.call_value(close, Vec::new()));
            let lines = self.ap_method(&content, "splitlines", Vec::new())?;
            let mut file_args = Vec::new();
            for line in self.ap_iter(&lines)? {
                let converted = self.ap_method(receiver, "convert_arg_line_to_args", vec![line])?;
                file_args.extend(self.ap_iter(&converted)?);
            }
            let expanded = self.ap_method(
                receiver,
                "_read_args_from_files",
                vec![list_value(file_args)],
            )?;
            new_arg_strings.extend(self.ap_iter(&expanded)?);
        }
        Ok(list_value(new_arg_strings))
    }

    /// `ArgumentParser.format_help`.
    fn ap_parser_format_help(&mut self, receiver: &Value) -> Result<Value, String> {
        let formatter = self.ap_method(receiver, "_get_formatter", Vec::new())?;
        let usage = self.ap_get(receiver, "usage")?;
        let actions = self.ap_get(receiver, "_actions")?;
        let groups = self.ap_get(receiver, "_mutually_exclusive_groups")?;
        self.ap_method(&formatter, "add_usage", vec![usage, actions, groups])?;
        let description = self.ap_get(receiver, "description")?;
        self.ap_method(&formatter, "add_text", vec![description])?;
        let action_groups = self.ap_get(receiver, "_action_groups")?;
        for group in self.ap_list_items(&action_groups)? {
            let title = self.ap_get(&group, "title")?;
            self.ap_method(&formatter, "start_section", vec![title])?;
            let description = self.ap_get(&group, "description")?;
            self.ap_method(&formatter, "add_text", vec![description])?;
            let group_actions = self.ap_get(&group, "_group_actions")?;
            self.ap_method(&formatter, "add_arguments", vec![group_actions])?;
            self.ap_method(&formatter, "end_section", Vec::new())?;
        }
        let epilog = self.ap_get(receiver, "epilog")?;
        self.ap_method(&formatter, "add_text", vec![epilog])?;
        self.ap_method(&formatter, "format_help", Vec::new())
    }

    /// `ArgumentParser._print_message`.
    fn ap_print_message(&mut self, message: Value, file: Value) -> Result<(), String> {
        if !self.ap_truth(&message)? {
            return Ok(());
        }
        let file = if self.ap_truth(&file)? {
            file
        } else {
            self.ap_sys_attr("stderr")?
        };
        let write = match self.load_attribute_catching(file, "write")? {
            Ok(write) => write,
            Err(exception)
                if exception_is(&exception, "AttributeError")
                    || exception_is(&exception, "OSError") =>
            {
                return Ok(());
            }
            Err(exception) => return Err(self.remember_runtime_exception_error(exception)),
        };
        match self.call_value_catching(write, vec![message])? {
            Ok(_) => Ok(()),
            Err(exception)
                if exception_is(&exception, "AttributeError")
                    || exception_is(&exception, "OSError") =>
            {
                Ok(())
            }
            Err(exception) => Err(self.remember_runtime_exception_error(exception)),
        }
    }
}

/// Mutable state shared by the `_parse_known_args` closures.
struct ParseState {
    namespace: Value,
    seen_actions: Vec<Value>,
    seen_non_default_actions: Vec<Value>,
    warned: Vec<Value>,
    action_conflicts: Vec<(Value, Vec<Value>)>,
    extras: Vec<Value>,
    extras_pattern: String,
}

/// The regular-expression subset argparse builds nargs patterns from:
/// literals, `.`, character classes, capturing and non-capturing groups,
/// and greedy `*`, `+`, `?`, `{n}` quantifiers, matched with backtracking
/// exactly like `re.match`.
pub(super) struct NargsRegex {
    items: Vec<RegexItem>,
    groups: usize,
}

#[derive(Debug, Clone)]
enum RegexAtom {
    Chars(Vec<char>),
    Any,
    Group {
        capture: Option<usize>,
        items: Vec<RegexItem>,
    },
}

#[derive(Debug, Clone)]
struct RegexItem {
    atom: RegexAtom,
    min: usize,
    max: Option<usize>,
}

pub(super) struct RegexMatch {
    pub groups: Vec<Option<(usize, usize)>>,
    pub end: usize,
}

impl NargsRegex {
    pub fn parse(pattern: &str) -> Result<Self, String> {
        let chars = pattern.chars().collect::<Vec<_>>();
        let mut position = 0;
        let mut groups = 0;
        let items = parse_regex_sequence(&chars, &mut position, &mut groups, 0)?;
        if position != chars.len() {
            return Err(format!("unbalanced parenthesis at position {position}"));
        }
        Ok(NargsRegex { items, groups })
    }

    pub fn match_prefix(&self, text: &str) -> Option<RegexMatch> {
        let text = text.chars().collect::<Vec<_>>();
        let mut captures = vec![None; self.groups];
        let mut end = 0;
        let matched =
            match_regex_items(&self.items, &text, 0, &mut captures, &mut |position, _| {
                end = position;
                true
            });
        matched.then_some(RegexMatch {
            groups: captures,
            end,
        })
    }
}

fn parse_regex_sequence(
    chars: &[char],
    position: &mut usize,
    groups: &mut usize,
    depth: usize,
) -> Result<Vec<RegexItem>, String> {
    let mut items = Vec::new();
    while *position < chars.len() {
        let ch = chars[*position];
        let atom = match ch {
            ')' => {
                if depth == 0 {
                    return Err(format!("unbalanced parenthesis at position {}", *position));
                }
                return Ok(items);
            }
            '(' => {
                *position += 1;
                let capture = if chars.get(*position) == Some(&'?')
                    && chars.get(*position + 1) == Some(&':')
                {
                    *position += 2;
                    None
                } else {
                    let index = *groups;
                    *groups += 1;
                    Some(index)
                };
                let inner = parse_regex_sequence(chars, position, groups, depth + 1)?;
                if chars.get(*position) != Some(&')') {
                    return Err(format!(
                        "missing ), unterminated subpattern at position {}",
                        *position
                    ));
                }
                *position += 1;
                RegexAtom::Group {
                    capture,
                    items: inner,
                }
            }
            '[' => {
                *position += 1;
                let mut set = Vec::new();
                while *position < chars.len() && chars[*position] != ']' {
                    set.push(chars[*position]);
                    *position += 1;
                }
                if *position >= chars.len() {
                    return Err("unterminated character set".to_string());
                }
                *position += 1;
                RegexAtom::Chars(set)
            }
            '.' => {
                *position += 1;
                RegexAtom::Any
            }
            '\\' => {
                let escaped = chars
                    .get(*position + 1)
                    .copied()
                    .ok_or("bad escape (end of pattern)")?;
                *position += 2;
                RegexAtom::Chars(vec![escaped])
            }
            '*' | '+' | '?' | '{' => {
                return Err(format!("nothing to repeat at position {}", *position));
            }
            other => {
                *position += 1;
                RegexAtom::Chars(vec![other])
            }
        };
        let (min, max) = match chars.get(*position) {
            Some('*') => {
                *position += 1;
                (0, None)
            }
            Some('+') => {
                *position += 1;
                (1, None)
            }
            Some('?') => {
                *position += 1;
                (0, Some(1))
            }
            Some('{') => {
                let start = *position + 1;
                let mut end = start;
                while end < chars.len() && chars[end].is_ascii_digit() {
                    end += 1;
                }
                if end > start && chars.get(end) == Some(&'}') {
                    let count = chars[start..end]
                        .iter()
                        .collect::<String>()
                        .parse::<usize>()
                        .unwrap_or(0);
                    *position = end + 1;
                    (count, Some(count))
                } else {
                    // Not a valid repeat: `{` is a literal in Python's re.
                    (1, Some(1))
                }
            }
            _ => (1, Some(1)),
        };
        items.push(RegexItem { atom, min, max });
    }
    Ok(items)
}

type Captures = Vec<Option<(usize, usize)>>;

fn match_regex_items(
    items: &[RegexItem],
    text: &[char],
    position: usize,
    captures: &mut Captures,
    next: &mut dyn FnMut(usize, &mut Captures) -> bool,
) -> bool {
    let Some((item, rest)) = items.split_first() else {
        return next(position, captures);
    };
    match_regex_repeat(item, 0, text, position, captures, &mut |after, captures| {
        match_regex_items(rest, text, after, captures, next)
    })
}

fn match_regex_repeat(
    item: &RegexItem,
    count: usize,
    text: &[char],
    position: usize,
    captures: &mut Captures,
    next: &mut dyn FnMut(usize, &mut Captures) -> bool,
) -> bool {
    if item.max.is_none_or(|max| count < max)
        && match_regex_atom(
            &item.atom,
            text,
            position,
            captures,
            &mut |after, captures| {
                // Guard unbounded repetition of empty matches.
                if after == position && count >= item.min {
                    return false;
                }
                match_regex_repeat(item, count + 1, text, after, captures, next)
            },
        )
    {
        return true;
    }
    count >= item.min && next(position, captures)
}

fn match_regex_atom(
    atom: &RegexAtom,
    text: &[char],
    position: usize,
    captures: &mut Captures,
    next: &mut dyn FnMut(usize, &mut Captures) -> bool,
) -> bool {
    match atom {
        RegexAtom::Chars(set) => {
            text.get(position).is_some_and(|ch| set.contains(ch)) && next(position + 1, captures)
        }
        RegexAtom::Any => {
            text.get(position).is_some_and(|ch| *ch != '\n') && next(position + 1, captures)
        }
        RegexAtom::Group { capture, items } => match_regex_items(
            items,
            text,
            position,
            captures,
            &mut |after, captures| match capture {
                Some(index) => {
                    let previous = captures[*index];
                    captures[*index] = Some((position, after));
                    if next(after, captures) {
                        return true;
                    }
                    captures[*index] = previous;
                    false
                }
                None => next(after, captures),
            },
        ),
    }
}

/// `difflib.get_close_matches(word, possibilities, n=1, cutoff=0.6)[0]`.
fn close_match(word: &str, possibilities: &[String]) -> Option<String> {
    let word = word.chars().collect::<Vec<_>>();
    let mut best: Option<(f64, String)> = None;
    for possibility in possibilities {
        let candidate = possibility.chars().collect::<Vec<_>>();
        let ratio = sequence_ratio(&candidate, &word);
        if ratio < 0.6 {
            continue;
        }
        let better = match &best {
            None => true,
            Some((best_ratio, best_text)) => {
                ratio > *best_ratio || (ratio == *best_ratio && possibility > best_text)
            }
        };
        if better {
            best = Some((ratio, possibility.clone()));
        }
    }
    best.map(|(_, text)| text)
}

/// `difflib.SequenceMatcher(None, a, b).ratio()`.
fn sequence_ratio(a: &[char], b: &[char]) -> f64 {
    let total = a.len() + b.len();
    if total == 0 {
        return 1.0;
    }
    // b2j with autojunk: popular elements of long sequences are ignored.
    let mut b2j: HashMap<char, Vec<usize>> = HashMap::new();
    for (index, ch) in b.iter().enumerate() {
        b2j.entry(*ch).or_default().push(index);
    }
    let mut popular = HashSet::new();
    if b.len() >= 200 {
        let threshold = b.len() / 100 + 1;
        for (ch, indices) in &b2j {
            if indices.len() > threshold {
                popular.insert(*ch);
            }
        }
        for ch in &popular {
            b2j.remove(ch);
        }
    }
    let find_longest_match =
        |alo: usize, ahi: usize, blo: usize, bhi: usize| -> (usize, usize, usize) {
            let (mut besti, mut bestj, mut bestsize) = (alo, blo, 0usize);
            let mut j2len: HashMap<usize, usize> = HashMap::new();
            for (i, ch) in a.iter().enumerate().take(ahi).skip(alo) {
                let mut new_j2len = HashMap::new();
                if let Some(indices) = b2j.get(ch) {
                    for &j in indices {
                        if j < blo {
                            continue;
                        }
                        if j >= bhi {
                            break;
                        }
                        let k = j
                            .checked_sub(1)
                            .and_then(|prev| j2len.get(&prev))
                            .copied()
                            .unwrap_or(0)
                            + 1;
                        new_j2len.insert(j, k);
                        if k > bestsize {
                            besti = i + 1 - k;
                            bestj = j + 1 - k;
                            bestsize = k;
                        }
                    }
                }
                j2len = new_j2len;
            }
            while besti > alo
                && bestj > blo
                && a[besti - 1] == b[bestj - 1]
                && !popular.contains(&b[bestj - 1])
            {
                besti -= 1;
                bestj -= 1;
                bestsize += 1;
            }
            while besti + bestsize < ahi
                && bestj + bestsize < bhi
                && a[besti + bestsize] == b[bestj + bestsize]
                && !popular.contains(&b[bestj + bestsize])
            {
                bestsize += 1;
            }
            (besti, bestj, bestsize)
        };
    let mut matches = 0;
    let mut queue = vec![(0, a.len(), 0, b.len())];
    while let Some((alo, ahi, blo, bhi)) = queue.pop() {
        let (i, j, k) = find_longest_match(alo, ahi, blo, bhi);
        if k > 0 {
            matches += k;
            if alo < i && blo < j {
                queue.push((alo, i, blo, j));
            }
            if i + k < ahi && j + k < bhi {
                queue.push((i + k, ahi, j + k, bhi));
            }
        }
    }
    2.0 * matches as f64 / total as f64
}
