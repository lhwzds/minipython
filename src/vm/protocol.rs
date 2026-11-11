// SPDX-License-Identifier: MIT

//! Object identity, rich equality, and hashing for the virtual machine.

use super::*;

/// The items of a sequence, read one at a time.
///
/// A comparison of the items can run user code, which can change a
/// mutable sequence, so the items are read again for every step: the
/// length first, and then the item at that index.
#[derive(Clone)]
pub(crate) enum SequenceItems {
    /// A list, a user list, or another sequence with a mutable store.
    Mutable(ListRef),
    /// A deque, whose iteration checks its change counter.
    Deque(ListRef, Rc<Cell<u64>>),
    /// A tuple or another sequence with an immutable store.
    Fixed(Rc<Vec<Value>>),
}

impl SequenceItems {
    /// The number of items the sequence holds now.
    fn len(&self) -> usize {
        match self {
            SequenceItems::Mutable(data) => data.borrow().len(),
            SequenceItems::Deque(data, _) => data.borrow().len(),
            SequenceItems::Fixed(items) => items.len(),
        }
    }
}

/// A reader of the items of a sequence.
///
/// The reader holds no borrow of the sequence, so the comparison of an
/// item can run user code that changes the sequence.
pub(crate) struct SequenceReader {
    items: SequenceItems,
    /// The change counter when reading a deque started.
    expected_state: u64,
    /// The index of the next item.
    index: usize,
}

impl SequenceReader {
    pub(crate) fn new(items: SequenceItems) -> Self {
        let expected_state = match &items {
            SequenceItems::Deque(_, state) => state.get(),
            _ => 0,
        };
        SequenceReader {
            items,
            expected_state,
            index: 0,
        }
    }

    /// The number of items the sequence holds now.
    pub(crate) fn len(&self) -> usize {
        self.items.len()
    }

    /// The number of items the reader has read.
    pub(crate) fn index(&self) -> usize {
        self.index
    }

    /// The item at `index`, as it is now.
    pub(crate) fn item(&self, index: usize) -> Option<Value> {
        match &self.items {
            SequenceItems::Mutable(data) => data.borrow().get(index).cloned(),
            SequenceItems::Deque(data, _) => data.borrow().get(index).cloned(),
            SequenceItems::Fixed(items) => items.get(index).cloned(),
        }
    }

    /// The next item of the sequence, or `None` when the sequence ends.
    ///
    /// A deque reports a change of its counter.
    pub(crate) fn next(&mut self) -> Result<Option<Value>, String> {
        let index = self.index;
        let item = match &self.items {
            SequenceItems::Mutable(data) => {
                let data = data.borrow();
                data.get(index).cloned()
            }
            SequenceItems::Deque(data, state) => {
                if state.get() != self.expected_state {
                    return Err(DEQUE_MUTATED_DURING_ITERATION.to_string());
                }
                let data = data.borrow();
                data.get(index).cloned()
            }
            SequenceItems::Fixed(items) => items.get(index).cloned(),
        };
        if item.is_some() {
            self.index += 1;
        }
        Ok(item)
    }
}

impl Vm {
    pub(crate) fn hash_key_value(&mut self, value: &Value) -> Result<Value, String> {
        // Nested hashable containers (tuples, frozensets) hash recursively.
        crate::stack::check_native_recursion("")?;
        if let Value::MappingProxyObject { mapping, .. } = value {
            return self.hash_key_value(mapping);
        }

        if let Value::WeakRef { target, .. } = value {
            return self.hash_key_value(target);
        }

        if let Some(alias) = generic_alias_subclass_alias(value) {
            return self.hash_key_value(&alias);
        }

        if let Value::FrozenSet(items) = value {
            return self.frozenset_hash(items.as_ref().clone());
        }

        if let Value::GenericAlias {
            origin,
            args,
            union_unhashable_count,
            ..
        } = value
            && matches!(origin.as_ref(), Value::Builtin(name) if name == "Union")
        {
            return self.hash_union_alias_value(args, *union_unhashable_count);
        }

        if dict_view_is_values_hashable_value(value) {
            return Ok(identity_hash_value(value));
        }

        if matches!(value, Value::Cell { .. }) {
            return Err("TypeError: unhashable type: 'cell'".to_string());
        }

        if let Some(method) = instance_hash_method(value)? {
            let result = match self.call_value_catching(method, Vec::new())? {
                Ok(value) => value,
                Err(exception) => return Err(self.propagate_captured_exception(exception)),
            };
            return hash_result_from_special_method(result);
        }

        if let Some(method) = class_hash_method(value)? {
            let result = match self.call_value_catching(method, Vec::new())? {
                Ok(value) => value,
                Err(exception) => return Err(self.propagate_captured_exception(exception)),
            };
            return hash_result_from_special_method(result);
        }

        hash_value(value)
    }

    /// `left == right` as a boolean.
    pub(crate) fn equal_values(&mut self, left: Value, right: Value) -> Result<bool, String> {
        self.rich_equal_values(&left, &right)
    }

    /// `left == right` as CPython answers it: the value the comparison method
    /// returned, unchanged, rather than a boolean derived from it.
    pub(super) fn equal_result_values(
        &mut self,
        left: &Value,
        right: &Value,
    ) -> Result<Value, String> {
        crate::stack::check_native_recursion(COMPARISON_RECURSION_CONTEXT)?;
        if let Some(value) = self.cmp_to_key_equal_values(left, right)? {
            return Ok(Value::Bool(value));
        }

        // CPython's rich comparison protocol: the reflected method of a
        // proper subclass on the right has priority, then the left operand's
        // own method answers, and the right operand's method answers when
        // the left one declines. A user-defined `__eq__` runs even when both
        // operands are the same object.
        let reflected_first = right_type_is_strict_subclass(left, right);

        if reflected_first {
            if let Some(value) = self.operand_equal_values(right, left)? {
                return Ok(value);
            }
        }
        if let Some(value) = self.operand_equal_values(left, right)? {
            return Ok(value);
        }
        if !reflected_first {
            if let Some(value) = self.operand_equal_values(right, left)? {
                return Ok(value);
            }
        }
        // Both operands declined the comparison: CPython compares them by
        // identity.
        Ok(Value::Bool(is_identical(left, right)))
    }

    /// `left == right` as a boolean.
    pub(crate) fn rich_equal_values(
        &mut self,
        left: &Value,
        right: &Value,
    ) -> Result<bool, String> {
        let result = self.equal_result_values(left, right)?;
        self.truth_value(result)
    }

    /// The comparison of two items of a container, as CPython's container
    /// operations answer it.
    ///
    /// Identical items are equal, and other items are compared with the
    /// rich equality, whose result is truth-tested.
    pub(crate) fn container_items_equal(
        &mut self,
        left: &Value,
        right: &Value,
    ) -> Result<bool, String> {
        if is_identical(left, right) {
            return Ok(true);
        }
        self.rich_equal_values(left, right)
    }

    /// The equality of two sequences, whose items are read one at a time.
    ///
    /// The items are read again for every step, so a comparison of an
    /// item that changes a sequence sees the sequence as it is now.
    pub(crate) fn rich_sequence_items_equal(
        &mut self,
        left: &mut SequenceReader,
        right: &mut SequenceReader,
    ) -> Result<bool, String> {
        if left.len() != right.len() {
            return Ok(false);
        }
        loop {
            let left_item = left.next()?;
            let right_item = right.next()?;
            let (Some(left_item), Some(right_item)) = (left_item, right_item) else {
                break;
            };
            if !self.container_items_equal(&left_item, &right_item)? {
                return Ok(false);
            }
        }
        Ok(left.len() == right.len())
    }

    /// The equality of two deques, whose items are read one at a time.
    ///
    /// A change of a deque counter is reported by its iteration,
    /// and the items of two deques are compared again when they differ,
    /// so the second comparison answers.
    pub(crate) fn rich_deque_items_equal(
        &mut self,
        left: &mut SequenceReader,
        right: &mut SequenceReader,
    ) -> Result<bool, String> {
        if left.len() != right.len() {
            return Ok(false);
        }
        loop {
            let left_item = left.next()?;
            let right_item = right.next()?;
            let (Some(left_item), Some(right_item)) = (left_item, right_item) else {
                break;
            };
            if !self.deque_items_equal(&left_item, &right_item)? {
                return Ok(false);
            }
        }
        Ok(left.len() == right.len())
    }

    /// The comparison of the items of two deques.
    ///
    /// CPython compares the items with the equality protocol, and
    /// compares them again when they differ, so the second comparison
    /// answers.
    fn deque_items_equal(&mut self, left: &Value, right: &Value) -> Result<bool, String> {
        if is_identical(left, right) {
            return Ok(true);
        }
        if self.rich_equal_values(left, right)? {
            return Ok(true);
        }
        self.rich_equal_values(left, right)
    }

    /// The ordering of two deques.
    ///
    /// CPython reads the items of the deques before it compares them, so a
    /// comparison that changes a deque does not change the items that are
    /// compared.
    pub(super) fn rich_deque_items_order(
        &mut self,
        left: &ListRef,
        left_state: &Rc<Cell<u64>>,
        right: &ListRef,
        right_state: &Rc<Cell<u64>>,
        op: SequenceOrder,
    ) -> Result<bool, String> {
        let mut left = SequenceReader::new(SequenceItems::Deque(left.clone(), left_state.clone()));
        let mut right =
            SequenceReader::new(SequenceItems::Deque(right.clone(), right_state.clone()));
        loop {
            let left_item = left.next()?;
            let right_item = right.next()?;
            let (Some(left_item), Some(right_item)) = (left_item, right_item) else {
                break;
            };
            // CPython compares the items with an identity shortcut, so the
            // same object never reaches a user-defined `__eq__`.
            if is_identical(&left_item, &right_item)
                || self.rich_equal_values(&left_item, &right_item)?
            {
                continue;
            }
            let result = self.sequence_item_order(left_item, right_item, op)?;
            return self.truth_value(result);
        }
        Ok(match op {
            SequenceOrder::Less => left.len() < right.len(),
            SequenceOrder::LessEqual => left.len() <= right.len(),
            SequenceOrder::Greater => left.len() > right.len(),
            SequenceOrder::GreaterEqual => left.len() >= right.len(),
        })
    }

    /// The ordering of two sequences, whose items are read one at a time.
    ///
    /// The items are read again for every step, so a comparison of an
    /// item that changes a sequence sees the sequence as it is now.
    pub(super) fn rich_sequence_items_order(
        &mut self,
        left: &mut SequenceReader,
        right: &mut SequenceReader,
        op: SequenceOrder,
    ) -> Result<Value, String> {
        loop {
            let left_item = left.next()?;
            let right_item = right.next()?;
            let (Some(left_item), Some(right_item)) = (left_item, right_item) else {
                break;
            };
            // CPython compares the items with an identity shortcut, so the
            // same object never reaches a user-defined `__eq__`.
            if is_identical(&left_item, &right_item)
                || self.rich_equal_values(&left_item, &right_item)?
            {
                continue;
            }
            // The items differ.  When one of the sequences ran out while
            // they were compared, the lengths decide; otherwise the items
            // at this index are compared again, as they are now.
            if left.index() > left.len() || right.index() > right.len() {
                break;
            }
            let index = left.index() - 1;
            let (Some(left_item), Some(right_item)) = (left.item(index), right.item(index)) else {
                break;
            };
            return self.sequence_item_order(left_item, right_item, op);
        }

        Ok(Value::Bool(match op {
            SequenceOrder::Less => left.len() < right.len(),
            SequenceOrder::LessEqual => left.len() <= right.len(),
            SequenceOrder::Greater => left.len() > right.len(),
            SequenceOrder::GreaterEqual => left.len() >= right.len(),
        }))
    }

    /// Whether the comparison of a user string lets the other operand
    /// answer with a comparison method of its own.
    ///
    /// A user string compares its text, so an operand with a comparison
    /// method of its own answers the comparison, while a user string answers
    /// with the text of both operands.
    fn user_string_comparison_answers(&mut self, right: &Value, name: &str) -> bool {
        if is_user_string_value(right) {
            return false;
        }
        matches!(
            self.operand_comparison_method(right, name),
            ComparisonMethod::User(_)
        )
    }

    /// The `==` answer the operand's type provides. `None` marks a declined
    /// comparison, which lets the other operand answer instead.
    pub(crate) fn operand_equal_values(
        &mut self,
        left: &Value,
        right: &Value,
    ) -> Result<Option<Value>, String> {
        match self.operand_comparison_method(left, "__eq__") {
            ComparisonMethod::User(method) => {
                let result = self.call_value(method, vec![right.clone()])?;
                if matches!(result, Value::NotImplemented) {
                    return Ok(None);
                }
                Ok(Some(result))
            }
            ComparisonMethod::Builtin if matches!(left, Value::UserString { .. }) => {
                // A user string compares its text, so an operand with a
                // comparison method of its own answers the comparison
                // instead. The text answers the comparison with the other
                // operands.
                if self.user_string_comparison_answers(right, "__eq__") {
                    let text = user_string_text_value(left);
                    return Ok(Some(self.equal_result_values(&text, right)?));
                }
                Ok(user_string_rich_equal_values(left, right).map(Value::Bool))
            }
            ComparisonMethod::Builtin
                if str_method_text(left).is_some() && matches!(right, Value::UserString { .. }) =>
            {
                Ok(None)
            }
            ComparisonMethod::Builtin => {
                // The builtin comparison answers a comparison with an operand
                // of a related type, and declines the other ones.
                if !builtin_comparison_answers(left, right) {
                    // CPython warns from the byte string side of a comparison
                    // with an operand of an unrelated type before it declines
                    // the comparison.
                    if let Some(message) = bytes_comparison_warning_message_from(left, right) {
                        self.emit_bytes_comparison_warning(message)?;
                    }
                    return Ok(None);
                }
                self.builtin_equal_values(left, right)
                    .map(|equal| Some(Value::Bool(equal)))
            }
            ComparisonMethod::Object => self.object_rich_compare_values("__eq__", left, right),
        }
    }

    /// The builtin comparison of two operands, without the user-defined
    /// comparison methods CPython runs first.
    pub(crate) fn builtin_equal_values(
        &mut self,
        left: &Value,
        right: &Value,
    ) -> Result<bool, String> {
        crate::stack::check_native_recursion(COMPARISON_RECURSION_CONTEXT)?;

        if let Some(result) = self.dict_item_view_equal(left, right)? {
            return Ok(result);
        }

        if self.is_set_like_equality_operand(left) || self.is_set_like_equality_operand(right) {
            if !self.is_set_like_equality_operand(left) || !self.is_set_like_equality_operand(right)
            {
                return Ok(false);
            }
            let left = self.set_operator_values(left.clone(), "==")?;
            let right = self.set_operator_values(right.clone(), "==")?;
            return self.set_values_equal(&left, &right);
        }

        // CPython only takes the identity shortcut for operands whose type
        // does not define its own comparison method; a user-defined `__eq__`
        // runs even when both operands are the same object.
        if is_identical(left, right)
            && !matches!(left, Value::Float(_))
            && !matches!(left, Value::Complex { .. })
            && float_subclass_float(left).is_none()
            && complex_subclass_parts(left).is_none()
        {
            return Ok(true);
        }

        if let Some(value) = self.chain_map_equal_values(left, right)? {
            return Ok(value);
        }

        if let (Value::Re(left), Value::Re(right)) = (left, right) {
            return Ok(left.equals(right));
        }

        if let (Value::CodeObject { .. }, Value::CodeObject { .. }) = (left, right) {
            return Ok(code_objects_equal(left, right));
        }

        if let Some(value) = self.cell_rich_equal_if_cells(left, right)? {
            return Ok(value);
        }

        if let Some(value) = self.counter_equal_values(left, right)? {
            return Ok(value);
        }

        if let Some(value) = self.counter_mapping_equal(left, right)? {
            return Ok(value);
        }

        if let Some(value) = self.scope_backed_mapping_equal(left, right)? {
            return Ok(value);
        }

        if array_array_storage(left).is_some() || array_array_storage(right).is_some() {
            return if array_array_storage(left).is_some() && array_array_storage(right).is_some() {
                let left = array_array_values(left).expect("array storage exists after guard");
                let right = array_array_values(right).expect("array storage exists after guard");
                self.rich_sequence_equal(&left, &right)
            } else {
                Ok(false)
            };
        }

        if let Some(value) = self.union_aliases_equal(left, right)? {
            return Ok(value);
        }

        if let Some(value) = bound_methods_directly_equal(left, right) {
            return Ok(value);
        }

        if let Some(value) = user_string_rich_equal_values(left, right) {
            return Ok(value);
        }

        if let Some(message) = bytes_comparison_warning_message(left, right) {
            self.emit_bytes_comparison_warning(message)?;
        }

        if let Some(left_items) = list_subclass_storage(left) {
            match right {
                Value::List(right_items) => {
                    let mut left = SequenceReader::new(SequenceItems::Mutable(left_items));
                    let mut right =
                        SequenceReader::new(SequenceItems::Mutable(right_items.clone()));
                    return self.rich_sequence_items_equal(&mut left, &mut right);
                }
                _ => {
                    if let Some(right_items) = list_subclass_storage(right) {
                        let mut left = SequenceReader::new(SequenceItems::Mutable(left_items));
                        let mut right = SequenceReader::new(SequenceItems::Mutable(right_items));
                        return self.rich_sequence_items_equal(&mut left, &mut right);
                    }
                }
            }
        }

        if let Some(left_items) = tuple_subclass_items(left) {
            match right {
                Value::Tuple(right_items) => {
                    return self.rich_sequence_equal(left_items.as_ref(), right_items.as_ref());
                }
                Value::NamedTuple { values, .. } => {
                    return self.rich_sequence_equal(left_items.as_ref(), values.as_ref());
                }
                value if tuple_subclass_items(value).is_some() => {
                    let right_items = tuple_subclass_items(value)
                        .expect("tuple subclass items exist after guard");
                    return self.rich_sequence_equal(left_items.as_ref(), right_items.as_ref());
                }
                _ => {}
            }
        }

        if let Some(right_items) = tuple_subclass_items(right) {
            match left {
                Value::Tuple(left_items) => {
                    return self.rich_sequence_equal(left_items.as_ref(), right_items.as_ref());
                }
                Value::NamedTuple { values, .. } => {
                    return self.rich_sequence_equal(values.as_ref(), right_items.as_ref());
                }
                _ => {}
            }
        }

        match (left, right) {
            (Value::Tuple(left), Value::Tuple(right)) => {
                return self.rich_sequence_equal(left.as_ref(), right.as_ref());
            }
            (
                Value::Deque {
                    data: left,
                    state: left_state,
                    ..
                },
                Value::Deque {
                    data: right,
                    state: right_state,
                    ..
                },
            ) => {
                let mut left =
                    SequenceReader::new(SequenceItems::Deque(left.clone(), left_state.clone()));
                let mut right =
                    SequenceReader::new(SequenceItems::Deque(right.clone(), right_state.clone()));
                return self.rich_deque_items_equal(&mut left, &mut right);
            }
            (Value::List(left), Value::List(right)) => {
                let mut left = SequenceReader::new(SequenceItems::Mutable(left.clone()));
                let mut right = SequenceReader::new(SequenceItems::Mutable(right.clone()));
                return self.rich_sequence_items_equal(&mut left, &mut right);
            }
            (Value::UserList { data: left, .. }, Value::UserList { data: right, .. }) => {
                let mut left = SequenceReader::new(SequenceItems::Mutable(left.clone()));
                let mut right = SequenceReader::new(SequenceItems::Mutable(right.clone()));
                return self.rich_sequence_items_equal(&mut left, &mut right);
            }
            (Value::List(left), Value::UserList { data: right, .. })
            | (Value::UserList { data: right, .. }, Value::List(left)) => {
                let mut left = SequenceReader::new(SequenceItems::Mutable(left.clone()));
                let mut right = SequenceReader::new(SequenceItems::Mutable(right.clone()));
                return self.rich_sequence_items_equal(&mut left, &mut right);
            }
            (Value::Dict(left), Value::Dict(right)) => {
                return self.dict_storage_entries_equal(left, right);
            }
            (Value::UserDict { data: left, .. }, Value::UserDict { data: right, .. }) => {
                return self.dict_storage_entries_equal(left, right);
            }
            (Value::SimpleNamespace { fields: left }, Value::SimpleNamespace { fields: right }) => {
                let left = left.borrow().entries.clone();
                let right = right.borrow().entries.clone();
                if let Some(value) = code_metadata_namespace_entries_equal(&left, &right) {
                    return Ok(value);
                }
                return Ok(
                    left.len() == right.len() && self.dict_item_entries_subset(&left, &right)?
                );
            }
            _ => {}
        }

        if let (Some(left_bytes), Some(right_bytes)) = (
            bytes_equality_value_bytes(left),
            bytes_equality_value_bytes(right),
        ) {
            return Ok(left_bytes == right_bytes);
        }

        if let (
            Value::WeakRef {
                target: left_target,
                ..
            },
            Value::WeakRef {
                target: right_target,
                ..
            },
        ) = (left, right)
        {
            return self.rich_equal_values(left_target, right_target);
        }

        if let Some(value) = forward_ref_equal_values(left, right)? {
            return Ok(value);
        }

        if let Some(left) = generic_alias_subclass_alias(left) {
            return self.rich_equal_values(&left, right);
        }
        if let Some(right) = generic_alias_subclass_alias(right) {
            return self.rich_equal_values(left, &right);
        }
        // The builtin value behind a numeric subclass compares without the
        // user-defined methods CPython already ran for the subclass.
        if let Some(left) = int_subclass_integer(left) {
            return self.builtin_equal_values(&left, right);
        }
        if let Some(right) = int_subclass_integer(right) {
            return self.builtin_equal_values(left, &right);
        }
        if let Some(left) = float_subclass_value(left) {
            return self.builtin_equal_values(&left, right);
        }
        if let Some(right) = float_subclass_value(right) {
            return self.builtin_equal_values(left, &right);
        }
        if let Some(left) = complex_subclass_value(left) {
            return self.builtin_equal_values(&left, right);
        }
        if let Some(right) = complex_subclass_value(right) {
            return self.builtin_equal_values(left, &right);
        }
        if matches!(left, Value::Instance { .. }) || matches!(right, Value::Instance { .. }) {
            // Both operands declined the comparison: CPython compares them by
            // identity.
            return Ok(is_identical(left, right));
        }
        Ok(left == right)
    }

    /// `left != right` as a boolean.
    pub(crate) fn not_equal_values(&mut self, left: Value, right: Value) -> Result<bool, String> {
        let result = self.not_equal_result_values(&left, &right)?;
        self.truth_value(result)
    }

    /// `left != right` as CPython answers it.
    ///
    /// The reflected method of a proper subclass on the right is tried first,
    /// then the left operand's own method, and a comparison both operands
    /// decline falls back to identity. The answer of a method is returned
    /// unchanged; only the default `object.__ne__`, which inverts the truth of
    /// `__eq__`, answers a boolean.
    pub(super) fn not_equal_result_values(
        &mut self,
        left: &Value,
        right: &Value,
    ) -> Result<Value, String> {
        if let Some(value) = self.cmp_to_key_not_equal_values(left, right)? {
            return Ok(Value::Bool(value));
        }

        let reflected_first = right_type_is_strict_subclass(left, right);

        if reflected_first {
            if let Some(value) = self.operand_not_equal_values(right, left)? {
                return Ok(value);
            }
        }
        if let Some(value) = self.operand_not_equal_values(left, right)? {
            return Ok(value);
        }
        if !reflected_first {
            if let Some(value) = self.operand_not_equal_values(right, left)? {
                return Ok(value);
            }
        }
        // Both operands declined the comparison: CPython compares them by
        // identity.
        Ok(Value::Bool(!is_identical(left, right)))
    }

    /// The `!=` answer the operand's type provides. `None` marks a declined
    /// comparison, which lets the other operand answer instead.
    pub(crate) fn operand_not_equal_values(
        &mut self,
        left: &Value,
        right: &Value,
    ) -> Result<Option<Value>, String> {
        match self.operand_comparison_method(left, "__ne__") {
            ComparisonMethod::User(method) => {
                let result = self.call_value(method, vec![right.clone()])?;
                if matches!(result, Value::NotImplemented) {
                    return Ok(None);
                }
                Ok(Some(result))
            }
            ComparisonMethod::Builtin if matches!(left, Value::UserString { .. }) => {
                // A user string has no comparison of its own for
                // inequality, so the equality of its text answers the
                // comparison and the answer is inverted. The equality
                // unwraps the text, so an operand with a comparison method
                // of its own answers the comparison.
                self.object_rich_compare_values("__ne__", left, right)
            }
            ComparisonMethod::Builtin
                if str_method_text(left).is_some() && matches!(right, Value::UserString { .. }) =>
            {
                Ok(None)
            }
            ComparisonMethod::Builtin => {
                // The builtin `__ne__` compares the operands itself, and it
                // declines a comparison with an operand of an unrelated type.
                if !builtin_comparison_answers(left, right) {
                    // CPython warns from the byte string side of a comparison
                    // with an operand of an unrelated type before it declines
                    // the comparison.
                    if let Some(message) = bytes_comparison_warning_message_from(left, right) {
                        self.emit_bytes_comparison_warning(message)?;
                    }
                    return Ok(None);
                }
                self.builtin_equal_values(left, right)
                    .map(|equal| Some(Value::Bool(!equal)))
            }
            ComparisonMethod::Object => self.object_rich_compare_values("__ne__", left, right),
        }
    }

    pub(super) fn object_rich_compare_values(
        &mut self,
        method: &str,
        left: &Value,
        right: &Value,
    ) -> Result<Option<Value>, String> {
        match method {
            "__eq__" => Ok(is_identical(left, right).then_some(Value::Bool(true))),
            "__ne__" => match self.operand_equal_values(left, right)? {
                Some(equal) => Ok(Some(Value::Bool(!self.truth_value(equal)?))),
                None => Ok(None),
            },
            _ => Ok(None),
        }
    }
}

/// The text of a user string, which is what its comparison uses.
fn user_string_text_value(value: &Value) -> Value {
    match value {
        Value::UserString { data, .. } => Value::String(data.borrow().clone()),
        value => value.clone(),
    }
}

/// Whether the value is a user string or an instance of a user string class.
fn is_user_string_value(value: &Value) -> bool {
    match value {
        Value::UserString { .. } => true,
        Value::Instance { class_bases, .. } => {
            class_bases_include_builtin(class_bases, "UserString")
        }
        _ => false,
    }
}

pub(super) fn is_identical(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::None, Value::None)
        | (Value::NotImplemented, Value::NotImplemented)
        | (Value::Ellipsis, Value::Ellipsis)
        | (Value::FunctoolsPlaceholder, Value::FunctoolsPlaceholder)
        | (Value::Bool(true), Value::Bool(true))
        | (Value::Bool(false), Value::Bool(false)) => true,
        // The interpreter keeps an integer or a string by value, so the same
        // immutable scalar read twice is one object.
        (Value::Number(left), Value::Number(right)) => left == right,
        (Value::BigInt(left), Value::BigInt(right)) => left == right,
        (Value::String(left), Value::String(right)) => left == right,
        (Value::Bytes(left), Value::Bytes(right)) => {
            (left.is_empty() && right.is_empty()) || Rc::ptr_eq(left, right)
        }
        (Value::Float(left), Value::Float(right)) => Rc::ptr_eq(left, right),
        (
            Value::Complex {
                identity: left_identity,
                ..
            },
            Value::Complex {
                identity: right_identity,
                ..
            },
        ) => Rc::ptr_eq(left_identity, right_identity),
        (Value::List(left), Value::List(right)) => Rc::ptr_eq(left, right),
        (
            Value::UserList {
                data: left_data, ..
            },
            Value::UserList {
                data: right_data, ..
            },
        ) => Rc::ptr_eq(left_data, right_data),
        (Value::Tuple(left), Value::Tuple(right)) => Rc::ptr_eq(left, right),
        (
            Value::NamedTuple {
                values: left_values,
                ..
            },
            Value::NamedTuple {
                values: right_values,
                ..
            },
        ) => Rc::ptr_eq(left_values, right_values),
        (Value::ByteArray(left), Value::ByteArray(right)) => Rc::ptr_eq(left, right),
        (Value::Deque { data: left, .. }, Value::Deque { data: right, .. }) => {
            Rc::ptr_eq(left, right)
        }
        (Value::Set(left), Value::Set(right)) => Rc::ptr_eq(left, right),
        (Value::FrozenSet(left), Value::FrozenSet(right)) => Rc::ptr_eq(left, right),
        (
            Value::Range {
                identity: left_identity,
                ..
            },
            Value::Range {
                identity: right_identity,
                ..
            },
        ) => Rc::ptr_eq(left_identity, right_identity),
        (Value::Dict(left), Value::Dict(right)) => Rc::ptr_eq(left, right),
        (Value::OrderedDict(left), Value::OrderedDict(right)) => Rc::ptr_eq(left, right),
        (Value::Counter { entries: left }, Value::Counter { entries: right }) => {
            Rc::ptr_eq(left, right)
        }
        (Value::ScopeDict(left), Value::ScopeDict(right)) => Rc::ptr_eq(left, right),
        (
            Value::DictView {
                kind: left_kind,
                identity: left_identity,
                ..
            },
            Value::DictView {
                kind: right_kind,
                identity: right_identity,
                ..
            },
        ) => left_kind == right_kind && Rc::ptr_eq(left_identity, right_identity),
        (
            Value::MappingView {
                kind: left_kind,
                identity: left_identity,
                ..
            },
            Value::MappingView {
                kind: right_kind,
                identity: right_identity,
                ..
            },
        ) => left_kind == right_kind && Rc::ptr_eq(left_identity, right_identity),
        (
            Value::MappingProxy {
                identity: left_identity,
                ..
            },
            Value::MappingProxy {
                identity: right_identity,
                ..
            },
        ) => Rc::ptr_eq(left_identity, right_identity),
        (
            Value::MappingProxyObject {
                identity: left_identity,
                ..
            },
            Value::MappingProxyObject {
                identity: right_identity,
                ..
            },
        ) => Rc::ptr_eq(left_identity, right_identity),
        (
            Value::UserDict {
                data: left_data, ..
            },
            Value::UserDict {
                data: right_data, ..
            },
        ) => Rc::ptr_eq(left_data, right_data),
        (
            Value::DefaultDict {
                entries: left_entries,
                ..
            },
            Value::DefaultDict {
                entries: right_entries,
                ..
            },
        ) => Rc::ptr_eq(left_entries, right_entries),
        (
            Value::SimpleNamespace {
                fields: left_fields,
            },
            Value::SimpleNamespace {
                fields: right_fields,
            },
        ) => Rc::ptr_eq(left_fields, right_fields),
        (
            Value::Frame {
                fields: left_fields,
            },
            Value::Frame {
                fields: right_fields,
            },
        ) => Rc::ptr_eq(left_fields, right_fields),
        (
            Value::Class {
                attrs: left_attrs, ..
            },
            Value::Class {
                attrs: right_attrs, ..
            },
        ) => Rc::ptr_eq(left_attrs, right_attrs),
        (Value::NamedTupleType(left), Value::NamedTupleType(right)) => {
            Rc::ptr_eq(&left.identity, &right.identity)
        }
        (
            Value::TypeParam {
                identity: left_identity,
                ..
            },
            Value::TypeParam {
                identity: right_identity,
                ..
            },
        ) => Rc::ptr_eq(left_identity, right_identity),
        (
            Value::ParamSpecAccess {
                identity: left_identity,
                ..
            },
            Value::ParamSpecAccess {
                identity: right_identity,
                ..
            },
        ) => Rc::ptr_eq(left_identity, right_identity),
        (
            Value::NamedTupleFieldDescriptor {
                typ: left_typ,
                index: left_index,
            },
            Value::NamedTupleFieldDescriptor {
                typ: right_typ,
                index: right_index,
            },
        ) => Rc::ptr_eq(left_typ, right_typ) && left_index == right_index,
        (
            Value::NamedTupleTypeMethod {
                identity: left_identity,
                ..
            },
            Value::NamedTupleTypeMethod {
                identity: right_identity,
                ..
            },
        )
        | (
            Value::NamedTupleInstanceMethod {
                identity: left_identity,
                ..
            },
            Value::NamedTupleInstanceMethod {
                identity: right_identity,
                ..
            },
        ) => Rc::ptr_eq(left_identity, right_identity),
        (
            Value::GenericAlias {
                identity: left_identity,
                ..
            },
            Value::GenericAlias {
                identity: right_identity,
                ..
            },
        ) => Rc::ptr_eq(left_identity, right_identity),
        (
            Value::Instance {
                fields: left_fields,
                ..
            },
            Value::Instance {
                fields: right_fields,
                ..
            },
        ) => Rc::ptr_eq(left_fields, right_fields),
        (
            Value::Exception {
                identity: left_identity,
                ..
            },
            Value::Exception {
                identity: right_identity,
                ..
            },
        ) => Rc::ptr_eq(left_identity, right_identity),
        (
            Value::Module {
                attrs: left_attrs, ..
            },
            Value::Module {
                attrs: right_attrs, ..
            },
        ) => Rc::ptr_eq(left_attrs, right_attrs),
        (
            Value::Function {
                identity: left_identity,
                ..
            },
            Value::Function {
                identity: right_identity,
                ..
            },
        ) => Rc::ptr_eq(left_identity, right_identity),
        (
            Value::MagicMock {
                identity: left_identity,
                ..
            },
            Value::MagicMock {
                identity: right_identity,
                ..
            },
        )
        | (
            Value::MockMethod {
                identity: left_identity,
                ..
            },
            Value::MockMethod {
                identity: right_identity,
                ..
            },
        ) => Rc::ptr_eq(left_identity, right_identity),
        (
            Value::Super {
                identity: left_identity,
                ..
            },
            Value::Super {
                identity: right_identity,
                ..
            },
        ) => Rc::ptr_eq(left_identity, right_identity),
        (
            Value::BoundMethod {
                identity: left_identity,
                ..
            },
            Value::BoundMethod {
                identity: right_identity,
                ..
            },
        ) => Rc::ptr_eq(left_identity, right_identity),
        (
            Value::Partial {
                identity: left_identity,
                ..
            },
            Value::Partial {
                identity: right_identity,
                ..
            },
        )
        | (
            Value::LruCacheWrapper {
                identity: left_identity,
                ..
            },
            Value::LruCacheWrapper {
                identity: right_identity,
                ..
            },
        )
        | (
            Value::SingleDispatch {
                identity: left_identity,
                ..
            },
            Value::SingleDispatch {
                identity: right_identity,
                ..
            },
        )
        | (
            Value::SingleDispatchRegister {
                identity: left_identity,
                ..
            },
            Value::SingleDispatchRegister {
                identity: right_identity,
                ..
            },
        )
        | (
            Value::SingleDispatchMethod {
                identity: left_identity,
                ..
            },
            Value::SingleDispatchMethod {
                identity: right_identity,
                ..
            },
        )
        | (
            Value::SingleDispatchMethodCallable {
                identity: left_identity,
                ..
            },
            Value::SingleDispatchMethodCallable {
                identity: right_identity,
                ..
            },
        )
        | (
            Value::PartialMethod {
                identity: left_identity,
                ..
            },
            Value::PartialMethod {
                identity: right_identity,
                ..
            },
        )
        | (
            Value::PartialMethodCall {
                identity: left_identity,
                ..
            },
            Value::PartialMethodCall {
                identity: right_identity,
                ..
            },
        )
        | (
            Value::CachedProperty {
                identity: left_identity,
                ..
            },
            Value::CachedProperty {
                identity: right_identity,
                ..
            },
        )
        | (
            Value::OperatorAttrGetter {
                identity: left_identity,
                ..
            },
            Value::OperatorAttrGetter {
                identity: right_identity,
                ..
            },
        )
        | (
            Value::OperatorItemGetter {
                identity: left_identity,
                ..
            },
            Value::OperatorItemGetter {
                identity: right_identity,
                ..
            },
        )
        | (
            Value::OperatorMethodCaller {
                identity: left_identity,
                ..
            },
            Value::OperatorMethodCaller {
                identity: right_identity,
                ..
            },
        ) => Rc::ptr_eq(left_identity, right_identity),
        (
            Value::TypesCoroutineFunction {
                identity: left_identity,
                ..
            },
            Value::TypesCoroutineFunction {
                identity: right_identity,
                ..
            },
        ) => Rc::ptr_eq(left_identity, right_identity),
        (
            Value::CodeObject {
                identity: left_identity,
                ..
            },
            Value::CodeObject {
                identity: right_identity,
                ..
            },
        ) => Rc::ptr_eq(left_identity, right_identity),
        (
            Value::Cell {
                identity: left_identity,
                ..
            },
            Value::Cell {
                identity: right_identity,
                ..
            },
        ) => Rc::ptr_eq(left_identity, right_identity),
        (
            Value::AstNode {
                identity: left_identity,
                ..
            },
            Value::AstNode {
                identity: right_identity,
                ..
            },
        ) => Rc::ptr_eq(left_identity, right_identity),
        (
            Value::Traceback {
                identity: left_identity,
                ..
            },
            Value::Traceback {
                identity: right_identity,
                ..
            },
        ) => Rc::ptr_eq(left_identity, right_identity),
        (
            Value::IdentityString {
                identity: left_identity,
                ..
            },
            Value::IdentityString {
                identity: right_identity,
                ..
            },
        ) => Rc::ptr_eq(left_identity, right_identity),
        (Value::String(left), Value::IdentityString { value: right, .. })
        | (Value::IdentityString { value: left, .. }, Value::String(right)) => left == right,
        (
            Value::MemberDescriptor {
                name: left_name,
                owner_name: left_owner_name,
                ..
            },
            Value::MemberDescriptor {
                name: right_name,
                owner_name: right_owner_name,
                ..
            },
        ) => left_name == right_name && left_owner_name == right_owner_name,
        (
            Value::Property {
                identity: left_identity,
                ..
            },
            Value::Property {
                identity: right_identity,
                ..
            },
        )
        | (
            Value::StaticMethod {
                identity: left_identity,
                ..
            },
            Value::StaticMethod {
                identity: right_identity,
                ..
            },
        )
        | (
            Value::ClassMethod {
                identity: left_identity,
                ..
            },
            Value::ClassMethod {
                identity: right_identity,
                ..
            },
        ) => Rc::ptr_eq(left_identity, right_identity),
        (
            Value::Template { identity: left, .. },
            Value::Template {
                identity: right, ..
            },
        ) => Rc::ptr_eq(left, right),
        (
            Value::AnextDefault { identity: left, .. },
            Value::AnextDefault {
                identity: right, ..
            },
        ) => Rc::ptr_eq(left, right),
        (
            Value::ConstEvaluator {
                kind: left_kind,
                target: left,
            },
            Value::ConstEvaluator {
                kind: right_kind,
                target: right,
            },
        ) => left_kind == right_kind && is_identical(left, right),
        (
            Value::TypeAlias { identity: left, .. },
            Value::TypeAlias {
                identity: right, ..
            },
        ) => Rc::ptr_eq(left, right),
        (Value::TemplateInterpolation(left), Value::TemplateInterpolation(right)) => {
            Rc::ptr_eq(&left.identity, &right.identity)
        }
        (Value::ItertoolsGroupBy { state: left }, Value::ItertoolsGroupBy { state: right }) => {
            Rc::ptr_eq(left, right)
        }
        (Value::Iterator(left), Value::Iterator(right)) => Rc::ptr_eq(left, right),
        (Value::Generator(left), Value::Generator(right)) => Rc::ptr_eq(left, right),
        (
            Value::GeneratorWrapper {
                identity: left_identity,
                ..
            },
            Value::GeneratorWrapper {
                identity: right_identity,
                ..
            },
        ) => Rc::ptr_eq(left_identity, right_identity),
        (
            Value::WeakRef {
                identity: left_identity,
                ..
            },
            Value::WeakRef {
                identity: right_identity,
                ..
            },
        ) => Rc::ptr_eq(left_identity, right_identity),
        (Value::Coroutine(left), Value::Coroutine(right)) => Rc::ptr_eq(left, right),
        (Value::CoroutineAwait(left), Value::CoroutineAwait(right)) => Rc::ptr_eq(left, right),
        (Value::AwaitIterator(_, left), Value::AwaitIterator(_, right)) => Rc::ptr_eq(left, right),
        (Value::Unpack(left), Value::Unpack(right))
        | (Value::PicklePayload(left), Value::PicklePayload(right)) => Rc::ptr_eq(left, right),
        (Value::AsyncGenerator(left), Value::AsyncGenerator(right)) => Rc::ptr_eq(left, right),
        (Value::MemoryView(left), Value::MemoryView(right)) => Rc::ptr_eq(left, right),
        (Value::Re(left), Value::Re(right)) => left.identity() == right.identity(),
        (Value::BytesIO(left), Value::BytesIO(right)) => Rc::ptr_eq(left, right),
        (Value::StringIOBuffer(left), Value::StringIOBuffer(right)) => Rc::ptr_eq(left, right),
        (Value::Builtin(left), Value::Builtin(right)) => left == right,
        _ => false,
    }
}

pub(super) fn identity_value(value: &Value) -> Value {
    normalize_big_int(BigInt::from(identity_bits(value)))
}

pub(super) fn identity_hash_value(value: &Value) -> Value {
    python_integer_hash_value(&BigInt::from(identity_bits(value)))
}

pub(super) fn identity_bits(value: &Value) -> u64 {
    match value {
        Value::AnextDefault { identity, .. } => rc_plain_identity_bits(identity),
        Value::ConstEvaluator { kind, target } => identity_bits(target) | (*kind as u64 + 1),
        Value::TypeAlias { identity, .. } | Value::Template { identity, .. } => {
            rc_plain_identity_bits(identity)
        }
        Value::TemplateInterpolation(interpolation) => {
            rc_plain_identity_bits(&interpolation.identity)
        }
        Value::Unpack(value) | Value::PicklePayload(value) => rc_plain_identity_bits(value),
        Value::AwaitIterator(_, identity) => rc_plain_identity_bits(identity),
        Value::NamedTupleType(typ) => rc_plain_identity_bits(&typ.identity),
        Value::DequeIterator { identity, .. } => rc_plain_identity_bits(identity),
        Value::ItertoolsGroupBy { state } => rc_identity_bits(state),
        Value::GenericAlias { identity, .. } => rc_plain_identity_bits(identity),
        Value::List(items) => rc_identity_bits(items),
        Value::Deque { data, .. } => rc_identity_bits(data),
        Value::UserList { data, .. } => rc_identity_bits(data),
        Value::UserString { data, .. } => rc_identity_bits(data),
        Value::IdentityString { identity, .. } => rc_plain_identity_bits(identity),
        Value::Bytes(bytes) if bytes.is_empty() => empty_bytes_identity_bits(),
        Value::Bytes(bytes) => rc_plain_identity_bits(bytes),
        Value::Tuple(items) => rc_plain_identity_bits(items),
        Value::NamedTuple { values, .. } => rc_plain_identity_bits(values),
        Value::ByteArray(bytes) => rc_identity_bits(bytes),
        Value::Set(items) => rc_identity_bits(items),
        Value::FrozenSet(items) => rc_plain_identity_bits(items),
        Value::Range { identity, .. } => rc_plain_identity_bits(identity),
        Value::Dict(entries) => rc_identity_bits(entries),
        Value::OrderedDict(entries) => rc_identity_bits(entries),
        Value::DefaultDict { entries, .. } => rc_identity_bits(entries),
        Value::Counter { entries } => rc_identity_bits(entries),
        Value::ScopeDict(scope) => scope_identity_bits(scope),
        Value::FrameLocalsProxy { identity, .. } => rc_plain_identity_bits(identity),
        Value::DictView { identity, .. } => rc_plain_identity_bits(identity),
        Value::MappingView { identity, .. } => rc_plain_identity_bits(identity),
        Value::MappingProxy { identity, .. } => rc_plain_identity_bits(identity),
        Value::MappingProxyObject { identity, .. } => rc_plain_identity_bits(identity),
        Value::ChainMap { maps } => maps.as_ptr() as usize as u64,
        Value::UserDict { data, .. } => rc_identity_bits(data),
        Value::SimpleNamespace { fields } => rc_identity_bits(fields),
        Value::TypesCoroutineFunction { identity, .. } => rc_plain_identity_bits(identity),
        Value::WeakRef { identity, .. } => rc_plain_identity_bits(identity),
        Value::Generator(state) => rc_identity_bits(state),
        Value::GeneratorWrapper { identity, .. } => rc_plain_identity_bits(identity),
        Value::Coroutine(state) => rc_identity_bits(state),
        Value::CoroutineAwait(state) => rc_identity_bits(state),
        Value::AsyncGenerator(state) => rc_identity_bits(state),
        Value::AsyncGeneratorNext { identity, .. } => rc_plain_identity_bits(identity),
        Value::AsyncGeneratorThrow { state, .. } => rc_identity_bits(state),
        Value::AsyncGeneratorClose(state) => rc_identity_bits(state),
        Value::AsyncGeneratorAthrowMixin { done, .. }
        | Value::AsyncGeneratorAcloseMixin { done, .. } => rc_plain_identity_bits(done),
        Value::Iterator(value) => rc_identity_bits(value),
        Value::Class { attrs, .. } => scope_identity_bits(attrs),
        Value::NamedTupleFieldDescriptor { typ, index } => {
            let mut hasher = DefaultHasher::new();
            "namedtuple_field_descriptor".hash(&mut hasher);
            rc_plain_identity_bits(&typ.identity).hash(&mut hasher);
            index.hash(&mut hasher);
            hasher.finish()
        }
        Value::Instance { fields, .. } => scope_identity_bits(fields),
        Value::Exception { identity, .. } => rc_plain_identity_bits(identity),
        Value::Module { attrs, .. } => scope_identity_bits(attrs),
        Value::Function { identity, .. }
        | Value::Property { identity, .. }
        | Value::StaticMethod { identity, .. }
        | Value::ClassMethod { identity, .. }
        | Value::NamedTupleTypeMethod { identity, .. }
        | Value::NamedTupleInstanceMethod { identity, .. } => rc_plain_identity_bits(identity),
        Value::CodeObject { identity, .. }
        | Value::AstNode { identity, .. }
        | Value::Slice { identity, .. }
        | Value::MemberDescriptor { identity, .. }
        | Value::Super { identity, .. }
        | Value::BoundMethod { identity, .. }
        | Value::MagicMock { identity, .. }
        | Value::MockMethod { identity, .. }
        | Value::WeakProxy { identity, .. }
        | Value::Partial { identity, .. }
        | Value::LruCacheWrapper { identity, .. } => rc_plain_identity_bits(identity),
        Value::Cell { identity, .. } => rc_plain_identity_bits(identity),
        Value::TypeParam { identity, .. } => rc_plain_identity_bits(identity),
        Value::ParamSpecAccess { identity, .. } => rc_plain_identity_bits(identity),
        Value::DeferredTypeParamExpr(deferred) => rc_plain_identity_bits(&deferred.identity),
        Value::NewType { identity, .. } => rc_plain_identity_bits(identity),
        Value::Frame { fields } => rc_identity_bits(fields),
        Value::Traceback { identity, .. } => rc_plain_identity_bits(identity),
        Value::Float(value) => rc_plain_identity_bits(value),
        Value::Complex { identity, .. } => rc_plain_identity_bits(identity),
        Value::MemoryView(view) => rc_identity_bits(view),
        Value::Re(object) => object.identity() as u64,
        Value::BytesIO(bytes_io) => rc_identity_bits(bytes_io),
        Value::StringIOBuffer(buffer) => rc_identity_bits(buffer),
        Value::PartialMethod { identity, .. }
        | Value::PartialMethodCall { identity, .. }
        | Value::RecursiveReprDecorator { identity, .. }
        | Value::RecursiveReprWrapper { identity, .. }
        | Value::SingleDispatch { identity, .. }
        | Value::SingleDispatchRegister { identity, .. }
        | Value::SingleDispatchMethod { identity, .. }
        | Value::SingleDispatchMethodCallable { identity, .. }
        | Value::CachedProperty { identity, .. }
        | Value::CmpToKey { identity, .. }
        | Value::CmpToKeyObject { identity, .. } => rc_plain_identity_bits(identity),
        Value::OperatorAttrGetter { identity, .. }
        | Value::OperatorItemGetter { identity, .. }
        | Value::OperatorMethodCaller { identity, .. } => rc_plain_identity_bits(identity),
        _ => stable_identity_bits(value),
    }
}

fn stable_identity_bits(value: &Value) -> u64 {
    let mut hasher = DefaultHasher::new();
    "id".hash(&mut hasher);
    match value {
        Value::ListIterator { .. }
        | Value::TupleIterator { .. }
        | Value::TemplateIterator { .. }
        | Value::ArrayIterator { .. }
        | Value::SetIterator { .. }
        | Value::DictIterator { .. }
        | Value::ReverseIterator { .. }
        | Value::DictReverseIterator { .. }
        | Value::EnumerateIterator { .. }
        | Value::ZipIterator { .. }
        | Value::MapIterator { .. }
        | Value::FilterIterator { .. }
        | Value::CallIterator { .. }
        | Value::SequenceIterator { .. }
        | Value::SequenceReverseIterator { .. }
        | Value::ItertoolsCount { .. }
        | Value::ItertoolsRepeat { .. }
        | Value::ItertoolsCycle { .. }
        | Value::ItertoolsChain { .. }
        | Value::ItertoolsChainFromIterable { .. }
        | Value::ItertoolsAccumulate { .. }
        | Value::ItertoolsCompress { .. }
        | Value::ItertoolsFilterFalse { .. }
        | Value::ItertoolsTakewhile { .. }
        | Value::ItertoolsDropwhile { .. }
        | Value::ItertoolsStarmap { .. }
        | Value::ItertoolsZipLongest { .. }
        | Value::ItertoolsIslice { .. }
        | Value::ItertoolsPairwise { .. }
        | Value::ItertoolsProduct { .. }
        | Value::ItertoolsCombinations { .. }
        | Value::ItertoolsCombinationsWithReplacement { .. }
        | Value::ItertoolsPermutations { .. }
        | Value::ItertoolsTee { .. }
        | Value::ItertoolsBatched { .. }
        | Value::ItertoolsGroup { .. } => std::mem::discriminant(value).hash(&mut hasher),
        _ => format!("{value:?}").hash(&mut hasher),
    }
    hasher.finish()
}

pub(super) fn scope_identity_bits(scope: &Scope) -> u64 {
    Rc::as_ptr(scope) as usize as u64
}

pub(super) fn rc_identity_bits<T>(value: &Rc<RefCell<T>>) -> u64 {
    Rc::as_ptr(value) as usize as u64
}

pub(super) fn rc_plain_identity_bits<T>(value: &Rc<T>) -> u64 {
    Rc::as_ptr(value) as usize as u64
}

fn empty_bytes_identity_bits() -> u64 {
    let mut hasher = DefaultHasher::new();
    "empty-bytes-singleton".hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    #[test]
    fn unpack_copies_have_new_identity() {
        let source = r#"
import copy
v = next(iter(tuple[int]))
print(copy.copy(v) is v, copy.deepcopy(v) is v)
a = copy.deepcopy([v, v])
print(a[0] is a[1], a[0] is v)
"#;
        assert_eq!(
            crate::run_source(source),
            Ok(vec!["False False".into(), "True False".into()])
        );
    }

    #[test]
    fn callable_iterator_text() {
        let source = r#"
import re
for item in (iter(lambda: 1, 0), re.finditer('a', 'a')):
    print(str(item), repr(item), format(item))
"#;
        assert_eq!(crate::run_source(source), Ok(vec!["<callable_iterator object> <callable_iterator object> <callable_iterator object>".into(); 2]));
    }

    #[test]
    fn anext_default_identity_and_bits() {
        use super::{identity_bits, is_identical};
        use crate::rc::Rc;
        use crate::value::Value;

        let value = Value::AnextDefault {
            identity: Rc::new(()),
            awaitable: Box::new(Value::None),
            default: Box::new(Value::None),
        };
        let cloned = value.clone();
        let other = Value::AnextDefault {
            identity: Rc::new(()),
            awaitable: Box::new(Value::None),
            default: Box::new(Value::None),
        };
        assert!(is_identical(&value, &cloned));
        assert_eq!(identity_bits(&value), identity_bits(&cloned));
        assert!(!is_identical(&value, &other));
        assert_ne!(identity_bits(&value), identity_bits(&other));
    }

    #[test]
    fn empty_immutable_constructors_preserve_their_identity_rules() {
        let source = r#"
a = tuple()
b = tuple([])
c = tuple(())
print(a is b, b is c, tuple('') is a)
a = frozenset()
b = frozenset([])
c = frozenset(())
print(a is b, b is c, frozenset('') is a)
print(a is a, a == b, a == c, frozenset(a) is a)
"#;
        assert_eq!(
            crate::run_source(source),
            Ok(vec![
                "True True True".to_string(),
                "False False False".to_string(),
                "True True True True".to_string(),
            ])
        );
    }

    #[test]
    fn descriptor_identity_is_distinct_from_its_wrapped_function() {
        let source = r#"
def f():
    pass
for constructor in (property, staticmethod, classmethod):
    a = constructor(f)
    b = constructor(f)
    saved = a
    print(a is saved, a == saved, a != saved, a is b, a == b, a != b, a == f, a != f)
"#;
        assert_eq!(
            crate::run_source(source),
            Ok(vec![
                "True True False False False True False True"
                    .to_string();
                3
            ])
        );
    }

    #[test]
    fn namedtuple_bound_method_aliases_keep_their_identity() {
        let source = r#"
from collections import namedtuple
Point = namedtuple('Point', 'x')
p = Point(4)
for receiver, name in ((Point, '_make'), (p, '_replace'), (p, '_asdict'),
                       (p, 'count'), (p, 'index')):
    a = getattr(receiver, name)
    b = getattr(receiver, name)
    saved = a
    print(a is saved, a is not saved, a is b, a is not b, id(a) == id(saved))
"#;
        assert_eq!(
            crate::run_source(source),
            Ok(vec!["True False False True True".to_string(); 5])
        );
    }

    #[test]
    fn alias_identity_survives_changes_to_its_origin() {
        let source = r#"
from types import GenericAlias
class Origin:
    pass
alias = GenericAlias(Origin, (int,))
saved = alias
before = id(alias)
Origin.changed = [1]
print(alias is saved, id(alias) == before, id(saved) == before)
another = GenericAlias(Origin, (int,))
print(alias == another, alias is another, id(alias) == id(another))
"#;
        assert_eq!(
            crate::run_source(source),
            Ok(vec![
                "True True True".to_string(),
                "True False False".to_string(),
            ])
        );
    }

    #[test]
    fn object_comparison_uses_only_the_operands_equality_method() {
        let source = r#"
calls = []
class FalseEq:
    def __eq__(self, other):
        calls.append('eq')
        return False
x = FalseEq()
print(object.__ne__(x, x), object.__ne__(x, FalseEq()), calls)
class NoneEq:
    def __eq__(self, other):
        return None
x = NoneEq()
print(object.__ne__(x, x), object.__ne__(x, NoneEq()))
class Result:
    def __bool__(self):
        raise ValueError('truth tested')
class ResultEq:
    def __eq__(self, other):
        return Result()
x = ResultEq()
try:
    object.__ne__(x, x)
except ValueError as error:
    print(str(error))
calls = []
class Declining:
    def __eq__(self, other):
        calls.append('left')
        return NotImplemented
class Reflected:
    def __eq__(self, other):
        calls.append('right')
        return True
x = Declining()
print(object.__eq__(x, x), object.__eq__(x, Reflected()) is NotImplemented)
print(object.__ne__(x, Reflected()) is NotImplemented, calls)
from collections import UserString
print(object.__ne__('abé', UserString('abé')) is NotImplemented)
print(object.__ne__(UserString('abé'), 1))
"#;
        assert_eq!(
            crate::run_source(source),
            Ok(vec![
                "True True ['eq', 'eq']".to_string(),
                "True True".to_string(),
                "truth tested".to_string(),
                "True True".to_string(),
                "True ['left']".to_string(),
                "True".to_string(),
                "True".to_string(),
            ])
        );
    }

    #[test]
    fn equality_returns_method_results_without_truth_testing() {
        let source = r#"
import operator
class Result:
    def __bool__(self):
        raise ValueError('unexpected truth test')
result = Result()
class Equal:
    def __eq__(self, other):
        return result
    def __ne__(self, other):
        return result
x = Equal()
print((x == x) is result, (x != x) is result)
print(operator.eq(x, x) is result, operator.ne(x, x) is result)
class Empty:
    def __eq__(self, other):
        return None
print(Empty() == 0)
"#;
        assert_eq!(
            crate::run_source(source),
            Ok(vec![
                "True True".to_string(),
                "True True".to_string(),
                "None".to_string(),
            ])
        );
    }

    #[test]
    fn reflected_comparison_preserves_results_and_decline_order() {
        let source = r#"
calls = []
class Result:
    def __bool__(self):
        raise ValueError('unexpected truth test')
result = Result()
class Base:
    def __eq__(self, other):
        calls.append('base eq')
        return NotImplemented
    def __ne__(self, other):
        calls.append('base ne')
        return NotImplemented
class Child(Base):
    def __eq__(self, other):
        calls.append('child eq')
        return result
    def __ne__(self, other):
        calls.append('child ne')
        return result
a = Base()
b = Child()
print((a == b) is result, (a != b) is result, calls)
calls.clear()
class Decline:
    def __eq__(self, other):
        calls.append('eq')
        return NotImplemented
    def __ne__(self, other):
        calls.append('ne')
        return NotImplemented
x = Decline()
print(x == x, x != x, calls)
"#;
        assert_eq!(
            crate::run_source(source),
            Ok(vec![
                "True True ['child eq', 'child ne']".to_string(),
                "True False ['eq', 'eq', 'ne', 'ne']".to_string(),
            ])
        );
    }

    #[test]
    fn copied_aliases_keep_equality_and_get_new_identity() {
        let source = r#"
import copy
x = list[int]
y = x
z = list[int]
print(x is y, x is z, x == z)
for copier in (copy.copy, copy.deepcopy):
    copied = copier(x)
    print(copied == x, copied is x)
x = list[int] | int
for copier in (copy.copy, copy.deepcopy):
    copied = copier(x)
    print(copied == x, copied is x)
"#;
        assert_eq!(
            crate::run_source(source),
            Ok(vec![
                "True False True".to_string(),
                "True False".to_string(),
                "True False".to_string(),
                "True False".to_string(),
                "True False".to_string(),
            ])
        );
    }
}

#[cfg(test)]
mod iterator_identity_tests {
    use super::*;
    #[test]
    fn stable_iterator_identity_does_not_format_recursive_values() {
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                let items = Rc::new(RefCell::new(Vec::new()));
                items.borrow_mut().push(Value::List(items.clone()));
                let value = Value::ListIterator {
                    items: items.clone(),
                    index: 0,
                    exhausted: false,
                };
                let _ = stable_identity_bits(&value);
                items.borrow_mut().clear();
            })
            .unwrap()
            .join()
            .unwrap();
    }
    #[test]
    fn evaluator_identity_tracks_target_and_kind() {
        use crate::value::ConstEvaluatorKind;
        let target = Value::TypeParam {
            kind: "TypeVar".into(),
            name: "T".into(),
            bound: Rc::new(RefCell::new(None)),
            default: Rc::new(RefCell::new(None)),
            infer_variance: false,
            covariant: false,
            contravariant: false,
            identity: Rc::new(()),
        };
        let kinds = [
            ConstEvaluatorKind::TypeAliasValue,
            ConstEvaluatorKind::TypeParamBound,
            ConstEvaluatorKind::TypeParamConstraints,
            ConstEvaluatorKind::TypeParamDefault,
        ];
        let values: Vec<_> = kinds
            .into_iter()
            .map(|kind| Value::ConstEvaluator {
                kind,
                target: Box::new(target.clone()),
            })
            .collect();
        for (i, value) in values.iter().enumerate() {
            let copy = value.clone();
            assert!(is_identical(value, &copy));
            assert_eq!(identity_bits(value), identity_bits(&copy));
            assert_ne!(identity_bits(value), identity_bits(&target));
            for other in &values[..i] {
                assert!(!is_identical(value, other));
                assert_ne!(identity_bits(value), identity_bits(other));
            }
        }
    }
}
