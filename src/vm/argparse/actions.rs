// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Lib/argparse.py (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! Action classes: `Action`, the built-in store/append/count/help/version
//! actions, `BooleanOptionalAction`, and `_SubParsersAction`.

use super::super::*;
use super::*;

/// Keyword fields `Action.__init__` assigns, in CPython's order.
const ACTION_FIELDS: [&str; 11] = [
    "option_strings",
    "dest",
    "nargs",
    "const",
    "default",
    "type",
    "choices",
    "required",
    "help",
    "metavar",
    "deprecated",
];

/// Values for `Action.__init__`, defaulted like CPython.
struct ActionInit {
    option_strings: Value,
    dest: Value,
    nargs: Option<Value>,
    const_: Option<Value>,
    default: Option<Value>,
    type_: Option<Value>,
    choices: Option<Value>,
    required: Option<Value>,
    help: Option<Value>,
    metavar: Option<Value>,
    deprecated: Option<Value>,
}

impl Default for ActionInit {
    fn default() -> Self {
        ActionInit {
            option_strings: Value::None,
            dest: Value::None,
            nargs: None,
            const_: None,
            default: None,
            type_: None,
            choices: None,
            required: None,
            help: None,
            metavar: None,
            deprecated: None,
        }
    }
}

impl Vm {
    fn ap_action_init(&mut self, receiver: &Value, init: ActionInit) -> Result<(), String> {
        let values = [
            init.option_strings,
            init.dest,
            init.nargs.unwrap_or(Value::None),
            init.const_.unwrap_or(Value::None),
            init.default.unwrap_or(Value::None),
            init.type_.unwrap_or(Value::None),
            init.choices.unwrap_or(Value::None),
            init.required.unwrap_or(Value::Bool(false)),
            init.help.unwrap_or(Value::None),
            init.metavar.unwrap_or(Value::None),
            init.deprecated.unwrap_or(Value::Bool(false)),
        ];
        for (name, value) in ACTION_FIELDS.iter().zip(values) {
            self.ap_set(receiver, name, value)?;
        }
        Ok(())
    }

    pub(super) fn argparse_action_method(
        &mut self,
        class: &str,
        method: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        match (class, method) {
            ("Action", "__init__") => self.method_action_init(args, keywords),
            ("Action", "_get_kwargs") => self.method_action_get_kwargs(args, keywords),
            ("Action", "format_usage") => self.method_action_format_usage(args, keywords),
            ("Action", "__call__") => self.method_action_call(args, keywords),
            ("BooleanOptionalAction", "__init__") => {
                self.method_boolean_optional_action_init(args, keywords)
            }
            ("BooleanOptionalAction", "__call__") => {
                self.method_boolean_optional_action_call(args, keywords)
            }
            ("BooleanOptionalAction", "format_usage") => {
                self.method_boolean_optional_action_format_usage(args, keywords)
            }
            ("_StoreAction" | "_AppendAction", "__init__") => {
                self.method_store_action_init(class, args, keywords)
            }
            ("_StoreAction", "__call__") => self.method_store_action_call(args, keywords),
            ("_StoreConstAction", "__init__") => {
                self.method_store_const_action_init(args, keywords)
            }
            ("_StoreConstAction", "__call__") => {
                self.method_store_const_action_call(args, keywords)
            }
            ("_StoreTrueAction" | "_StoreFalseAction", "__init__") => {
                self.method_store_true_action_init(class, args, keywords)
            }
            ("_AppendAction", "__call__") | ("_ExtendAction", "__call__") => {
                self.method_append_action_call(class, args, keywords)
            }
            ("_AppendConstAction", "__init__") => {
                self.method_append_const_action_init(args, keywords)
            }
            ("_AppendConstAction", "__call__") => {
                self.method_append_const_action_call(args, keywords)
            }
            ("_CountAction", "__init__") => self.method_count_action_init(args, keywords),
            ("_CountAction", "__call__") => self.method_count_action_call(args, keywords),
            ("_HelpAction", "__init__") => self.method_help_action_init(args, keywords),
            ("_HelpAction", "__call__") => self.method_help_action_call(args, keywords),
            ("_VersionAction", "__init__") => self.method_version_action_init(args, keywords),
            ("_VersionAction", "__call__") => self.method_version_action_call(args, keywords),
            ("_SubParsersAction", "__init__") => {
                self.method_sub_parsers_action_init(args, keywords)
            }
            ("_SubParsersAction", "add_parser") => {
                self.method_sub_parsers_action_add_parser(args, keywords)
            }
            ("_SubParsersAction", "_get_subactions") => {
                self.method_sub_parsers_action_get_subactions(args, keywords)
            }
            ("_SubParsersAction", "__call__") => {
                self.method_sub_parsers_action_call(args, keywords)
            }
            ("_SubParsersAction._ChoicesPseudoAction", "__init__") => {
                self.method_sub_parsers_action_choices_pseudo_action_init(args, keywords)
            }
            _ => Err(format!("unknown builtin: argparse.{class}.{method}")),
        }
    }

    #[inline(never)]
    fn method_action_init(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "Action.__init__",
                params: &[
                    "self",
                    "option_strings",
                    "dest",
                    "nargs",
                    "const",
                    "default",
                    "type",
                    "choices",
                    "required",
                    "help",
                    "metavar",
                    "deprecated",
                ],
                required: 3,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let receiver = bound.arg(0);
        self.ap_action_init(
            &receiver,
            ActionInit {
                option_strings: bound.arg(1),
                dest: bound.arg(2),
                nargs: bound.get(3),
                const_: bound.get(4),
                default: bound.get(5),
                type_: bound.get(6),
                choices: bound.get(7),
                required: bound.get(8),
                help: bound.get(9),
                metavar: bound.get(10),
                deprecated: bound.get(11),
            },
        )?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_action_get_kwargs(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let receiver = self.ap_bind_self("Action._get_kwargs", args, keywords)?;
        let mut items = Vec::new();
        for name in ACTION_FIELDS {
            let value = self.ap_get(&receiver, name)?;
            items.push(tuple_value(vec![str_value(name), value]));
        }
        Ok(list_value(items))
    }

    #[inline(never)]
    fn method_action_format_usage(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let receiver = self.ap_bind_self("Action.format_usage", args, keywords)?;
        let option_strings = self.ap_get(&receiver, "option_strings")?;
        self.load_subscript_value(option_strings, Value::Number(0))
    }

    #[inline(never)]
    fn method_action_call(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        self.ap_bind_call("Action.__call__", args, keywords)?;
        self.ap_error("NotImplementedError", ".__call__() not defined")
    }

    #[inline(never)]
    fn method_boolean_optional_action_init(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "BooleanOptionalAction.__init__",
                params: &[
                    "self",
                    "option_strings",
                    "dest",
                    "default",
                    "required",
                    "help",
                    "deprecated",
                ],
                required: 3,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let receiver = bound.arg(0);
        let mut option_strings = Vec::new();
        for option_string in self.ap_iter(&bound.arg(1))? {
            option_strings.push(option_string.clone());
            let starts = self.ap_method(&option_string, "startswith", vec![str_value("--")])?;
            if self.ap_truth(&starts)? {
                let negated =
                    self.ap_method(&option_string, "startswith", vec![str_value("--no-")])?;
                if self.ap_truth(&negated)? {
                    let rendered = self.ap_repr(&option_string)?;
                    return self.ap_error(
                        "ValueError",
                        format!("invalid option name {rendered} for BooleanOptionalAction"),
                    );
                }
                let text = self.ap_text(&option_string)?;
                let tail: String = text.chars().skip(2).collect();
                option_strings.push(str_value(format!("--no-{tail}")));
            }
        }
        self.ap_action_init(
            &receiver,
            ActionInit {
                option_strings: list_value(option_strings),
                dest: bound.arg(2),
                nargs: Some(Value::Number(0)),
                default: bound.get(3),
                required: bound.get(4),
                help: bound.get(5),
                deprecated: bound.get(6),
                ..ActionInit::default()
            },
        )?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_boolean_optional_action_call(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = self.ap_bind_call("BooleanOptionalAction.__call__", args, keywords)?;
        let receiver = bound.arg(0);
        let option_string = bound.arg(4);
        let option_strings = self.ap_get(&receiver, "option_strings")?;
        if self.ap_contains(&option_strings, &option_string)? {
            let dest = self.ap_get(&receiver, "dest")?;
            let dest = self.ap_attr_name(&dest)?;
            let negated = self.ap_method(&option_string, "startswith", vec![str_value("--no-")])?;
            let value = !self.ap_truth(&negated)?;
            self.ap_set(&bound.arg(2), &dest, Value::Bool(value))?;
        }
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_boolean_optional_action_format_usage(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let receiver = self.ap_bind_self("BooleanOptionalAction.format_usage", args, keywords)?;
        let option_strings = self.ap_get(&receiver, "option_strings")?;
        let separator = str_value(" | ");
        self.ap_method(&separator, "join", vec![option_strings])
    }

    #[inline(never)]
    fn method_store_action_init(
        &mut self,
        class: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let qualname = if class == "_StoreAction" {
            "_StoreAction.__init__"
        } else {
            "_AppendAction.__init__"
        };
        let bound = bind_args(
            &Sig {
                qualname,
                params: &[
                    "self",
                    "option_strings",
                    "dest",
                    "nargs",
                    "const",
                    "default",
                    "type",
                    "choices",
                    "required",
                    "help",
                    "metavar",
                    "deprecated",
                ],
                required: 3,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let nargs = bound.arg(3);
        if self.ap_eq(&nargs, &Value::Number(0))? {
            let message = if class == "_StoreAction" {
                "nargs for store actions must be != 0; if you have nothing to store, actions such as store true or store const may be more appropriate"
            } else {
                "nargs for append actions must be != 0; if arg strings are not supplying the value to append, the append const action may be more appropriate"
            };
            return self.ap_error("ValueError", message);
        }
        if !matches!(bound.arg(4), Value::None) && !self.ap_eq(&nargs, &str_value(OPTIONAL))? {
            return self.ap_error("ValueError", "nargs must be '?' to supply const");
        }
        let receiver = bound.arg(0);
        self.ap_action_init(
            &receiver,
            ActionInit {
                option_strings: bound.arg(1),
                dest: bound.arg(2),
                nargs: bound.get(3),
                const_: bound.get(4),
                default: bound.get(5),
                type_: bound.get(6),
                choices: bound.get(7),
                required: bound.get(8),
                help: bound.get(9),
                metavar: bound.get(10),
                deprecated: bound.get(11),
            },
        )?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_store_action_call(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = self.ap_bind_call("_StoreAction.__call__", args, keywords)?;
        let dest = self.ap_get(&bound.arg(0), "dest")?;
        let dest = self.ap_attr_name(&dest)?;
        self.ap_set(&bound.arg(2), &dest, bound.arg(3))?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_store_const_action_init(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "_StoreConstAction.__init__",
                params: &[
                    "self",
                    "option_strings",
                    "dest",
                    "const",
                    "default",
                    "required",
                    "help",
                    "metavar",
                    "deprecated",
                ],
                required: 3,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        // CPython does not forward `metavar` here.
        self.ap_action_init(
            &bound.arg(0),
            ActionInit {
                option_strings: bound.arg(1),
                dest: bound.arg(2),
                nargs: Some(Value::Number(0)),
                const_: bound.get(3),
                default: bound.get(4),
                required: bound.get(5),
                help: bound.get(6),
                deprecated: bound.get(8),
                ..ActionInit::default()
            },
        )?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_store_const_action_call(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = self.ap_bind_call("_StoreConstAction.__call__", args, keywords)?;
        let receiver = bound.arg(0);
        let dest = self.ap_get(&receiver, "dest")?;
        let dest = self.ap_attr_name(&dest)?;
        let constant = self.ap_get(&receiver, "const")?;
        self.ap_set(&bound.arg(2), &dest, constant)?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_store_true_action_init(
        &mut self,
        class: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let store_true = class == "_StoreTrueAction";
        let bound = bind_args(
            &Sig {
                qualname: if store_true {
                    "_StoreTrueAction.__init__"
                } else {
                    "_StoreFalseAction.__init__"
                },
                params: &[
                    "self",
                    "option_strings",
                    "dest",
                    "default",
                    "required",
                    "help",
                    "deprecated",
                ],
                required: 3,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        self.ap_action_init(
            &bound.arg(0),
            ActionInit {
                option_strings: bound.arg(1),
                dest: bound.arg(2),
                nargs: Some(Value::Number(0)),
                const_: Some(Value::Bool(store_true)),
                default: Some(bound.or(3, Value::Bool(!store_true))),
                required: bound.get(4),
                help: bound.get(5),
                deprecated: bound.get(6),
                ..ActionInit::default()
            },
        )?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_append_action_call(
        &mut self,
        class: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let extend = class == "_ExtendAction";
        let bound = self.ap_bind_call(
            if extend {
                "_ExtendAction.__call__"
            } else {
                "_AppendAction.__call__"
            },
            args,
            keywords,
        )?;
        let receiver = bound.arg(0);
        let namespace = bound.arg(2);
        let dest = self.ap_get(&receiver, "dest")?;
        let dest = self.ap_attr_name(&dest)?;
        let items = self.ap_get_opt(&namespace, &dest)?.unwrap_or(Value::None);
        let items = self.ap_copy_items(&items)?;
        if extend {
            self.ap_method(&items, "extend", vec![bound.arg(3)])?;
        } else {
            self.ap_method(&items, "append", vec![bound.arg(3)])?;
        }
        self.ap_set(&namespace, &dest, items)?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_append_const_action_init(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "_AppendConstAction.__init__",
                params: &[
                    "self",
                    "option_strings",
                    "dest",
                    "const",
                    "default",
                    "required",
                    "help",
                    "metavar",
                    "deprecated",
                ],
                required: 3,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        self.ap_action_init(
            &bound.arg(0),
            ActionInit {
                option_strings: bound.arg(1),
                dest: bound.arg(2),
                nargs: Some(Value::Number(0)),
                const_: bound.get(3),
                default: bound.get(4),
                required: bound.get(5),
                help: bound.get(6),
                metavar: bound.get(7),
                deprecated: bound.get(8),
                ..ActionInit::default()
            },
        )?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_append_const_action_call(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = self.ap_bind_call("_AppendConstAction.__call__", args, keywords)?;
        let receiver = bound.arg(0);
        let namespace = bound.arg(2);
        let dest = self.ap_get(&receiver, "dest")?;
        let dest = self.ap_attr_name(&dest)?;
        let items = self.ap_get_opt(&namespace, &dest)?.unwrap_or(Value::None);
        let items = self.ap_copy_items(&items)?;
        let constant = self.ap_get(&receiver, "const")?;
        self.ap_method(&items, "append", vec![constant])?;
        self.ap_set(&namespace, &dest, items)?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_count_action_init(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "_CountAction.__init__",
                params: &[
                    "self",
                    "option_strings",
                    "dest",
                    "default",
                    "required",
                    "help",
                    "deprecated",
                ],
                required: 3,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        self.ap_action_init(
            &bound.arg(0),
            ActionInit {
                option_strings: bound.arg(1),
                dest: bound.arg(2),
                nargs: Some(Value::Number(0)),
                default: bound.get(3),
                required: bound.get(4),
                help: bound.get(5),
                deprecated: bound.get(6),
                ..ActionInit::default()
            },
        )?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_count_action_call(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = self.ap_bind_call("_CountAction.__call__", args, keywords)?;
        let receiver = bound.arg(0);
        let namespace = bound.arg(2);
        let dest = self.ap_get(&receiver, "dest")?;
        let dest = self.ap_attr_name(&dest)?;
        let count = self.ap_get_opt(&namespace, &dest)?.unwrap_or(Value::None);
        let count = if matches!(count, Value::None) {
            Value::Number(0)
        } else {
            count
        };
        let count = self.add_values(count, Value::Number(1))?;
        self.ap_set(&namespace, &dest, count)?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_help_action_init(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "_HelpAction.__init__",
                params: &[
                    "self",
                    "option_strings",
                    "dest",
                    "default",
                    "help",
                    "deprecated",
                ],
                required: 2,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        self.ap_action_init(
            &bound.arg(0),
            ActionInit {
                option_strings: bound.arg(1),
                dest: bound.or(2, str_value(SUPPRESS)),
                nargs: Some(Value::Number(0)),
                default: Some(bound.or(3, str_value(SUPPRESS))),
                help: bound.get(4),
                deprecated: bound.get(5),
                ..ActionInit::default()
            },
        )?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_help_action_call(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = self.ap_bind_call("_HelpAction.__call__", args, keywords)?;
        let parser = bound.arg(1);
        self.ap_method(&parser, "print_help", Vec::new())?;
        self.ap_method(&parser, "exit", Vec::new())?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_version_action_init(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "_VersionAction.__init__",
                params: &[
                    "self",
                    "option_strings",
                    "version",
                    "dest",
                    "default",
                    "help",
                    "deprecated",
                ],
                required: 2,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let receiver = bound.arg(0);
        let help = match bound.arg(5) {
            Value::None => str_value("show program's version number and exit"),
            help => help,
        };
        // CPython does not forward `deprecated` here.
        self.ap_action_init(
            &receiver,
            ActionInit {
                option_strings: bound.arg(1),
                dest: bound.or(3, str_value(SUPPRESS)),
                nargs: Some(Value::Number(0)),
                default: Some(bound.or(4, str_value(SUPPRESS))),
                help: Some(help),
                ..ActionInit::default()
            },
        )?;
        self.ap_set(&receiver, "version", bound.arg(2))?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_version_action_call(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = self.ap_bind_call("_VersionAction.__call__", args, keywords)?;
        let receiver = bound.arg(0);
        let parser = bound.arg(1);
        let mut version = self.ap_get(&receiver, "version")?;
        if matches!(version, Value::None) {
            version = self.ap_get(&parser, "version")?;
        }
        let formatter = self.ap_method(&parser, "_get_formatter", Vec::new())?;
        self.ap_method(&formatter, "add_text", vec![version])?;
        let text = self.ap_method(&formatter, "format_help", Vec::new())?;
        let stdout = self.ap_sys_attr("stdout")?;
        self.ap_method(&parser, "_print_message", vec![text, stdout])?;
        self.ap_method(&parser, "exit", Vec::new())?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_sub_parsers_action_init(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "_SubParsersAction.__init__",
                params: &[
                    "self",
                    "option_strings",
                    "prog",
                    "parser_class",
                    "dest",
                    "required",
                    "help",
                    "metavar",
                ],
                required: 4,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let receiver = bound.arg(0);
        let name_parser_map = dict_value(Vec::new());
        self.ap_set(&receiver, "_prog_prefix", bound.arg(2))?;
        self.ap_set(&receiver, "_parser_class", bound.arg(3))?;
        self.ap_set(&receiver, "_name_parser_map", name_parser_map.clone())?;
        self.ap_set(&receiver, "_choices_actions", list_value(Vec::new()))?;
        let deprecated = self.call_value(Value::Builtin("set".to_string()), Vec::new())?;
        self.ap_set(&receiver, "_deprecated", deprecated)?;
        self.ap_set(&receiver, "_color", Value::Bool(true))?;
        self.ap_action_init(
            &receiver,
            ActionInit {
                option_strings: bound.arg(1),
                dest: bound.or(4, str_value(SUPPRESS)),
                nargs: Some(str_value(PARSER)),
                choices: Some(name_parser_map),
                required: bound.get(5),
                help: bound.get(6),
                metavar: bound.get(7),
                ..ActionInit::default()
            },
        )?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_sub_parsers_action_add_parser(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "_SubParsersAction.add_parser",
                params: &["self", "name"],
                required: 2,
                kwonly: &["deprecated"],
                varargs: false,
                varkw: true,
            },
            args,
            keywords,
        )?;
        self.ap_add_parser(
            &bound.arg(0),
            bound.arg(1),
            bound.or(2, Value::Bool(false)),
            bound.extra_kwargs,
        )
    }

    #[inline(never)]
    fn method_sub_parsers_action_get_subactions(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let receiver = self.ap_bind_self("_SubParsersAction._get_subactions", args, keywords)?;
        self.ap_get(&receiver, "_choices_actions")
    }

    #[inline(never)]
    fn method_sub_parsers_action_call(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = self.ap_bind_call("_SubParsersAction.__call__", args, keywords)?;
        self.ap_subparsers_call(&bound.arg(0), &bound.arg(1), &bound.arg(2), bound.arg(3))?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_sub_parsers_action_choices_pseudo_action_init(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "_SubParsersAction._ChoicesPseudoAction.__init__",
                params: &["self", "name", "aliases", "help"],
                required: 4,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let name = bound.arg(1);
        let aliases = bound.arg(2);
        let mut metavar = name.clone();
        if self.ap_truth(&aliases)? {
            let separator = str_value(", ");
            let joined = self.ap_method(&separator, "join", vec![aliases])?;
            let joined = self.ap_text(&joined)?;
            let suffix = str_value(format!(" ({joined})"));
            metavar = self.add_values(metavar, suffix)?;
        }
        self.ap_action_init(
            &bound.arg(0),
            ActionInit {
                option_strings: list_value(Vec::new()),
                dest: name,
                help: bound.get(3),
                metavar: Some(metavar),
                ..ActionInit::default()
            },
        )?;
        Ok(Value::None)
    }

    /// Binds a method that takes only `self`.
    pub(super) fn ap_bind_self(
        &mut self,
        qualname: &'static str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname,
                params: &["self"],
                required: 1,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        Ok(bound.arg(0))
    }

    /// Binds `__call__(self, parser, namespace, values, option_string=None)`.
    fn ap_bind_call(
        &mut self,
        qualname: &'static str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Bound, String> {
        bind_args(
            &Sig {
                qualname,
                params: &["self", "parser", "namespace", "values", "option_string"],
                required: 4,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )
    }

    /// `_SubParsersAction.add_parser(name, *, deprecated=False, **kwargs)`.
    fn ap_add_parser(
        &mut self,
        receiver: &Value,
        name: Value,
        deprecated: Value,
        mut kwargs: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let get = |kwargs: &Vec<(String, Value)>, key: &str| {
            kwargs
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value.clone())
        };
        if matches!(get(&kwargs, "prog"), None | Some(Value::None)) {
            let prefix = self.ap_get(receiver, "_prog_prefix")?;
            let prefix = self.ap_str(&prefix)?;
            let name_text = self.ap_str(&name)?;
            set_keyword(
                &mut kwargs,
                "prog",
                str_value(format!("{prefix} {name_text}")),
            );
        }
        if matches!(get(&kwargs, "color"), None | Some(Value::None)) {
            let color = self.ap_get(receiver, "_color")?;
            set_keyword(&mut kwargs, "color", color);
        }
        let aliases = match take_keyword(&mut kwargs, "aliases") {
            Some(aliases) => aliases,
            None => tuple_value(Vec::new()),
        };
        let name_parser_map = self.ap_get(receiver, "_name_parser_map")?;
        if self.ap_contains(&name_parser_map, &name)? {
            let name_text = self.ap_str(&name)?;
            return self.ap_error("ValueError", format!("conflicting subparser: {name_text}"));
        }
        let alias_items = self.ap_iter(&aliases)?;
        for alias in &alias_items {
            if self.ap_contains(&name_parser_map, alias)? {
                let alias_text = self.ap_str(alias)?;
                return self.ap_error(
                    "ValueError",
                    format!("conflicting subparser alias: {alias_text}"),
                );
            }
        }
        let choice_action = match take_keyword(&mut kwargs, "help") {
            Some(help) => {
                let pseudo_class = self.ap_get(receiver, "_ChoicesPseudoAction")?;
                let action =
                    self.call_value(pseudo_class, vec![name.clone(), aliases.clone(), help])?;
                let choices_actions = self.ap_get(receiver, "_choices_actions")?;
                self.ap_method(&choices_actions, "append", vec![action.clone()])?;
                Some(action)
            }
            None => None,
        };
        let parser_class = self.ap_get(receiver, "_parser_class")?;
        let parser = self.call_value_with_keywords(parser_class, Vec::new(), kwargs)?;
        if let Some(action) = choice_action {
            self.ap_method(&parser, "_check_help", vec![action])?;
        }
        self.store_subscript_value(name_parser_map.clone(), name.clone(), parser.clone())?;
        for alias in &alias_items {
            self.store_subscript_value(name_parser_map.clone(), alias.clone(), parser.clone())?;
        }
        if self.ap_truth(&deprecated)? {
            let deprecated_names = self.ap_get(receiver, "_deprecated")?;
            self.ap_method(&deprecated_names, "add", vec![name])?;
            self.ap_method(&deprecated_names, "update", vec![aliases])?;
        }
        Ok(parser)
    }

    /// `_SubParsersAction.__call__`.
    fn ap_subparsers_call(
        &mut self,
        receiver: &Value,
        parser: &Value,
        namespace: &Value,
        values: Value,
    ) -> Result<(), String> {
        let parser_name = self.load_subscript_value(values.clone(), Value::Number(0))?;
        let arg_strings = self.load_subscript_value(
            values,
            Value::Slice {
                start: Some(Rc::new(Value::Number(1))),
                stop: None,
                step: None,
                identity: Rc::new(()),
            },
        )?;
        let dest = self.ap_get(receiver, "dest")?;
        if !is_suppress(&dest) {
            let dest = self.ap_attr_name(&dest)?;
            self.ap_set(namespace, &dest, parser_name.clone())?;
        }
        let name_parser_map = self.ap_get(receiver, "_name_parser_map")?;
        let subparser = match self
            .load_subscript_value_catching(name_parser_map.clone(), parser_name.clone())?
        {
            Ok(subparser) => subparser,
            Err(exception) if exception_is(&exception, "KeyError") => {
                let rendered = self.ap_repr(&parser_name)?;
                let mut names = Vec::new();
                for key in self.ap_iter(&name_parser_map)? {
                    names.push(self.ap_text(&key)?);
                }
                return self.ap_raise_argument_error(
                    receiver.clone(),
                    format!("unknown parser {rendered} (choices: {})", names.join(", ")),
                );
            }
            Err(exception) => return Err(self.propagate_captured_exception(exception)),
        };
        let deprecated_names = self.ap_get(receiver, "_deprecated")?;
        if self.ap_contains(&deprecated_names, &parser_name)? {
            let name = self.ap_str(&parser_name)?;
            self.ap_method(
                parser,
                "_warning",
                vec![str_value(format!("command '{name}' is deprecated"))],
            )?;
        }
        let result = self.ap_method(
            &subparser,
            "parse_known_args",
            vec![arg_strings, Value::None],
        )?;
        let pair = self.ap_list_items(&result)?;
        let [subnamespace, remaining] = pair.as_slice() else {
            return self.ap_error("ValueError", "too many values to unpack (expected 2)");
        };
        for (key, value) in self.ap_instance_dict_items(subnamespace)? {
            let key = self.ap_text(&key)?;
            self.ap_set(namespace, &key, value)?;
        }
        if self.ap_truth(remaining)? {
            if !self.ap_hasattr(namespace, UNRECOGNIZED_ARGS_ATTR)? {
                self.ap_set(namespace, UNRECOGNIZED_ARGS_ATTR, list_value(Vec::new()))?;
            }
            let unrecognized = self.ap_get(namespace, UNRECOGNIZED_ARGS_ATTR)?;
            self.ap_method(&unrecognized, "extend", vec![remaining.clone()])?;
        }
        Ok(())
    }
}

pub(super) fn set_keyword(keywords: &mut Vec<(String, Value)>, name: &str, value: Value) {
    if let Some(entry) = keywords.iter_mut().find(|(key, _)| key == name) {
        entry.1 = value;
    } else {
        keywords.push((name.to_string(), value));
    }
}

pub(super) fn take_keyword(keywords: &mut Vec<(String, Value)>, name: &str) -> Option<Value> {
    let index = keywords.iter().position(|(key, _)| key == name)?;
    Some(keywords.remove(index).1)
}
