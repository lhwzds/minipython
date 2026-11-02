// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Lib/argparse.py (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! `_ActionsContainer`, `_ArgumentGroup`, and `_MutuallyExclusiveGroup`.

use super::super::*;
use super::actions::{set_keyword, take_keyword};
use super::*;

/// The registry entries `_ActionsContainer.__init__` registers, in order.
const ACTION_REGISTRY: &[(Option<&str>, &str)] = &[
    (None, "_StoreAction"),
    (Some("store"), "_StoreAction"),
    (Some("store_const"), "_StoreConstAction"),
    (Some("store_true"), "_StoreTrueAction"),
    (Some("store_false"), "_StoreFalseAction"),
    (Some("append"), "_AppendAction"),
    (Some("append_const"), "_AppendConstAction"),
    (Some("count"), "_CountAction"),
    (Some("help"), "_HelpAction"),
    (Some("version"), "_VersionAction"),
    (Some("parsers"), "_SubParsersAction"),
    (Some("extend"), "_ExtendAction"),
];

/// `re.match(r'-\.?\d', text)`.
pub(super) fn looks_like_negative_number(text: &str) -> bool {
    let mut chars = text.chars();
    if chars.next() != Some('-') {
        return false;
    }
    let mut next = chars.next();
    if next == Some('.') {
        next = chars.next();
    }
    next.is_some_and(is_decimal_char)
}

pub(super) fn keyword_dict(keywords: Vec<(String, Value)>) -> Value {
    dict_value(
        keywords
            .into_iter()
            .map(|(key, value)| (Value::String(key), value))
            .collect(),
    )
}

impl Vm {
    /// Converts a `**kwargs` dict back into keyword arguments.
    pub(super) fn ap_dict_keywords(
        &mut self,
        dict: &Value,
    ) -> Result<Vec<(String, Value)>, String> {
        let items = self.ap_method(dict, "items", Vec::new())?;
        let mut keywords = Vec::new();
        for item in self.ap_iter(&items)? {
            let pair = self.ap_list_items(&item)?;
            let [key, value] = pair.as_slice() else {
                continue;
            };
            match string_of(key) {
                Some(key) => keywords.push((key.to_string(), value.clone())),
                None => return self.ap_error("TypeError", "keywords must be strings"),
            }
        }
        Ok(keywords)
    }

    pub(super) fn ap_dict_get(&mut self, dict: &Value, key: Value) -> Result<Value, String> {
        self.ap_method(dict, "get", vec![key])
    }

    pub(super) fn argparse_container_method(
        &mut self,
        class: &str,
        method: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        match (class, method) {
            ("_ActionsContainer", "__init__") => self.method_actions_container_init(args, keywords),
            ("_ActionsContainer", "register") => {
                self.method_actions_container_register(args, keywords)
            }
            ("_ActionsContainer", "_registry_get") => {
                self.method_actions_container_registry_get(args, keywords)
            }
            ("_ActionsContainer", "set_defaults") => {
                self.method_actions_container_set_defaults(args, keywords)
            }
            ("_ActionsContainer", "get_default") => {
                self.method_actions_container_get_default(args, keywords)
            }
            ("_ActionsContainer", "add_argument") => {
                self.method_actions_container_add_argument(args, keywords)
            }
            ("_ActionsContainer", "add_argument_group") => {
                self.method_actions_container_add_argument_group(args, keywords)
            }
            ("_ActionsContainer", "add_mutually_exclusive_group") => {
                self.method_actions_container_add_mutually_exclusive_group(args, keywords)
            }
            ("_ActionsContainer", "_add_action") => {
                self.method_actions_container_add_action(args, keywords)
            }
            ("_ActionsContainer", "_remove_action") => {
                self.method_actions_container_remove_action(args, keywords)
            }
            ("_ActionsContainer", "_add_container_actions") => {
                self.method_actions_container_add_container_actions(args, keywords)
            }
            ("_ActionsContainer", "_get_positional_kwargs") => {
                self.method_actions_container_get_positional_kwargs(args, keywords)
            }
            ("_ActionsContainer", "_get_optional_kwargs") => {
                self.method_actions_container_get_optional_kwargs(args, keywords)
            }
            ("_ActionsContainer", "_pop_action_class") => {
                self.method_actions_container_pop_action_class(args, keywords)
            }
            ("_ActionsContainer", "_get_handler") => {
                self.method_actions_container_get_handler(args, keywords)
            }
            ("_ActionsContainer", "_check_conflict") => {
                self.method_actions_container_check_conflict(args, keywords)
            }
            ("_ActionsContainer", "_handle_conflict_error") => {
                self.method_actions_container_handle_conflict_error(args, keywords)
            }
            ("_ActionsContainer", "_handle_conflict_resolve") => {
                self.method_actions_container_handle_conflict_resolve(args, keywords)
            }
            ("_ActionsContainer", "_check_help") => {
                self.method_actions_container_check_help(args, keywords)
            }
            ("_ArgumentGroup", "__init__") => self.method_argument_group_init(args, keywords),
            ("_ArgumentGroup", "_add_action") => {
                self.method_argument_group_add_action(args, keywords)
            }
            ("_ArgumentGroup", "_remove_action") => {
                self.method_argument_group_remove_action(args, keywords)
            }
            ("_ArgumentGroup", "add_argument_group") => {
                self.method_argument_group_add_argument_group(args, keywords)
            }
            ("_MutuallyExclusiveGroup", "__init__") => {
                self.method_mutually_exclusive_group_init(args, keywords)
            }
            ("_MutuallyExclusiveGroup", "_add_action") => {
                self.method_mutually_exclusive_group_add_action(args, keywords)
            }
            ("_MutuallyExclusiveGroup", "_remove_action") => {
                self.method_mutually_exclusive_group_remove_action(args, keywords)
            }
            ("_MutuallyExclusiveGroup", "add_mutually_exclusive_group") => {
                self.method_mutually_exclusive_group_add_mutually_exclusive_group(args, keywords)
            }
            _ => Err(format!("unknown builtin: argparse.{class}.{method}")),
        }
    }

    #[inline(never)]
    fn method_actions_container_init(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "_ActionsContainer.__init__",
                params: &[
                    "self",
                    "description",
                    "prefix_chars",
                    "argument_default",
                    "conflict_handler",
                ],
                required: 5,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        self.ap_container_init(
            &bound.arg(0),
            bound.arg(1),
            bound.arg(2),
            bound.arg(3),
            bound.arg(4),
        )?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_actions_container_register(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "_ActionsContainer.register",
                params: &["self", "registry_name", "value", "object"],
                required: 4,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let registries = self.ap_get(&bound.arg(0), "_registries")?;
        let registry = self.ap_method(
            &registries,
            "setdefault",
            vec![bound.arg(1), dict_value(Vec::new())],
        )?;
        self.store_subscript_value(registry, bound.arg(2), bound.arg(3))?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_actions_container_registry_get(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "_ActionsContainer._registry_get",
                params: &["self", "registry_name", "value", "default"],
                required: 3,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let registries = self.ap_get(&bound.arg(0), "_registries")?;
        let registry = self.load_subscript_value(registries, bound.arg(1))?;
        self.ap_method(&registry, "get", vec![bound.arg(2), bound.arg(3)])
    }

    #[inline(never)]
    fn method_actions_container_set_defaults(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "_ActionsContainer.set_defaults",
                params: &["self"],
                required: 1,
                kwonly: &[],
                varargs: false,
                varkw: true,
            },
            args,
            keywords,
        )?;
        let receiver = bound.arg(0);
        let kwargs = keyword_dict(bound.extra_kwargs);
        let defaults = self.ap_get(&receiver, "_defaults")?;
        self.ap_method(&defaults, "update", vec![kwargs.clone()])?;
        let actions = self.ap_get(&receiver, "_actions")?;
        for action in self.ap_list_items(&actions)? {
            let dest = self.ap_get(&action, "dest")?;
            if self.ap_contains(&kwargs, &dest)? {
                let value = self.load_subscript_value(kwargs.clone(), dest)?;
                self.ap_set(&action, "default", value)?;
            }
        }
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_actions_container_get_default(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "_ActionsContainer.get_default",
                params: &["self", "dest"],
                required: 2,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let receiver = bound.arg(0);
        let dest = bound.arg(1);
        let actions = self.ap_get(&receiver, "_actions")?;
        for action in self.ap_list_items(&actions)? {
            let action_dest = self.ap_get(&action, "dest")?;
            if self.ap_eq(&action_dest, &dest)? {
                let default = self.ap_get(&action, "default")?;
                if !matches!(default, Value::None) {
                    return Ok(default);
                }
            }
        }
        let defaults = self.ap_get(&receiver, "_defaults")?;
        self.ap_method(&defaults, "get", vec![dest, Value::None])
    }

    #[inline(never)]
    fn method_actions_container_add_argument(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "_ActionsContainer.add_argument",
                params: &["self"],
                required: 1,
                kwonly: &[],
                varargs: true,
                varkw: true,
            },
            args,
            keywords,
        )?;
        self.ap_add_argument(&bound.arg(0), bound.extra_args, bound.extra_kwargs)
    }

    #[inline(never)]
    fn method_actions_container_add_argument_group(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "_ActionsContainer.add_argument_group",
                params: &["self"],
                required: 1,
                kwonly: &[],
                varargs: true,
                varkw: true,
            },
            args,
            keywords,
        )?;
        let receiver = bound.arg(0);
        let class = self.ap_class("_ArgumentGroup")?;
        let mut group_args = vec![receiver.clone()];
        group_args.extend(bound.extra_args);
        let group = self.call_value_with_keywords(class, group_args, bound.extra_kwargs)?;
        let groups = self.ap_get(&receiver, "_action_groups")?;
        self.ap_method(&groups, "append", vec![group.clone()])?;
        Ok(group)
    }

    #[inline(never)]
    fn method_actions_container_add_mutually_exclusive_group(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "_ActionsContainer.add_mutually_exclusive_group",
                params: &["self"],
                required: 1,
                kwonly: &[],
                varargs: false,
                varkw: true,
            },
            args,
            keywords,
        )?;
        let receiver = bound.arg(0);
        let class = self.ap_class("_MutuallyExclusiveGroup")?;
        let group =
            self.call_value_with_keywords(class, vec![receiver.clone()], bound.extra_kwargs)?;
        let groups = self.ap_get(&receiver, "_mutually_exclusive_groups")?;
        self.ap_method(&groups, "append", vec![group.clone()])?;
        Ok(group)
    }

    #[inline(never)]
    fn method_actions_container_add_action(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = self.ap_bind_self_action("_ActionsContainer._add_action", args, keywords)?;
        self.ap_container_add_action(&bound.0, &bound.1)
    }

    #[inline(never)]
    fn method_actions_container_remove_action(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let (receiver, action) =
            self.ap_bind_self_action("_ActionsContainer._remove_action", args, keywords)?;
        let actions = self.ap_get(&receiver, "_actions")?;
        self.ap_method(&actions, "remove", vec![action])?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_actions_container_add_container_actions(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "_ActionsContainer._add_container_actions",
                params: &["self", "container"],
                required: 2,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        self.ap_add_container_actions(&bound.arg(0), &bound.arg(1))?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_actions_container_get_positional_kwargs(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "_ActionsContainer._get_positional_kwargs",
                params: &["self", "dest"],
                required: 2,
                kwonly: &[],
                varargs: false,
                varkw: true,
            },
            args,
            keywords,
        )?;
        let dest = bound.arg(1);
        let mut kwargs = bound.extra_kwargs;
        if kwargs.iter().any(|(key, _)| key == "required") {
            return self.ap_error(
                "TypeError",
                "'required' is an invalid argument for positionals",
            );
        }
        let nargs = kwargs
            .iter()
            .find(|(key, _)| key == "nargs")
            .map(|(_, value)| value.clone())
            .unwrap_or(Value::None);
        if self.ap_eq(&nargs, &Value::Number(0))? {
            return self.ap_error("ValueError", "nargs for positionals must be != 0");
        }
        let mut optional_nargs = false;
        for kind in [OPTIONAL, ZERO_OR_MORE, REMAINDER, SUPPRESS] {
            if self.ap_eq(&str_value(kind), &nargs)? {
                optional_nargs = true;
                break;
            }
        }
        if !optional_nargs {
            set_keyword(&mut kwargs, "required", Value::Bool(true));
        }
        set_keyword(&mut kwargs, "dest", dest);
        set_keyword(&mut kwargs, "option_strings", list_value(Vec::new()));
        Ok(keyword_dict(kwargs))
    }

    #[inline(never)]
    fn method_actions_container_get_optional_kwargs(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "_ActionsContainer._get_optional_kwargs",
                params: &["self"],
                required: 1,
                kwonly: &[],
                varargs: true,
                varkw: true,
            },
            args,
            keywords,
        )?;
        self.ap_get_optional_kwargs(&bound.arg(0), bound.extra_args, bound.extra_kwargs)
    }

    #[inline(never)]
    fn method_actions_container_pop_action_class(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "_ActionsContainer._pop_action_class",
                params: &["self", "kwargs", "default"],
                required: 2,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let receiver = bound.arg(0);
        let action = self.ap_method(
            &bound.arg(1),
            "pop",
            vec![str_value("action"), bound.arg(2)],
        )?;
        self.ap_method(
            &receiver,
            "_registry_get",
            vec![str_value("action"), action.clone(), action],
        )
    }

    #[inline(never)]
    fn method_actions_container_get_handler(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let receiver = self.ap_bind_self("_ActionsContainer._get_handler", args, keywords)?;
        self.ap_get_handler(&receiver)
    }

    #[inline(never)]
    fn method_actions_container_check_conflict(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let (receiver, action) =
            self.ap_bind_self_action("_ActionsContainer._check_conflict", args, keywords)?;
        self.ap_check_conflict(&receiver, &action)?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_actions_container_handle_conflict_error(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "_ActionsContainer._handle_conflict_error",
                params: &["self", "action", "conflicting_actions"],
                required: 3,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let conflicts = self.ap_list_items(&bound.arg(2))?;
        let mut names = Vec::new();
        for conflict in &conflicts {
            let pair = self.ap_list_items(conflict)?;
            if let Some(option_string) = pair.first() {
                names.push(self.ap_text(option_string)?);
            }
        }
        let label = if conflicts.len() == 1 {
            "conflicting option string"
        } else {
            "conflicting option strings"
        };
        self.ap_raise_argument_error(bound.arg(1), format!("{label}: {}", names.join(", ")))
    }

    #[inline(never)]
    fn method_actions_container_handle_conflict_resolve(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "_ActionsContainer._handle_conflict_resolve",
                params: &["self", "action", "conflicting_actions"],
                required: 3,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let receiver = bound.arg(0);
        for conflict in self.ap_list_items(&bound.arg(2))? {
            let pair = self.ap_list_items(&conflict)?;
            let [option_string, action] = pair.as_slice() else {
                return self.ap_error("ValueError", "too many values to unpack (expected 2)");
            };
            let option_strings = self.ap_get(action, "option_strings")?;
            self.ap_method(&option_strings, "remove", vec![option_string.clone()])?;
            let optionals = self.ap_get(&receiver, "_option_string_actions")?;
            self.ap_method(&optionals, "pop", vec![option_string.clone(), Value::None])?;
            if !self.ap_truth(&option_strings)? {
                let container = self.ap_get(action, "container")?;
                self.ap_method(&container, "_remove_action", vec![action.clone()])?;
            }
        }
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_actions_container_check_help(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let (receiver, action) =
            self.ap_bind_self_action("_ActionsContainer._check_help", args, keywords)?;
        self.ap_check_help(&receiver, &action)?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_argument_group_init(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "_ArgumentGroup.__init__",
                params: &["self", "container", "title", "description"],
                required: 2,
                kwonly: &[],
                varargs: false,
                varkw: true,
            },
            args,
            keywords,
        )?;
        self.ap_argument_group_init(
            &bound.arg(0),
            &bound.arg(1),
            bound.arg(2),
            bound.arg(3),
            bound.extra_kwargs,
        )?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_argument_group_add_action(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let (receiver, action) =
            self.ap_bind_self_action("_ArgumentGroup._add_action", args, keywords)?;
        let action = self.ap_container_add_action(&receiver, &action)?;
        let group_actions = self.ap_get(&receiver, "_group_actions")?;
        self.ap_method(&group_actions, "append", vec![action.clone()])?;
        Ok(action)
    }

    #[inline(never)]
    fn method_argument_group_remove_action(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let (receiver, action) =
            self.ap_bind_self_action("_ArgumentGroup._remove_action", args, keywords)?;
        let actions = self.ap_get(&receiver, "_actions")?;
        self.ap_method(&actions, "remove", vec![action.clone()])?;
        let group_actions = self.ap_get(&receiver, "_group_actions")?;
        self.ap_method(&group_actions, "remove", vec![action])?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_argument_group_add_argument_group(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        bind_args(
            &Sig {
                qualname: "_ArgumentGroup.add_argument_group",
                params: &["self"],
                required: 1,
                kwonly: &[],
                varargs: true,
                varkw: true,
            },
            args,
            keywords,
        )?;
        self.ap_error("ValueError", "argument groups cannot be nested")
    }

    #[inline(never)]
    fn method_mutually_exclusive_group_init(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = bind_args(
            &Sig {
                qualname: "_MutuallyExclusiveGroup.__init__",
                params: &["self", "container", "required"],
                required: 2,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let receiver = bound.arg(0);
        let container = bound.arg(1);
        self.ap_argument_group_init(&receiver, &container, Value::None, Value::None, Vec::new())?;
        self.ap_set(&receiver, "required", bound.or(2, Value::Bool(false)))?;
        self.ap_set(&receiver, "_container", container)?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_mutually_exclusive_group_add_action(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let (receiver, action) =
            self.ap_bind_self_action("_MutuallyExclusiveGroup._add_action", args, keywords)?;
        let required = self.ap_get(&action, "required")?;
        if self.ap_truth(&required)? {
            return self.ap_error(
                "ValueError",
                "mutually exclusive arguments must be optional",
            );
        }
        let container = self.ap_get(&receiver, "_container")?;
        let action = self.ap_method(&container, "_add_action", vec![action])?;
        let group_actions = self.ap_get(&receiver, "_group_actions")?;
        self.ap_method(&group_actions, "append", vec![action.clone()])?;
        Ok(action)
    }

    #[inline(never)]
    fn method_mutually_exclusive_group_remove_action(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let (receiver, action) =
            self.ap_bind_self_action("_MutuallyExclusiveGroup._remove_action", args, keywords)?;
        let container = self.ap_get(&receiver, "_container")?;
        self.ap_method(&container, "_remove_action", vec![action.clone()])?;
        let group_actions = self.ap_get(&receiver, "_group_actions")?;
        self.ap_method(&group_actions, "remove", vec![action])?;
        Ok(Value::None)
    }

    #[inline(never)]
    fn method_mutually_exclusive_group_add_mutually_exclusive_group(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        bind_args(
            &Sig {
                qualname: "_MutuallyExclusiveGroup.add_mutually_exclusive_group",
                params: &["self"],
                required: 1,
                kwonly: &[],
                varargs: false,
                varkw: true,
            },
            args,
            keywords,
        )?;
        self.ap_error("ValueError", "mutually exclusive groups cannot be nested")
    }

    pub(super) fn ap_bind_self_action(
        &mut self,
        qualname: &'static str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<(Value, Value), String> {
        let bound = bind_args(
            &Sig {
                qualname,
                params: &["self", "action"],
                required: 2,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        Ok((bound.arg(0), bound.arg(1)))
    }

    /// `_ActionsContainer.__init__`.
    pub(super) fn ap_container_init(
        &mut self,
        receiver: &Value,
        description: Value,
        prefix_chars: Value,
        argument_default: Value,
        conflict_handler: Value,
    ) -> Result<(), String> {
        self.ap_set(receiver, "description", description)?;
        self.ap_set(receiver, "argument_default", argument_default)?;
        self.ap_set(receiver, "prefix_chars", prefix_chars)?;
        self.ap_set(receiver, "conflict_handler", conflict_handler)?;
        self.ap_set(receiver, "_registries", dict_value(Vec::new()))?;
        for (name, class) in ACTION_REGISTRY {
            let key = name.map(str_value).unwrap_or(Value::None);
            let class = self.ap_class(class)?;
            self.ap_method(receiver, "register", vec![str_value("action"), key, class])?;
        }
        self.ap_method(receiver, "_get_handler", Vec::new())?;
        self.ap_set(receiver, "_actions", list_value(Vec::new()))?;
        self.ap_set(receiver, "_option_string_actions", dict_value(Vec::new()))?;
        self.ap_set(receiver, "_action_groups", list_value(Vec::new()))?;
        self.ap_set(
            receiver,
            "_mutually_exclusive_groups",
            list_value(Vec::new()),
        )?;
        self.ap_set(receiver, "_defaults", dict_value(Vec::new()))?;
        self.ap_set(
            receiver,
            "_has_negative_number_optionals",
            list_value(Vec::new()),
        )?;
        Ok(())
    }

    /// `_ArgumentGroup.__init__`.
    fn ap_argument_group_init(
        &mut self,
        receiver: &Value,
        container: &Value,
        title: Value,
        description: Value,
        mut kwargs: Vec<(String, Value)>,
    ) -> Result<(), String> {
        for name in ["conflict_handler", "prefix_chars", "argument_default"] {
            if !kwargs.iter().any(|(key, _)| key == name) {
                let value = self.ap_get(container, name)?;
                kwargs.push((name.to_string(), value));
            }
        }
        let mut init_keywords = vec![("description".to_string(), description)];
        init_keywords.extend(kwargs);
        let bound = bind_args(
            &Sig {
                qualname: "_ActionsContainer.__init__",
                params: &[
                    "self",
                    "description",
                    "prefix_chars",
                    "argument_default",
                    "conflict_handler",
                ],
                required: 5,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            vec![receiver.clone()],
            init_keywords,
        )?;
        self.ap_container_init(
            receiver,
            bound.arg(1),
            bound.arg(2),
            bound.arg(3),
            bound.arg(4),
        )?;
        self.ap_set(receiver, "title", title)?;
        self.ap_set(receiver, "_group_actions", list_value(Vec::new()))?;
        for name in [
            "_registries",
            "_actions",
            "_option_string_actions",
            "_defaults",
            "_has_negative_number_optionals",
            "_mutually_exclusive_groups",
        ] {
            let value = self.ap_get(container, name)?;
            self.ap_set(receiver, name, value)?;
        }
        Ok(())
    }

    /// `_ActionsContainer._add_action`.
    pub(super) fn ap_container_add_action(
        &mut self,
        receiver: &Value,
        action: &Value,
    ) -> Result<Value, String> {
        self.ap_method(receiver, "_check_conflict", vec![action.clone()])?;
        let actions = self.ap_get(receiver, "_actions")?;
        self.ap_method(&actions, "append", vec![action.clone()])?;
        self.ap_set(action, "container", receiver.clone())?;
        let option_strings = self.ap_get(action, "option_strings")?;
        let option_strings = self.ap_iter(&option_strings)?;
        let optionals = self.ap_get(receiver, "_option_string_actions")?;
        for option_string in &option_strings {
            self.store_subscript_value(optionals.clone(), option_string.clone(), action.clone())?;
        }
        for option_string in &option_strings {
            let text = self.ap_text(option_string)?;
            if looks_like_negative_number(&text) {
                let flags = self.ap_get(receiver, "_has_negative_number_optionals")?;
                if !self.ap_truth(&flags)? {
                    self.ap_method(&flags, "append", vec![Value::Bool(true)])?;
                }
            }
        }
        Ok(action.clone())
    }

    /// `_ActionsContainer.add_argument(*args, **kwargs)`.
    fn ap_add_argument(
        &mut self,
        receiver: &Value,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let chars = self.ap_get(receiver, "prefix_chars")?;
        let positional = if args.is_empty() {
            true
        } else if args.len() == 1 {
            let first = self.load_subscript_value(args[0].clone(), Value::Number(0))?;
            !self.ap_contains(&chars, &first)?
        } else {
            false
        };
        let kwargs = if positional {
            if !args.is_empty() && keywords.iter().any(|(key, _)| key == "dest") {
                return self.ap_error(
                    "TypeError",
                    "dest supplied twice for positional argument, did you mean metavar?",
                );
            }
            self.ap_method_kw(receiver, "_get_positional_kwargs", args, keywords)?
        } else {
            self.ap_method_kw(receiver, "_get_optional_kwargs", args, keywords)?
        };

        if !self.ap_contains(&kwargs, &str_value("default"))? {
            let dest = self.load_subscript_value(kwargs.clone(), str_value("dest"))?;
            let defaults = self.ap_get(receiver, "_defaults")?;
            if self.ap_contains(&defaults, &dest)? {
                let default = self.load_subscript_value(defaults, dest)?;
                self.store_subscript_value(kwargs.clone(), str_value("default"), default)?;
            } else {
                let argument_default = self.ap_get(receiver, "argument_default")?;
                if !matches!(argument_default, Value::None) {
                    self.store_subscript_value(
                        kwargs.clone(),
                        str_value("default"),
                        argument_default,
                    )?;
                }
            }
        }

        let action_name = self.ap_dict_get(&kwargs, str_value("action"))?;
        let action_class = self.ap_method(receiver, "_pop_action_class", vec![kwargs.clone()])?;
        if !self.ap_callable(&action_class)? {
            let rendered = self.ap_repr(&action_class)?;
            return self.ap_error("ValueError", format!("unknown action {rendered}"));
        }
        let action_keywords = self.ap_dict_keywords(&kwargs)?;
        let action = self.call_value_with_keywords(action_class, Vec::new(), action_keywords)?;

        let option_strings = self.ap_get(&action, "option_strings")?;
        if !self.ap_truth(&option_strings)? {
            let nargs = self.ap_get(&action, "nargs")?;
            if self.ap_eq(&nargs, &Value::Number(0))? {
                let rendered = self.ap_repr(&action_name)?;
                return self.ap_error(
                    "ValueError",
                    format!("action {rendered} is not valid for positional arguments"),
                );
            }
        }

        let type_value = self.ap_get(&action, "type")?;
        let type_func = self.ap_method(
            receiver,
            "_registry_get",
            vec![str_value("type"), type_value.clone(), type_value],
        )?;
        if !self.ap_callable(&type_func)? {
            let rendered = self.ap_repr(&type_func)?;
            return self.ap_error("TypeError", format!("{rendered} is not callable"));
        }
        let file_type = self.ap_class("FileType")?;
        if same_object(&type_func, &file_type) {
            let rendered = self.ap_repr(&type_func)?;
            return self.ap_error(
                "TypeError",
                format!("{rendered} is a FileType class object, instance of it must be passed"),
            );
        }

        if self.ap_hasattr(receiver, "_get_validation_formatter")? {
            let formatter = self.ap_method(receiver, "_get_validation_formatter", Vec::new())?;
            let format_args = self.ap_get(&formatter, "_format_args")?;
            match self.call_value_catching(format_args, vec![action.clone(), Value::None])? {
                Ok(_) => {}
                Err(exception) if exception_is(&exception, "TypeError") => {
                    return self
                        .ap_error("ValueError", "length of metavar tuple does not match nargs");
                }
                Err(exception) => return Err(self.remember_runtime_exception_error(exception)),
            }
        }
        self.ap_method(receiver, "_check_help", vec![action.clone()])?;
        self.ap_method(receiver, "_add_action", vec![action])
    }

    /// `_ActionsContainer._get_optional_kwargs(*args, **kwargs)`.
    fn ap_get_optional_kwargs(
        &mut self,
        receiver: &Value,
        args: Vec<Value>,
        mut kwargs: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let prefix_chars = self.ap_get(receiver, "prefix_chars")?;
        let mut option_strings = Vec::new();
        let mut long_option_strings = Vec::new();
        let mut last_option_string = Value::None;
        for option_string in args {
            let first = self.load_subscript_value(option_string.clone(), Value::Number(0))?;
            if !self.ap_contains(&prefix_chars, &first)? {
                let rendered = self.ap_repr(&option_string)?;
                let chars = self.ap_repr(&prefix_chars)?;
                return self.ap_error(
                    "ValueError",
                    format!(
                        "invalid option string {rendered}: must start with a character {chars}"
                    ),
                );
            }
            option_strings.push(option_string.clone());
            if self.ap_len(&option_string)? > 1 {
                let second = self.load_subscript_value(option_string.clone(), Value::Number(1))?;
                if self.ap_contains(&prefix_chars, &second)? {
                    long_option_strings.push(option_string.clone());
                }
            }
            last_option_string = option_string;
        }
        let dest = take_keyword(&mut kwargs, "dest").unwrap_or(Value::None);
        let dest = if matches!(dest, Value::None) {
            let dest_option_string = match long_option_strings.first() {
                Some(option) => option.clone(),
                None => match option_strings.first() {
                    Some(option) => option.clone(),
                    None => {
                        return self.ap_error("IndexError", "list index out of range");
                    }
                },
            };
            let stripped = self.ap_method(&dest_option_string, "lstrip", vec![prefix_chars])?;
            if !self.ap_truth(&stripped)? {
                let rendered = self.ap_repr(&last_option_string)?;
                return self.ap_error(
                    "TypeError",
                    format!("dest= is required for options like {rendered}"),
                );
            }
            self.ap_method(&stripped, "replace", vec![str_value("-"), str_value("_")])?
        } else {
            dest
        };
        set_keyword(&mut kwargs, "dest", dest);
        set_keyword(&mut kwargs, "option_strings", list_value(option_strings));
        Ok(keyword_dict(kwargs))
    }

    /// `_ActionsContainer._get_handler`.
    fn ap_get_handler(&mut self, receiver: &Value) -> Result<Value, String> {
        let handler = self.ap_get(receiver, "conflict_handler")?;
        let handler_text = self.ap_str(&handler)?;
        let name = format!("_handle_conflict_{handler_text}");
        match self.ap_get_opt(receiver, &name)? {
            Some(handler) => Ok(handler),
            None => {
                let rendered = self.ap_repr(&handler)?;
                self.ap_error(
                    "ValueError",
                    format!("invalid conflict_resolution value: {rendered}"),
                )
            }
        }
    }

    /// `_ActionsContainer._check_conflict`.
    fn ap_check_conflict(&mut self, receiver: &Value, action: &Value) -> Result<(), String> {
        let option_strings = self.ap_get(action, "option_strings")?;
        let optionals = self.ap_get(receiver, "_option_string_actions")?;
        let mut conflicts = Vec::new();
        for option_string in self.ap_iter(&option_strings)? {
            if self.ap_contains(&optionals, &option_string)? {
                let existing =
                    self.load_subscript_value(optionals.clone(), option_string.clone())?;
                conflicts.push(tuple_value(vec![option_string, existing]));
            }
        }
        if !conflicts.is_empty() {
            let handler = self.ap_method(receiver, "_get_handler", Vec::new())?;
            self.call_value(handler, vec![action.clone(), list_value(conflicts)])?;
        }
        Ok(())
    }

    /// `_ActionsContainer._check_help`.
    pub(super) fn ap_check_help(&mut self, receiver: &Value, action: &Value) -> Result<(), String> {
        let help = self.ap_get(action, "help")?;
        if !self.ap_truth(&help)? || !self.ap_hasattr(receiver, "_get_validation_formatter")? {
            return Ok(());
        }
        let formatter = self.ap_method(receiver, "_get_validation_formatter", Vec::new())?;
        let expand = self.ap_get(&formatter, "_expand_help")?;
        match self.call_value_catching(expand, vec![action.clone()])? {
            Ok(_) => Ok(()),
            Err(exception)
                if exception_is(&exception, "ValueError")
                    || exception_is(&exception, "TypeError")
                    || exception_is(&exception, "KeyError") =>
            {
                self.ap_error("ValueError", "badly formed help string")
            }
            Err(exception) => Err(self.remember_runtime_exception_error(exception)),
        }
    }

    /// `_ActionsContainer._add_container_actions`.
    fn ap_add_container_actions(
        &mut self,
        receiver: &Value,
        container: &Value,
    ) -> Result<(), String> {
        let mut title_group_map: Vec<(Value, Value)> = Vec::new();
        let groups = self.ap_get(receiver, "_action_groups")?;
        for group in self.ap_list_items(&groups)? {
            let title = self.ap_get(&group, "title")?;
            if self.ap_title_lookup(&title_group_map, &title)?.is_some() {
                let rendered = self.ap_repr(&title)?;
                return self.ap_error(
                    "ValueError",
                    format!("cannot merge actions - two groups are named {rendered}"),
                );
            }
            title_group_map.push((title, group));
        }

        let mut group_map: Vec<(Value, Value)> = Vec::new();
        let container_groups = self.ap_get(container, "_action_groups")?;
        for group in self.ap_list_items(&container_groups)? {
            let title = self.ap_get(&group, "title")?;
            if self.ap_title_lookup(&title_group_map, &title)?.is_none() {
                let description = self.ap_get(&group, "description")?;
                let conflict_handler = self.ap_get(&group, "conflict_handler")?;
                let new_group = self.ap_method_kw(
                    receiver,
                    "add_argument_group",
                    Vec::new(),
                    vec![
                        ("title".to_string(), title.clone()),
                        ("description".to_string(), description),
                        ("conflict_handler".to_string(), conflict_handler),
                    ],
                )?;
                title_group_map.push((title.clone(), new_group));
            }
            let target = self
                .ap_title_lookup(&title_group_map, &title)?
                .expect("group title was just mapped");
            let group_actions = self.ap_get(&group, "_group_actions")?;
            for action in self.ap_list_items(&group_actions)? {
                map_insert(&mut group_map, action, target.clone());
            }
        }

        let mutex_groups = self.ap_get(container, "_mutually_exclusive_groups")?;
        for group in self.ap_list_items(&mutex_groups)? {
            let group_container = self.ap_get(&group, "_container")?;
            let target_container = if same_object(&group_container, container) {
                receiver.clone()
            } else {
                let title = self.ap_get(&group_container, "title")?;
                match self.ap_title_lookup(&title_group_map, &title)? {
                    Some(target) => target,
                    None => {
                        let rendered = self.ap_repr(&title)?;
                        return self.ap_error("KeyError", rendered);
                    }
                }
            };
            let required = self.ap_get(&group, "required")?;
            let mutex_group = self.ap_method_kw(
                &target_container,
                "add_mutually_exclusive_group",
                Vec::new(),
                vec![("required".to_string(), required)],
            )?;
            let group_actions = self.ap_get(&group, "_group_actions")?;
            for action in self.ap_list_items(&group_actions)? {
                map_insert(&mut group_map, action, mutex_group.clone());
            }
        }

        let actions = self.ap_get(container, "_actions")?;
        for action in self.ap_list_items(&actions)? {
            let target = group_map
                .iter()
                .find(|(key, _)| same_object(key, &action))
                .map(|(_, group)| group.clone())
                .unwrap_or_else(|| receiver.clone());
            self.ap_method(&target, "_add_action", vec![action])?;
        }
        Ok(())
    }

    fn ap_title_lookup(
        &mut self,
        map: &[(Value, Value)],
        title: &Value,
    ) -> Result<Option<Value>, String> {
        for (key, group) in map {
            if same_object(key, title) || self.ap_eq(key, title)? {
                return Ok(Some(group.clone()));
            }
        }
        Ok(None)
    }
}

fn map_insert(map: &mut Vec<(Value, Value)>, key: Value, value: Value) {
    if let Some(entry) = map
        .iter_mut()
        .find(|(existing, _)| same_object(existing, &key))
    {
        entry.1 = value;
    } else {
        map.push((key, value));
    }
}
