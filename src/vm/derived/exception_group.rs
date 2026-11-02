// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Objects/exceptions.c (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! `ExceptionGroup` splitting (`split`, `subgroup`, `derive`) and the
//! `except*` matching and re-raise helpers.

use super::super::*;

/// `except*` additionally
/// rejects exception group classes.
fn check_except_star_matcher(matcher: &Value) -> Result<(), String> {
    check_except_matcher(matcher)?;
    let base_group = Value::Builtin("BaseExceptionGroup".to_string());
    let catches_group = match except_matcher_tuple_items(matcher) {
        Some(items) => items
            .iter()
            .any(|class| exception_class_is_subclass(class, &base_group)),
        None => exception_class_is_subclass(matcher, &base_group),
    };
    if catches_group {
        return Err(EXCEPT_STAR_GROUP_TYPE_ERROR.to_string());
    }
    Ok(())
}

/// How `BaseExceptionGroup.split`/`subgroup` and `except*` select leaves.
enum ExceptionGroupMatcher {
    /// A validated exception class or tuple of classes.
    Classes(Value),
    /// A callable called with each exception and group.
    Predicate(Value),
    /// Leaf exceptions with these identities (groups never match).
    LeafIdentities(Vec<Rc<()>>),
}

fn exception_group_matcher(value: Value) -> Result<ExceptionGroupMatcher, String> {
    if is_exception_class_value(&value) {
        return Ok(ExceptionGroupMatcher::Classes(value));
    }
    if let Value::Tuple(items) = &value {
        if items.iter().all(is_exception_class_value) {
            return Ok(ExceptionGroupMatcher::Classes(value));
        }
        return Err(EXCEPTION_GROUP_MATCHER_TYPE_ERROR.to_string());
    }
    if is_classinfo_type(&value) || !is_callable_value(&value) {
        return Err(EXCEPTION_GROUP_MATCHER_TYPE_ERROR.to_string());
    }
    Ok(ExceptionGroupMatcher::Predicate(value))
}

/// `BaseExceptionGroup(message, excs)` for the groups the runtime builds:
/// an `ExceptionGroup` when every exception is an `Exception`.
fn new_exception_group(message: Value, excs: Value) -> Result<MiniException, String> {
    let items = match &excs {
        Value::List(items) => items.borrow().clone(),
        Value::Tuple(items) => items.as_ref().clone(),
        _ => unreachable!("runtime exception groups wrap a list or tuple"),
    };
    let type_name = if exception_values_are_exceptions(&items)? {
        "ExceptionGroup"
    } else {
        "BaseExceptionGroup"
    };
    let exceptions = items
        .into_iter()
        .map(exception_from_value)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(MiniException {
        type_name: type_name.to_string(),
        type_hierarchy: builtin_exception_type_hierarchy(type_name),
        type_object: None,
        message: Some(str_value_checked(&message).unwrap_or_else(|_| message.to_string())),
        args: vec![message, excs],
        attrs: Vec::new(),
        cause: None,
        context: None,
        suppress_context: false,
        exceptions: Some(exceptions),
        identity: Rc::new(()),
    })
}

/// CPython `exceptiongroup_subset`: `orig.derive(excs)` with the traceback,
/// cause, context, and notes of `orig`, or `None` when `excs` is empty.
fn exception_group_subset(
    orig: &MiniException,
    excs: Vec<MiniException>,
) -> Result<Option<MiniException>, String> {
    if excs.is_empty() {
        return Ok(None);
    }
    let message = orig
        .args
        .first()
        .cloned()
        .unwrap_or_else(|| Value::String(orig.message.clone().unwrap_or_default()));
    let excs = list_value(excs.iter().map(value_from_exception).collect());
    let mut group = new_exception_group(message, excs)?;
    let traceback = exception_traceback_value(&orig.attrs);
    if !matches!(traceback, Value::None) {
        set_exception_traceback_attr(&mut group.attrs, traceback);
    }
    group.cause = orig.cause.clone();
    group.context = orig.context.clone();
    group.suppress_context = orig.suppress_context;
    if let Some(notes) = exception_notes_value(&orig.identity) {
        let notes = match &notes {
            Value::List(items) => list_value(items.borrow().clone()),
            Value::Tuple(items) => list_value(items.as_ref().clone()),
            _ => notes,
        };
        set_exception_notes_value(&group.identity, notes);
    }
    Ok(Some(group))
}

/// Wraps a naked exception that an `except*` clause matched in an
/// `ExceptionGroup` with an empty message.
fn wrap_naked_exception_for_except_star(
    exception: &MiniException,
) -> Result<MiniException, String> {
    let mut group = new_exception_group(
        Value::String(String::new()),
        tuple_value(vec![value_from_exception(exception)]),
    )?;
    set_exception_traceback_attr(&mut group.attrs, new_traceback_value());
    Ok(group)
}

fn same_optional_exception(
    left: &Option<Box<MiniException>>,
    right: &Option<Box<MiniException>>,
) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => Rc::ptr_eq(&left.identity, &right.identity),
        _ => false,
    }
}

/// CPython `is_same_exception_metadata`: whether an exception an `except*`
/// clause raised is (part of) the original group re-raised.
fn is_same_exception_metadata(exception: &MiniException, orig: &MiniException) -> bool {
    let same_traceback = match (
        exception_traceback_value(&exception.attrs),
        exception_traceback_value(&orig.attrs),
    ) {
        (Value::None, Value::None) => true,
        (Value::Traceback { identity: left }, Value::Traceback { identity: right }) => {
            Rc::ptr_eq(&left, &right)
        }
        _ => false,
    };
    // CPython compares the exceptions' C-level `notes` slots, which stay
    // NULL because `__notes__` lives in the instance dict; notes never make
    // a re-raised part look new (split parts get a copy of the notes list).
    same_traceback
        && same_optional_exception(&exception.cause, &orig.cause)
        && same_optional_exception(&exception.context, &orig.context)
}

fn collect_exception_leaf_identities(exception: &MiniException, identities: &mut Vec<Rc<()>>) {
    match &exception.exceptions {
        Some(children) => {
            for child in children {
                collect_exception_leaf_identities(child, identities);
            }
        }
        None => identities.push(exception.identity.clone()),
    }
}

impl Vm {
    fn exception_group_matcher_matches(
        &mut self,
        exception: &MiniException,
        matcher: &ExceptionGroupMatcher,
    ) -> Result<bool, String> {
        match matcher {
            ExceptionGroupMatcher::Classes(classes) => {
                Ok(exception_matches_classes(exception, classes))
            }
            ExceptionGroupMatcher::Predicate(predicate) => {
                let result =
                    self.call_value(predicate.clone(), vec![value_from_exception(exception)])?;
                self.truth_value(result)
            }
            ExceptionGroupMatcher::LeafIdentities(identities) => Ok(exception.exceptions.is_none()
                && identities
                    .iter()
                    .any(|identity| Rc::ptr_eq(identity, &exception.identity))),
        }
    }

    /// CPython `exceptiongroup_split_recursive`.
    fn split_exception_group(
        &mut self,
        exception: &MiniException,
        matcher: &ExceptionGroupMatcher,
        construct_rest: bool,
    ) -> Result<(Option<MiniException>, Option<MiniException>), String> {
        if self.exception_group_matcher_matches(exception, matcher)? {
            return Ok((Some(exception.clone()), None));
        }
        let Some(children) = &exception.exceptions else {
            return Ok((None, construct_rest.then(|| exception.clone())));
        };
        let mut matched = Vec::new();
        let mut rest = Vec::new();
        for child in children {
            let (child_match, child_rest) =
                self.split_exception_group(child, matcher, construct_rest)?;
            matched.extend(child_match);
            rest.extend(child_rest);
        }
        let matched = exception_group_subset(exception, matched)?;
        let rest = if construct_rest {
            exception_group_subset(exception, rest)?
        } else {
            None
        };
        Ok((matched, rest))
    }

    /// `BaseExceptionGroup.split(matcher)` and `.subgroup(matcher)`.
    pub(crate) fn call_exception_group_split(
        &mut self,
        name: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let method = name.rsplit('.').next().unwrap_or(name);
        if !keywords.is_empty() {
            return Err(format!(
                "TypeError: BaseExceptionGroup.{method}() takes no keyword arguments"
            ));
        }
        let [receiver, matcher] = <[Value; 2]>::try_from(args).map_err(|args| {
            format!(
                "TypeError: BaseExceptionGroup.{method}() takes exactly one argument ({} given)",
                args.len().saturating_sub(1)
            )
        })?;
        let exception = exception_from_value(receiver)?;
        let matcher = exception_group_matcher(matcher)?;
        let construct_rest = method == "split";
        let (matched, rest) = self.split_exception_group(&exception, &matcher, construct_rest)?;
        let to_value = |exception: Option<MiniException>| {
            exception
                .as_ref()
                .map(value_from_exception)
                .unwrap_or(Value::None)
        };
        if construct_rest {
            Ok(tuple_value(vec![to_value(matched), to_value(rest)]))
        } else {
            Ok(to_value(matched))
        }
    }

    /// `BaseExceptionGroup.derive(excs)`.
    pub(crate) fn call_exception_group_derive(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        if !keywords.is_empty() {
            return Err(
                "TypeError: BaseExceptionGroup.derive() takes no keyword arguments".to_string(),
            );
        }
        let [receiver, excs] = <[Value; 2]>::try_from(args).map_err(|args| {
            format!(
                "TypeError: BaseExceptionGroup.derive() takes exactly one argument ({} given)",
                args.len().saturating_sub(1)
            )
        })?;
        let exception = exception_from_value(receiver)?;
        let message = exception
            .args
            .first()
            .cloned()
            .unwrap_or_else(|| Value::String(exception.message.clone().unwrap_or_default()));
        self.call_value(
            Value::Builtin("BaseExceptionGroup".to_string()),
            vec![message, excs],
        )
    }

    /// `CheckExceptStarMatch`: CPython `CHECK_EG_MATCH`.
    pub(crate) fn check_except_star_match(
        &mut self,
        rest: Register,
        matcher: Register,
        match_dst: Register,
        target: usize,
    ) -> Result<(), String> {
        let matcher = self.read_register(matcher)?.clone();
        if let Err(message) = check_except_star_matcher(&matcher) {
            return self.raise_exception(type_error_exception(&message), true);
        }
        let rest_value = self.read_register(rest)?.clone();
        if matches!(rest_value, Value::None) {
            return self.jump_to(target);
        }
        let remaining = exception_from_value(rest_value)?;
        let (matched, remaining) = if exception_matches_classes(&remaining, &matcher) {
            let matched = if remaining.exceptions.is_some() {
                remaining
            } else {
                wrap_naked_exception_for_except_star(&remaining)?
            };
            (Some(matched), None)
        } else if remaining.exceptions.is_some() {
            self.split_exception_group(&remaining, &ExceptionGroupMatcher::Classes(matcher), true)?
        } else {
            (None, Some(remaining))
        };
        self.write_register(
            rest,
            remaining
                .as_ref()
                .map(value_from_exception)
                .unwrap_or(Value::None),
        );
        let Some(matched) = matched else {
            return self.jump_to(target);
        };
        self.write_register(match_dst, value_from_exception(&matched));
        self.current_exception = Some(matched);
        Ok(())
    }

    /// `CollectExceptStarRaise`: records the exception an `except*` clause
    /// body raised.
    pub(crate) fn collect_except_star_raise(
        &mut self,
        raised: Register,
        handled: Register,
    ) -> Result<(), String> {
        let exception = self
            .current_exception
            .take()
            .ok_or_else(|| "except* clause raised no exception".to_string())?;
        let Value::List(items) = self.read_register(raised)?.clone() else {
            return Err("except* raised exceptions are not a list".to_string());
        };
        items.borrow_mut().push(value_from_exception(&exception));
        // The clause's collect handler is done; the clause's matched group
        // is the exception being handled again.
        self.handled_exceptions.pop();
        self.current_exception = Some(exception_from_value(self.read_register(handled)?.clone())?);
        Ok(())
    }

    /// End of a `try`/`except*`: combines the unhandled and newly raised
    /// exceptions into the group to re-raise, then raises it.
    pub(crate) fn reraise_except_star(
        &mut self,
        orig: Register,
        rest: Register,
        raised: Register,
    ) -> Result<(), String> {
        let orig = exception_from_value(self.read_register(orig)?.clone())?;
        let Value::List(items) = self.read_register(raised)?.clone() else {
            return Err("except* raised exceptions are not a list".to_string());
        };
        let mut excs = items.borrow().clone();
        excs.push(self.read_register(rest)?.clone());

        let result = if orig.exceptions.is_none() {
            // A naked exception was wrapped: at most one clause ran.
            match excs.into_iter().next() {
                Some(Value::None) | None => None,
                Some(value) => Some(exception_from_value(value)?),
            }
        } else {
            let mut raised_excs = Vec::new();
            let mut reraised_excs = Vec::new();
            for value in excs {
                if matches!(value, Value::None) {
                    continue;
                }
                let exception = exception_from_value(value)?;
                if is_same_exception_metadata(&exception, &orig) {
                    reraised_excs.push(exception);
                } else {
                    raised_excs.push(exception);
                }
            }
            let reraised = if reraised_excs.is_empty() {
                None
            } else {
                let mut identities = Vec::new();
                for exception in &reraised_excs {
                    collect_exception_leaf_identities(exception, &mut identities);
                }
                self.split_exception_group(
                    &orig,
                    &ExceptionGroupMatcher::LeafIdentities(identities),
                    false,
                )?
                .0
            };
            raised_excs.extend(reraised);
            if raised_excs.len() > 1 {
                let excs = list_value(raised_excs.iter().map(value_from_exception).collect());
                Some(new_exception_group(Value::String(String::new()), excs)?)
            } else {
                raised_excs.pop()
            }
        };

        match result {
            Some(exception) => {
                self.current_exception = Some(exception.clone());
                self.raise_exception(exception, false)
            }
            None => {
                self.leave_exception_handler();
                Ok(())
            }
        }
    }
}
