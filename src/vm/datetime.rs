// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Lib/_pydatetime.py (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! Native `datetime` module.
//!
//! The module exposes real classes (`date`, `time`, `datetime`,
//! `timedelta`, `tzinfo`, `timezone`) whose methods are Rust builtins named
//! `datetime.<class>.<method>`. Instances are ordinary instances whose value
//! lives in one hidden field, so user subclasses (including `tzinfo`
//! subclasses written in Python) work the way they do in CPython 3.14.
//!
//! Every reading of the wall clock and every local-time-zone decision goes
//! through `crate::host::clock` (see `Vm::dt_now_micros` and
//! `Vm::dt_local_offset`).

use super::*;
use crate::host::clock as host_clock;

mod calendar;
mod date;
mod delta;
mod format;
mod iso;
mod moment;
mod seq;
mod strptime;
mod timeofday;
mod tz;

use calendar::*;

/// Prefix of every native `datetime` builtin.
pub(crate) const DATETIME_PREFIX: &str = "datetime.";
/// Hidden instance field holding an object's value.
pub(crate) const DATETIME_STATE: &str = "\0datetime_state";
/// Hidden class attribute naming the native class an object derives from.
const CLASS_KIND: &str = "\0datetime_kind";
/// Hidden class attribute holding every class of the module.
const REGISTRY: &str = "\0datetime_registry";

const MODULE_DOC: &str = "Date, time, duration and time zone types.";

const MODULE_ALL: &[&str] = &[
    "date",
    "datetime",
    "time",
    "timedelta",
    "timezone",
    "tzinfo",
    "MINYEAR",
    "MAXYEAR",
    "UTC",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Delta,
    Date,
    Time,
    DateTime,
    TzInfo,
    TimeZone,
    IsoCal,
    StructTime,
}

impl Kind {
    const ALL: [Kind; 8] = [
        Kind::Delta,
        Kind::Date,
        Kind::Time,
        Kind::DateTime,
        Kind::TzInfo,
        Kind::TimeZone,
        Kind::IsoCal,
        Kind::StructTime,
    ];

    fn code(self) -> i64 {
        Kind::ALL
            .iter()
            .position(|kind| *kind == self)
            .expect("every kind is listed") as i64
    }

    fn from_code(code: i64) -> Option<Kind> {
        usize::try_from(code)
            .ok()
            .and_then(|index| Kind::ALL.get(index).copied())
    }

    /// The class name used in builtin names and `__name__`.
    pub(super) fn class_name(self) -> &'static str {
        match self {
            Kind::Delta => "timedelta",
            Kind::Date => "date",
            Kind::Time => "time",
            Kind::DateTime => "datetime",
            Kind::TzInfo => "tzinfo",
            Kind::TimeZone => "timezone",
            Kind::IsoCal => "IsoCalendarDate",
            Kind::StructTime => "struct_time",
        }
    }

    /// The qualified type name (`datetime.date`, ...) error messages show.
    pub(super) fn tp_name(self) -> &'static str {
        match self {
            Kind::Delta => "datetime.timedelta",
            Kind::Date => "datetime.date",
            Kind::Time => "datetime.time",
            Kind::DateTime => "datetime.datetime",
            Kind::TzInfo => "datetime.tzinfo",
            Kind::TimeZone => "datetime.timezone",
            Kind::IsoCal => "datetime.IsoCalendarDate",
            Kind::StructTime => "time.struct_time",
        }
    }

    fn from_class_name(name: &str) -> Option<Kind> {
        Kind::ALL
            .iter()
            .copied()
            .find(|kind| kind.class_name() == name)
    }

    pub(super) fn is_date(self) -> bool {
        matches!(self, Kind::Date | Kind::DateTime)
    }

    pub(super) fn is_tzinfo(self) -> bool {
        matches!(self, Kind::TzInfo | Kind::TimeZone)
    }
}

/// A normalized `timedelta`: `0 <= seconds < 86400`, `0 <= us < 10**6`,
/// `|days| <= 999999999`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Delta {
    pub days: i64,
    pub seconds: i64,
    pub us: i64,
}

impl Delta {
    pub const ZERO: Delta = Delta {
        days: 0,
        seconds: 0,
        us: 0,
    };

    pub fn total_us(self) -> i128 {
        (i128::from(self.days) * SECONDS_PER_DAY + i128::from(self.seconds)) * US_PER_SECOND
            + i128::from(self.us)
    }

    /// Normalizes a microsecond count, raising CPython's `OverflowError`s.
    pub fn from_us(total: i128) -> Result<Delta, String> {
        let days = total.div_euclid(US_PER_DAY);
        let rest = total.rem_euclid(US_PER_DAY);
        if days < i128::from(i32::MIN) || days > i128::from(i32::MAX) {
            return Err("OverflowError: Python int too large to convert to C int".to_string());
        }
        if days.abs() > MAX_DELTA_DAYS {
            return Err(format!(
                "OverflowError: days={days}; must have magnitude <= {MAX_DELTA_DAYS}"
            ));
        }
        Ok(Delta {
            days: days as i64,
            seconds: (rest / US_PER_SECOND) as i64,
            us: (rest % US_PER_SECOND) as i64,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Ymd {
    pub year: i64,
    pub month: i64,
    pub day: i64,
}

impl Ymd {
    pub fn ordinal(self) -> i64 {
        ymd_to_ordinal(self.year, self.month, self.day)
    }

    pub fn from_ordinal(ordinal: i64) -> Ymd {
        let (year, month, day) = ordinal_to_ymd(ordinal);
        Ymd { year, month, day }
    }
}

/// A time of day with its `tzinfo` (`Value::None` when naive) and `fold`.
#[derive(Clone, Debug)]
pub(super) struct Hms {
    pub hour: i64,
    pub minute: i64,
    pub second: i64,
    pub us: i64,
    pub tz: Value,
    pub fold: i64,
}

impl Hms {
    pub fn midnight() -> Hms {
        Hms {
            hour: 0,
            minute: 0,
            second: 0,
            us: 0,
            tz: Value::None,
            fold: 0,
        }
    }

    /// Microseconds since midnight.
    pub fn day_us(&self) -> i128 {
        (i128::from(self.hour * 3600 + self.minute * 60 + self.second)) * US_PER_SECOND
            + i128::from(self.us)
    }

    pub fn with_day_us(&self, day_us: i128) -> Hms {
        let seconds = (day_us / US_PER_SECOND) as i64;
        Hms {
            hour: seconds / 3600,
            minute: seconds / 60 % 60,
            second: seconds % 60,
            us: (day_us % US_PER_SECOND) as i64,
            tz: self.tz.clone(),
            fold: self.fold,
        }
    }

    pub fn is_naive_tz(&self) -> bool {
        matches!(self.tz, Value::None)
    }
}

/// The decoded value of a datetime-family object.
#[derive(Clone, Debug)]
pub(super) enum State {
    Delta(Delta),
    Date(Ymd),
    Time(Hms),
    DateTime(Ymd, Hms),
    TimeZone(Delta, Option<String>),
}

fn num(value: i64) -> Value {
    Value::Number(value)
}

/// Hidden instance field holding an aware value's `tzinfo`.
const DATETIME_TZ: &str = "\0datetime_tz";
/// Hidden instance field holding a `timezone`'s name.
const DATETIME_NAME: &str = "\0datetime_name";

/// Stores a value in an instance's fields. The integers are packed into
/// one small `bytes` value so the VM's allocation budget charges each
/// datetime object only a few dozen bytes.
fn store_state(fields: &mut HashMap<String, Value>, state: &State) {
    let mut packed = Vec::with_capacity(72);
    let mut push = |value: i64| packed.extend_from_slice(&value.to_le_bytes());
    let (code, tz, name) = match state {
        State::Delta(delta) => {
            push(delta.days);
            push(delta.seconds);
            push(delta.us);
            (Kind::Delta, Value::None, None)
        }
        State::Date(ymd) => {
            push(ymd.year);
            push(ymd.month);
            push(ymd.day);
            (Kind::Date, Value::None, None)
        }
        State::Time(hms) => {
            for value in [hms.hour, hms.minute, hms.second, hms.us, hms.fold] {
                push(value);
            }
            (Kind::Time, hms.tz.clone(), None)
        }
        State::DateTime(ymd, hms) => {
            for value in [
                ymd.year, ymd.month, ymd.day, hms.hour, hms.minute, hms.second, hms.us, hms.fold,
            ] {
                push(value);
            }
            (Kind::DateTime, hms.tz.clone(), None)
        }
        State::TimeZone(offset, name) => {
            push(offset.days);
            push(offset.seconds);
            push(offset.us);
            (Kind::TimeZone, Value::None, name.clone())
        }
    };
    packed.insert(0, code.code() as u8);
    fields.insert(DATETIME_STATE.to_string(), bytes_value(packed));
    if matches!(tz, Value::None) {
        fields.remove(DATETIME_TZ);
    } else {
        fields.insert(DATETIME_TZ.to_string(), tz);
    }
    match name {
        Some(name) => {
            fields.insert(DATETIME_NAME.to_string(), Value::String(name));
        }
        None => {
            fields.remove(DATETIME_NAME);
        }
    }
}

fn load_state(fields: &HashMap<String, Value>) -> Option<State> {
    let Value::Bytes(packed) = fields.get(DATETIME_STATE)? else {
        return None;
    };
    let (&code, rest) = packed.split_first()?;
    let int = |index: usize| -> Option<i64> {
        let chunk = rest.get(index * 8..index * 8 + 8)?;
        Some(i64::from_le_bytes(chunk.try_into().ok()?))
    };
    let tz = || fields.get(DATETIME_TZ).cloned().unwrap_or(Value::None);
    let kind = Kind::from_code(i64::from(code))?;
    Some(match kind {
        Kind::Delta => State::Delta(Delta {
            days: int(0)?,
            seconds: int(1)?,
            us: int(2)?,
        }),
        Kind::Date => State::Date(Ymd {
            year: int(0)?,
            month: int(1)?,
            day: int(2)?,
        }),
        Kind::Time => State::Time(Hms {
            hour: int(0)?,
            minute: int(1)?,
            second: int(2)?,
            us: int(3)?,
            tz: tz(),
            fold: int(4)?,
        }),
        Kind::DateTime => State::DateTime(
            Ymd {
                year: int(0)?,
                month: int(1)?,
                day: int(2)?,
            },
            Hms {
                hour: int(3)?,
                minute: int(4)?,
                second: int(5)?,
                us: int(6)?,
                tz: tz(),
                fold: int(7)?,
            },
        ),
        Kind::TimeZone => State::TimeZone(
            Delta {
                days: int(0)?,
                seconds: int(1)?,
                us: int(2)?,
            },
            match fields.get(DATETIME_NAME) {
                Some(Value::String(name)) => Some(name.clone()),
                _ => None,
            },
        ),
        Kind::TzInfo | Kind::IsoCal | Kind::StructTime => return None,
    })
}

/// The decoded value stored in a datetime-family instance.
pub(super) fn state_of(value: &Value) -> Option<State> {
    let Value::Instance { fields, .. } = value else {
        return None;
    };
    load_state(&fields.borrow())
}

/// Replaces the value stored in an instance (used on fresh objects only).
pub(super) fn set_state(value: &Value, state: &State) {
    if let Value::Instance { fields, .. } = value {
        store_state(&mut fields.borrow_mut(), state);
    }
}

/// The native class kind an instance's class derives from.
pub(super) fn kind_of(value: &Value) -> Option<Kind> {
    let Value::Instance {
        class_attrs,
        class_bases,
        ..
    } = value
    else {
        return None;
    };
    class_kind(class_attrs, class_bases)
}

fn class_kind(attrs: &Scope, bases: &[Value]) -> Option<Kind> {
    match find_class_attr(attrs, bases, CLASS_KIND)? {
        Value::Number(code) => Kind::from_code(code),
        _ => None,
    }
}

/// The native class kind of a class value (`date`, a subclass of it, ...).
pub(super) fn kind_of_class(class: &Value) -> Option<Kind> {
    let Value::Class { attrs, bases, .. } = class else {
        return None;
    };
    class_kind(attrs, bases)
}

/// Whether the class is exactly one of this module's native classes.
pub(super) fn class_is_native(class: &Value) -> bool {
    matches!(class, Value::Class { attrs, .. } if attrs.borrow().contains_key(CLASS_KIND))
}

/// Whether the instance's class is exactly a native class.
pub(super) fn instance_is_native(value: &Value) -> bool {
    matches!(value, Value::Instance { class_attrs, .. } if class_attrs.borrow().contains_key(CLASS_KIND))
}

/// The class of an instance.
pub(super) fn class_of(value: &Value) -> Value {
    match value {
        Value::Instance {
            class_name,
            class_attrs,
            class_bases,
            ..
        } => Value::Class {
            name: class_name.clone(),
            type_params: Vec::new(),
            metaclass: None,
            bases: class_bases.clone(),
            attrs: class_attrs.clone(),
        },
        other => Value::Builtin(type_name(other).to_string()),
    }
}

/// The qualified type name (`datetime.date`, ...) for instances of exactly a
/// native class; the runtime's `type_name` uses it in error messages.
pub(crate) fn native_type_name(class_attrs: &Scope) -> Option<&'static str> {
    match class_attrs.borrow().get(CLASS_KIND) {
        Some(Value::Number(code)) => Kind::from_code(*code).map(Kind::tp_name),
        _ => None,
    }
}

/// The name of a value's type as error messages print it.
pub(super) fn tp_name(value: &Value) -> String {
    if let Value::Instance {
        class_name,
        class_attrs,
        ..
    } = value
    {
        if let Some(Value::Number(code)) = class_attrs.borrow().get(CLASS_KIND)
            && let Some(kind) = Kind::from_code(*code)
        {
            return kind.tp_name().to_string();
        }
        return class_name.clone();
    }
    match value {
        Value::None => "NoneType".to_string(),
        Value::Class { .. } => "type".to_string(),
        other => type_name(other).to_string(),
    }
}

/// The type name argument-type errors print: `None` itself for `None`.
pub(super) fn arg_type_name(value: &Value) -> String {
    match value {
        Value::None => "None".to_string(),
        other => tp_name(other),
    }
}

/// The name `repr` uses for an instance: the native type name, or a
/// subclass's `__name__`.
pub(super) fn repr_type_name(value: &Value) -> String {
    tp_name(value)
}

/// Every class of the module, reachable from any of them.
#[derive(Clone)]
#[allow(dead_code)]
pub(super) struct Registry {
    pub delta: Value,
    pub date: Value,
    pub time: Value,
    pub datetime: Value,
    pub tzinfo: Value,
    pub timezone: Value,
    pub isocal: Value,
    pub struct_time: Value,
    pub utc: Value,
}

fn registry_from_value(value: &Value) -> Option<Registry> {
    let Value::Tuple(items) = value else {
        return None;
    };
    let [
        delta,
        date,
        time,
        datetime,
        tzinfo,
        timezone,
        isocal,
        struct_time,
        utc,
    ] = items.as_slice()
    else {
        return None;
    };
    Some(Registry {
        delta: delta.clone(),
        date: date.clone(),
        time: time.clone(),
        datetime: datetime.clone(),
        tzinfo: tzinfo.clone(),
        timezone: timezone.clone(),
        isocal: isocal.clone(),
        struct_time: struct_time.clone(),
        utc: utc.clone(),
    })
}

/// The registry reachable from a class or an instance of this module.
pub(super) fn registry_of(anchor: &Value) -> Option<Registry> {
    let found = match anchor {
        Value::Instance {
            class_attrs,
            class_bases,
            ..
        } => find_class_attr(class_attrs, class_bases, REGISTRY),
        Value::Class { attrs, bases, .. } => find_class_attr(attrs, bases, REGISTRY),
        _ => None,
    }?;
    registry_from_value(&found)
}

/// A new instance of `class` (a native class or a subclass) holding `state`.
pub(super) fn new_instance(class: &Value, state: &State) -> Value {
    let Value::Class {
        name, attrs, bases, ..
    } = class
    else {
        unreachable!("datetime instances are created from classes");
    };
    let fields = new_scope();
    store_state(&mut fields.borrow_mut(), state);
    Value::Instance {
        class_name: name.clone(),
        fields,
        class_attrs: attrs.clone(),
        class_bases: bases.clone(),
    }
}

/// Whether `builtin` is the native implementation of a datetime special
/// method for `kind` (so a subclass did not override it).
fn is_native_method(found: Option<Value>, kind: Kind, method: &str) -> bool {
    matches!(found, Some(Value::Builtin(name))
    if name.strip_prefix(DATETIME_PREFIX)
        .and_then(|rest| rest.split_once('.'))
        .is_some_and(|(class, found_method)| {
            found_method == method
                && Kind::from_class_name(class).is_some_and(|owner| {
                    owner == kind || (owner == Kind::Date && kind == Kind::DateTime)
                })
        }))
}

// ----------------------------------------------------------------------
// Hooks used outside the VM's call path (containers compare and display
// their items without calling back into Python).
// ----------------------------------------------------------------------

/// Native `repr` of a datetime-family instance for container displays, when
/// its class keeps the native `__repr__`.
pub(crate) fn native_instance_repr(value: &Value) -> Option<String> {
    let kind = kind_of(value)?;
    let Value::Instance {
        class_attrs,
        class_bases,
        ..
    } = value
    else {
        return None;
    };
    if !is_native_method(
        find_class_attr(class_attrs, class_bases, "__repr__"),
        kind,
        "__repr__",
    ) {
        return None;
    }
    match kind {
        Kind::IsoCal | Kind::StructTime => seq::native_repr(kind, value),
        _ => format::repr_state(value, &state_of(value)?, |tz| {
            native_instance_repr(tz).unwrap_or_else(|| tz.to_string())
        }),
    }
}

/// Value equality of two datetime-family instances when it can be decided
/// without running Python code (dict keys, set members, tuple items).
/// `None` means "not decidable natively".
pub(crate) fn native_instances_equal(left: &Value, right: &Value) -> Option<bool> {
    let left_kind = kind_of(left)?;
    let right_kind = kind_of(right)?;
    for (value, kind) in [(left, left_kind), (right, right_kind)] {
        let Value::Instance {
            class_attrs,
            class_bases,
            ..
        } = value
        else {
            return None;
        };
        if !is_native_method(
            find_class_attr(class_attrs, class_bases, "__eq__"),
            kind,
            "__eq__",
        ) {
            return None;
        }
    }
    let left_state = state_of(left)?;
    let right_state = state_of(right)?;
    match (&left_state, &right_state) {
        (State::Delta(a), State::Delta(b)) => Some(a == b),
        (State::Date(a), State::Date(b)) => Some(a == b),
        (State::Date(_), State::DateTime(..)) | (State::DateTime(..), State::Date(_)) => {
            Some(false)
        }
        (State::TimeZone(a, _), State::TimeZone(b, _)) => Some(a == b),
        (State::DateTime(ad, at), State::DateTime(bd, bt)) => {
            let a_offset = native_offset(&at.tz)?;
            let b_offset = native_offset(&bt.tz)?;
            let same_tz = same_object(&at.tz, &bt.tz);
            let a_total = i128::from(ad.ordinal()) * US_PER_DAY + at.day_us();
            let b_total = i128::from(bd.ordinal()) * US_PER_DAY + bt.day_us();
            Some(match (a_offset, b_offset) {
                _ if same_tz => a_total == b_total,
                (None, None) => a_total == b_total,
                (Some(a), Some(b)) => a_total - a.total_us() == b_total - b.total_us(),
                _ => false,
            })
        }
        (State::Time(at), State::Time(bt)) => {
            let a_offset = native_offset(&at.tz)?;
            let b_offset = native_offset(&bt.tz)?;
            let same_tz = same_object(&at.tz, &bt.tz);
            Some(match (a_offset, b_offset) {
                _ if same_tz => at.day_us() == bt.day_us(),
                (None, None) => at.day_us() == bt.day_us(),
                (Some(a), Some(b)) if a == b => at.day_us() == bt.day_us(),
                // Container lookups need equal hashes too, and CPython's
                // time hash keeps the offset's microseconds.
                (Some(a), Some(b)) => {
                    timeofday::time_compare_key(at, a) == timeofday::time_compare_key(bt, b)
                        && at.day_us() - a.total_us() == bt.day_us() - b.total_us()
                }
                _ => false,
            })
        }
        _ => Some(false),
    }
}

/// A hash consistent with `native_instances_equal`, for values nested in
/// hashed containers (tuples, frozensets) when it can be computed without
/// running Python code.
pub(crate) fn native_instance_hash(value: &Value) -> Option<i128> {
    let kind = kind_of(value)?;
    let Value::Instance {
        class_attrs,
        class_bases,
        ..
    } = value
    else {
        return None;
    };
    for method in ["__eq__", "__hash__"] {
        if !is_native_method(
            find_class_attr(class_attrs, class_bases, method),
            kind,
            method,
        ) {
            return None;
        }
    }
    Some(match state_of(value)? {
        State::Delta(delta) => delta.total_us(),
        State::Date(ymd) => i128::from(ymd.ordinal()),
        State::TimeZone(offset, _) => offset.total_us(),
        State::DateTime(ymd, hms) => {
            let total = i128::from(ymd.ordinal()) * US_PER_DAY + hms.day_us();
            match native_offset(&hms.tz)? {
                None => total,
                Some(offset) => total - offset.total_us(),
            }
        }
        State::Time(hms) => match native_offset(&hms.tz)? {
            None => hms.day_us(),
            Some(offset) => hms.day_us() - offset.total_us(),
        },
    })
}

/// The UTC offset of a `tzinfo` that can be read without running Python
/// code: `Some(None)` for naive, `Some(Some(offset))` for `timezone`.
fn native_offset(tz: &Value) -> Option<Option<Delta>> {
    match tz {
        Value::None => Some(None),
        value => match state_of(value)? {
            State::TimeZone(offset, _) => Some(Some(offset)),
            _ => None,
        },
    }
}

/// Object identity for tzinfo values.
pub(super) fn same_object(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Instance { fields: a, .. }, Value::Instance { fields: b, .. }) => Rc::ptr_eq(a, b),
        (Value::None, Value::None) => true,
        _ => false,
    }
}

// ----------------------------------------------------------------------
// Module construction.
// ----------------------------------------------------------------------

/// What a class attribute is.
#[derive(Clone, Copy)]
enum Member {
    /// A method bound to instances.
    Method(&'static str),
    /// A classmethod.
    ClassMethod(&'static str),
    /// A read-only data attribute.
    Getter(&'static str),
}

use Member::{ClassMethod as CM, Getter as G, Method as M};

const RICH_COMPARE: [Member; 6] = [
    M("__eq__"),
    M("__ne__"),
    M("__lt__"),
    M("__le__"),
    M("__gt__"),
    M("__ge__"),
];

struct ClassSpec {
    kind: Kind,
    doc: &'static str,
    members: &'static [Member],
    has_new: bool,
}

const DELTA_MEMBERS: &[Member] = &[
    M("__repr__"),
    M("__str__"),
    M("__hash__"),
    M("__add__"),
    M("__radd__"),
    M("__sub__"),
    M("__rsub__"),
    M("__mul__"),
    M("__rmul__"),
    M("__mod__"),
    M("__rmod__"),
    M("__divmod__"),
    M("__rdivmod__"),
    M("__neg__"),
    M("__pos__"),
    M("__abs__"),
    M("__bool__"),
    M("__floordiv__"),
    M("__rfloordiv__"),
    M("__truediv__"),
    M("__rtruediv__"),
    M("total_seconds"),
    M("__reduce__"),
    M("__reduce_ex__"),
    G("days"),
    G("seconds"),
    G("microseconds"),
];

const DATE_MEMBERS: &[Member] = &[
    M("__repr__"),
    M("__hash__"),
    M("__str__"),
    M("__add__"),
    M("__radd__"),
    M("__sub__"),
    M("__rsub__"),
    CM("fromtimestamp"),
    CM("fromordinal"),
    CM("fromisoformat"),
    CM("fromisocalendar"),
    CM("strptime"),
    CM("today"),
    M("ctime"),
    M("strftime"),
    M("__format__"),
    M("timetuple"),
    M("isocalendar"),
    M("isoformat"),
    M("isoweekday"),
    M("toordinal"),
    M("weekday"),
    M("replace"),
    M("__replace__"),
    M("__reduce__"),
    M("__reduce_ex__"),
    G("year"),
    G("month"),
    G("day"),
];

const TIME_MEMBERS: &[Member] = &[
    M("__repr__"),
    M("__hash__"),
    M("__str__"),
    CM("strptime"),
    M("isoformat"),
    M("strftime"),
    M("__format__"),
    M("utcoffset"),
    M("tzname"),
    M("dst"),
    M("replace"),
    M("__replace__"),
    CM("fromisoformat"),
    M("__reduce_ex__"),
    M("__reduce__"),
    G("hour"),
    G("minute"),
    G("second"),
    G("microsecond"),
    G("tzinfo"),
    G("fold"),
];

const DATETIME_MEMBERS: &[Member] = &[
    M("__repr__"),
    M("__hash__"),
    M("__str__"),
    M("__add__"),
    M("__radd__"),
    M("__sub__"),
    M("__rsub__"),
    CM("now"),
    CM("utcnow"),
    CM("fromtimestamp"),
    CM("utcfromtimestamp"),
    CM("strptime"),
    CM("combine"),
    CM("fromisoformat"),
    M("date"),
    M("time"),
    M("timetz"),
    M("ctime"),
    M("timetuple"),
    M("timestamp"),
    M("utctimetuple"),
    M("isoformat"),
    M("utcoffset"),
    M("tzname"),
    M("dst"),
    M("replace"),
    M("__replace__"),
    M("astimezone"),
    M("__reduce_ex__"),
    M("__reduce__"),
    G("hour"),
    G("minute"),
    G("second"),
    G("microsecond"),
    G("tzinfo"),
    G("fold"),
];

const TZINFO_MEMBERS: &[Member] = &[
    M("tzname"),
    M("utcoffset"),
    M("dst"),
    M("fromutc"),
    M("__reduce__"),
    M("__reduce_ex__"),
];

const TIMEZONE_MEMBERS: &[Member] = &[
    M("__repr__"),
    M("__hash__"),
    M("__str__"),
    M("tzname"),
    M("utcoffset"),
    M("dst"),
    M("fromutc"),
    M("__getinitargs__"),
    CM("__init_subclass__"),
];

const ISOCAL_MEMBERS: &[Member] = &[
    M("__repr__"),
    M("__reduce__"),
    M("__reduce_ex__"),
    G("year"),
    G("week"),
    G("weekday"),
];

const STRUCT_TIME_MEMBERS: &[Member] = &[
    M("__repr__"),
    M("__reduce__"),
    M("__reduce_ex__"),
    G("tm_year"),
    G("tm_mon"),
    G("tm_mday"),
    G("tm_hour"),
    G("tm_min"),
    G("tm_sec"),
    G("tm_wday"),
    G("tm_yday"),
    G("tm_isdst"),
    G("tm_zone"),
    G("tm_gmtoff"),
];

const CLASS_SPECS: &[ClassSpec] = &[
    ClassSpec {
        kind: Kind::Delta,
        doc: "A duration, stored as days, seconds and microseconds.",
        members: DELTA_MEMBERS,
        has_new: true,
    },
    ClassSpec {
        kind: Kind::Date,
        doc: "A calendar date in the proleptic Gregorian calendar.",
        members: DATE_MEMBERS,
        has_new: true,
    },
    ClassSpec {
        kind: Kind::Time,
        doc: "A time of day, optionally with a tzinfo.",
        members: TIME_MEMBERS,
        has_new: true,
    },
    ClassSpec {
        kind: Kind::DateTime,
        doc: "A date combined with a time of day, optionally with a tzinfo.",
        members: DATETIME_MEMBERS,
        has_new: true,
    },
    ClassSpec {
        kind: Kind::TzInfo,
        doc: "Base class for time zone information objects.",
        members: TZINFO_MEMBERS,
        has_new: true,
    },
    ClassSpec {
        kind: Kind::TimeZone,
        doc: "A tzinfo with a fixed UTC offset and an optional name.",
        members: TIMEZONE_MEMBERS,
        has_new: true,
    },
    ClassSpec {
        kind: Kind::IsoCal,
        doc: "The (year, week, weekday) named tuple returned by isocalendar().",
        members: ISOCAL_MEMBERS,
        has_new: true,
    },
    ClassSpec {
        kind: Kind::StructTime,
        doc: "Broken-down time as a named tuple of nine fields, as returned by timetuple().",
        members: STRUCT_TIME_MEMBERS,
        has_new: true,
    },
];

fn builtin_name(kind: Kind, member: &str) -> String {
    format!("{DATETIME_PREFIX}{}.{member}", kind.class_name())
}

fn getter_property(kind: Kind, name: &str) -> Value {
    Value::Property {
        fget: Some(Rc::new(Value::Builtin(builtin_name(kind, name)))),
        fset: Some(Rc::new(Value::Builtin(builtin_name(
            kind,
            &format!("{name}:set"),
        )))),
        fdel: Some(Rc::new(Value::Builtin(builtin_name(
            kind,
            &format!("{name}:del"),
        )))),
        doc: Rc::new(RefCell::new(None)),
        doc_from_getter: false,
        name: Rc::new(RefCell::new(Some(Value::String(name.to_string())))),
        identity: Rc::new(()),
    }
}

/// Builds the `datetime` module value.
pub(crate) fn datetime_module_value() -> Value {
    let mut classes: Vec<(Kind, Value)> = Vec::new();
    for spec in CLASS_SPECS {
        let base = match spec.kind {
            Kind::DateTime => classes
                .iter()
                .find(|(kind, _)| *kind == Kind::Date)
                .map(|(_, class)| class.clone())
                .expect("date is declared before datetime"),
            Kind::TimeZone => classes
                .iter()
                .find(|(kind, _)| *kind == Kind::TzInfo)
                .map(|(_, class)| class.clone())
                .expect("tzinfo is declared before timezone"),
            Kind::IsoCal | Kind::StructTime => builtin_type_value("tuple"),
            _ => builtin_type_value("object"),
        };
        let module = if spec.kind == Kind::StructTime {
            "time"
        } else {
            "datetime"
        };
        let scope = new_scope();
        {
            let mut values = scope.borrow_mut();
            let mut order = Vec::new();
            let mut insert = |key: &str, value: Value| {
                order.push(Value::String(key.to_string()));
                values.insert(key.to_string(), value);
            };
            insert("__module__", Value::String(module.to_string()));
            insert(
                "__qualname__",
                Value::String(spec.kind.class_name().to_string()),
            );
            insert("__doc__", Value::String(spec.doc.to_string()));
            if spec.has_new {
                insert(
                    "__new__",
                    Value::StaticMethod {
                        function: Rc::new(Value::Builtin(builtin_name(spec.kind, "__new__"))),
                        identity: Rc::new(()),
                    },
                );
            }
            for member in spec.members {
                match *member {
                    Member::Method(name) => {
                        insert(name, Value::Builtin(builtin_name(spec.kind, name)));
                    }
                    Member::ClassMethod(name) => insert(
                        name,
                        Value::ClassMethod {
                            function: Rc::new(Value::Builtin(builtin_name(spec.kind, name))),
                            identity: Rc::new(()),
                        },
                    ),
                    Member::Getter(name) => insert(name, getter_property(spec.kind, name)),
                }
            }
            if matches!(
                spec.kind,
                Kind::Delta | Kind::Date | Kind::Time | Kind::DateTime | Kind::TimeZone
            ) {
                for member in RICH_COMPARE {
                    if let Member::Method(name) = member {
                        insert(name, Value::Builtin(builtin_name(spec.kind, name)));
                    }
                }
            }
            // Instances of the native classes have no `__dict__`.
            values.insert("__slots__".to_string(), tuple_value(Vec::new()));
            values.insert(
                CLASS_QUALNAME_ATTR.to_string(),
                Value::String(spec.kind.class_name().to_string()),
            );
            values.insert(CLASS_KIND.to_string(), Value::Number(spec.kind.code()));
            values.insert(CLASS_ATTR_ORDER_ATTR.to_string(), tuple_value(order));
        }
        let class = Value::Class {
            name: spec.kind.class_name().to_string(),
            type_params: Vec::new(),
            metaclass: None,
            bases: vec![base],
            attrs: scope,
        };
        classes.push((spec.kind, class));
    }
    let class = |kind: Kind| {
        classes
            .iter()
            .find(|(found, _)| *found == kind)
            .map(|(_, class)| class.clone())
            .expect("every class is built")
    };
    let utc = new_instance(&class(Kind::TimeZone), &State::TimeZone(Delta::ZERO, None));
    let registry = tuple_value(vec![
        class(Kind::Delta),
        class(Kind::Date),
        class(Kind::Time),
        class(Kind::DateTime),
        class(Kind::TzInfo),
        class(Kind::TimeZone),
        class(Kind::IsoCal),
        class(Kind::StructTime),
        utc.clone(),
    ]);
    for (_, class) in &classes {
        if let Value::Class { attrs, .. } = class {
            attrs
                .borrow_mut()
                .insert(REGISTRY.to_string(), registry.clone());
        }
    }

    // Class constants are instances of the class itself.
    let set = |kind: Kind, name: &str, value: Value| {
        if let Value::Class { attrs, .. } = class(kind) {
            store_ordered_scope_item(&attrs, name.to_string(), value);
        }
    };
    let delta = |days: i64, seconds: i64, us: i64| {
        new_instance(
            &class(Kind::Delta),
            &State::Delta(Delta { days, seconds, us }),
        )
    };
    set(Kind::Delta, "min", delta(-999_999_999, 0, 0));
    set(Kind::Delta, "max", delta(999_999_999, 86_399, 999_999));
    set(Kind::Delta, "resolution", delta(0, 0, 1));
    let ymd = |year, month, day| Ymd { year, month, day };
    let hms = |hour, minute, second, us| Hms {
        hour,
        minute,
        second,
        us,
        tz: Value::None,
        fold: 0,
    };
    set(
        Kind::Date,
        "min",
        new_instance(&class(Kind::Date), &State::Date(ymd(1, 1, 1))),
    );
    set(
        Kind::Date,
        "max",
        new_instance(&class(Kind::Date), &State::Date(ymd(9999, 12, 31))),
    );
    set(Kind::Date, "resolution", delta(1, 0, 0));
    set(
        Kind::Time,
        "min",
        new_instance(&class(Kind::Time), &State::Time(hms(0, 0, 0, 0))),
    );
    set(
        Kind::Time,
        "max",
        new_instance(&class(Kind::Time), &State::Time(hms(23, 59, 59, 999_999))),
    );
    set(Kind::Time, "resolution", delta(0, 0, 1));
    set(
        Kind::DateTime,
        "min",
        new_instance(
            &class(Kind::DateTime),
            &State::DateTime(ymd(1, 1, 1), hms(0, 0, 0, 0)),
        ),
    );
    set(
        Kind::DateTime,
        "max",
        new_instance(
            &class(Kind::DateTime),
            &State::DateTime(ymd(9999, 12, 31), hms(23, 59, 59, 999_999)),
        ),
    );
    set(Kind::DateTime, "resolution", delta(0, 0, 1));
    let max_offset = Delta {
        days: 0,
        seconds: 86_340,
        us: 0,
    };
    set(Kind::TimeZone, "utc", utc.clone());
    set(
        Kind::TimeZone,
        "min",
        new_instance(
            &class(Kind::TimeZone),
            &State::TimeZone(
                Delta::from_us(-max_offset.total_us()).expect("in range"),
                None,
            ),
        ),
    );
    set(
        Kind::TimeZone,
        "max",
        new_instance(&class(Kind::TimeZone), &State::TimeZone(max_offset, None)),
    );
    if let Value::Class { attrs, .. } = class(Kind::StructTime) {
        for (name, value) in [
            ("n_sequence_fields", 9),
            ("n_fields", 11),
            ("n_unnamed_fields", 0),
        ] {
            store_ordered_scope_item(&attrs, name.to_string(), Value::Number(value));
        }
        store_ordered_scope_item(
            &attrs,
            "__match_args__".to_string(),
            tuple_value(
                [
                    "tm_year", "tm_mon", "tm_mday", "tm_hour", "tm_min", "tm_sec", "tm_wday",
                    "tm_yday", "tm_isdst",
                ]
                .iter()
                .map(|name| Value::String((*name).to_string()))
                .collect(),
            ),
        );
    }

    let scope = new_scope();
    {
        let mut values = scope.borrow_mut();
        let mut insert = |key: &str, value: Value| {
            values.insert(key.to_string(), value);
        };
        insert("__name__", Value::String("datetime".to_string()));
        insert("__doc__", Value::String(MODULE_DOC.to_string()));
        insert("__package__", Value::String(String::new()));
        insert(
            "__all__",
            tuple_value(
                MODULE_ALL
                    .iter()
                    .map(|name| Value::String((*name).to_string()))
                    .collect(),
            ),
        );
        insert("MINYEAR", Value::Number(MINYEAR));
        insert("MAXYEAR", Value::Number(MAXYEAR));
        insert("UTC", utc);
        for kind in [
            Kind::Date,
            Kind::DateTime,
            Kind::Time,
            Kind::Delta,
            Kind::TimeZone,
            Kind::TzInfo,
        ] {
            insert(kind.class_name(), class(kind));
        }
    }
    Value::Module {
        name: "datetime".to_string(),
        attrs: scope,
    }
}

/// Whether a builtin name belongs to this module.
pub(crate) fn is_datetime_builtin(name: &str) -> bool {
    name.starts_with(DATETIME_PREFIX)
}

/// Whether a builtin of this module is a plain function attribute that
/// binds to instances (methods and property accessors do; `__new__` and
/// classmethods are wrapped already).
pub(crate) fn datetime_builtin_binds(name: &str) -> bool {
    is_datetime_builtin(name) && !name.ends_with(".__new__")
}

// ----------------------------------------------------------------------
// Argument handling with CPython's messages.
// ----------------------------------------------------------------------

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

/// A signature of named parameters, some of them positional-only by count.
pub(super) struct ArgSpec<'a> {
    /// `None` reports as "function"/"this function".
    pub fname: Option<&'a str>,
    pub params: &'a [&'a str],
    pub required: usize,
    /// Parameters that may be passed positionally.
    pub positional: usize,
}

impl ArgSpec<'_> {
    fn label(&self) -> String {
        match self.fname {
            Some(name) => format!("{name}()"),
            None => "function".to_string(),
        }
    }
}

impl Vm {
    /// Binds positional and keyword arguments, converting each
    /// present argument in order with `convert` so conversion errors and
    /// missing-argument errors surface in CPython's order.
    pub(super) fn dt_parse_args(
        &mut self,
        spec: &ArgSpec<'_>,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
        convert: &mut dyn FnMut(&mut Vm, usize, Value) -> Result<Value, String>,
    ) -> Result<Vec<Option<Value>>, String> {
        let total = spec.params.len();
        let nargs = args.len();
        let mut nkwargs = keywords.len();
        if nargs + nkwargs > total {
            return Err(format!(
                "TypeError: {} takes at most {total} {}argument{} ({} given)",
                spec.label(),
                if nargs == 0 { "keyword " } else { "" },
                plural(total),
                nargs + nkwargs
            ));
        }
        if nargs > spec.positional {
            return Err(format!(
                "TypeError: {} takes at most {} positional argument{} ({nargs} given)",
                spec.label(),
                spec.positional,
                plural(spec.positional)
            ));
        }
        let mut result = vec![None; total];
        let mut args = args.into_iter();
        let mut consumed = vec![false; keywords.len()];
        for (index, name) in spec.params.iter().enumerate() {
            let current = if index < nargs {
                args.next()
            } else if nkwargs > 0 {
                keywords
                    .iter()
                    .position(|(key, _)| key == name)
                    .map(|position| {
                        consumed[position] = true;
                        nkwargs -= 1;
                        keywords[position].1.clone()
                    })
            } else {
                None
            };
            if let Some(value) = current {
                result[index] = Some(convert(self, index, value)?);
                continue;
            }
            if index < spec.required {
                return Err(format!(
                    "TypeError: {} missing required argument '{name}' (pos {})",
                    spec.label(),
                    index + 1
                ));
            }
            if nkwargs == 0 {
                return Ok(result);
            }
        }
        if nkwargs > 0 {
            for (index, name) in spec.params.iter().enumerate().take(nargs) {
                if keywords.iter().any(|(key, _)| key == name) {
                    return Err(format!(
                        "TypeError: argument for {} given by name ('{name}') and position ({})",
                        spec.label(),
                        index + 1
                    ));
                }
            }
            for ((key, _), used) in keywords.iter().zip(consumed) {
                if used || spec.params.contains(&key.as_str()) {
                    continue;
                }
                let suggestion = super::argparse::suggest_keyword(key, spec.params)
                    .map(|name| format!(". Did you mean '{name}'?"))
                    .unwrap_or_default();
                return Err(format!(
                    "TypeError: {} got an unexpected keyword argument '{key}'{suggestion}",
                    match spec.fname {
                        Some(name) => format!("{name}()"),
                        None => "this function".to_string(),
                    }
                ));
            }
        }
        Ok(result)
    }

    /// Exactly `count` positional arguments and no keywords.
    pub(super) fn dt_exact_args(
        &mut self,
        fname: &str,
        count: usize,
        args: &[Value],
        keywords: &[(String, Value)],
    ) -> Result<(), String> {
        if !keywords.is_empty() {
            return Err(format!("TypeError: {fname}() takes no keyword arguments"));
        }
        if args.len() != count {
            return Err(format!(
                "TypeError: {fname}() takes exactly {count} argument{} ({} given)",
                plural(count),
                args.len()
            ));
        }
        Ok(())
    }

    /// `METH_NOARGS` checks.
    pub(super) fn dt_no_args(
        &mut self,
        qualname: &str,
        args: &[Value],
        keywords: &[(String, Value)],
    ) -> Result<(), String> {
        if !keywords.is_empty() {
            return Err(format!(
                "TypeError: {qualname}() takes no keyword arguments"
            ));
        }
        if !args.is_empty() {
            return Err(format!(
                "TypeError: {qualname}() takes no arguments ({} given)",
                args.len()
            ));
        }
        Ok(())
    }

    /// `METH_O` checks.
    pub(super) fn dt_one_arg(
        &mut self,
        qualname: &str,
        args: Vec<Value>,
        keywords: &[(String, Value)],
    ) -> Result<Value, String> {
        if !keywords.is_empty() {
            return Err(format!(
                "TypeError: {qualname}() takes no keyword arguments"
            ));
        }
        if args.len() != 1 {
            return Err(format!(
                "TypeError: {qualname}() takes exactly one argument ({} given)",
                args.len()
            ));
        }
        Ok(args.into_iter().next().expect("one argument"))
    }

    /// Slot wrapper checks (`__add__`, `__eq__`, ...).
    pub(super) fn dt_slot_args(
        &mut self,
        method: &str,
        count: usize,
        args: Vec<Value>,
        keywords: &[(String, Value)],
    ) -> Result<Vec<Value>, String> {
        if !keywords.is_empty() {
            return Err(format!(
                "TypeError: wrapper {method}() takes no keyword arguments"
            ));
        }
        if args.len() != count {
            return Err(format!(
                "TypeError: expected {count} argument{}, got {}",
                plural(count),
                args.len()
            ));
        }
        Ok(args)
    }

    /// An integer argument that must fit a 32-bit C `int`.
    pub(super) fn dt_c_int(&mut self, value: &Value) -> Result<i64, String> {
        let wide = self.dt_index(value)?;
        if wide > i64::from(i32::MAX) {
            return Err("OverflowError: signed integer is greater than maximum".to_string());
        }
        if wide < i64::from(i32::MIN) {
            return Err("OverflowError: signed integer is less than minimum".to_string());
        }
        Ok(wide)
    }

    /// An integer through `__index__`, as a C `long`.
    pub(super) fn dt_index(&mut self, value: &Value) -> Result<i64, String> {
        match self.dt_index_big(value)? {
            Some(big) => big.to_i64().ok_or_else(|| {
                "OverflowError: Python int too large to convert to C long".to_string()
            }),
            None => Err(format!(
                "TypeError: '{}' object cannot be interpreted as an integer",
                tp_name(value)
            )),
        }
    }

    /// The integer value of an int-like object, or `None` when it has no
    /// `__index__`.
    pub(super) fn dt_index_big(&mut self, value: &Value) -> Result<Option<BigInt>, String> {
        match value {
            Value::Bool(flag) => Ok(Some(BigInt::from(i64::from(*flag)))),
            Value::Number(number) => Ok(Some(BigInt::from(*number))),
            Value::BigInt(number) => Ok(Some(number.clone())),
            other => {
                if let Some(inner) = int_subclass_integer(other) {
                    return self.dt_index_big(&inner);
                }
                if matches!(other, Value::Instance { .. })
                    && let Some(method) = instance_special_method(other, "__index__")
                {
                    let result = self.call_value(method, Vec::new())?;
                    return match &result {
                        Value::Bool(_) | Value::Number(_) | Value::BigInt(_) => {
                            self.dt_index_big(&result)
                        }
                        _ if int_subclass_integer(&result).is_some() => self.dt_index_big(&result),
                        _ => Err(format!(
                            "TypeError: __index__ returned non-int (type {})",
                            tp_name(&result)
                        )),
                    };
                }
                Ok(None)
            }
        }
    }

    /// A `str` argument, with CPython's message for other types.
    pub(super) fn dt_str_arg(
        &mut self,
        value: &Value,
        fname: &str,
        position: usize,
    ) -> Result<String, String> {
        match value {
            Value::String(text) | Value::IdentityString { value: text, .. } => Ok(text.clone()),
            other => match str_subclass_string(other) {
                Some(text) => Ok(text),
                None => Err(format!(
                    "TypeError: {fname}() argument {position} must be str, not {}",
                    arg_type_name(other)
                )),
            },
        }
    }

    /// Checks a `tzinfo` argument: `None` or a `tzinfo` instance.
    pub(super) fn dt_check_tzinfo(&mut self, value: &Value) -> Result<(), String> {
        if matches!(value, Value::None) || kind_of(value).is_some_and(Kind::is_tzinfo) {
            return Ok(());
        }
        Err(format!(
            "TypeError: tzinfo argument must be None or of a tzinfo subclass, not type '{}'",
            tp_name(value)
        ))
    }

    /// Raises an exception message as an error.
    pub(super) fn dt_error<T>(
        &mut self,
        kind: &str,
        message: impl Into<String>,
    ) -> Result<T, String> {
        Err(format!("{kind}: {}", message.into()))
    }

    /// The registry of the module that owns `anchor`.
    pub(super) fn dt_registry(&mut self, anchor: &Value) -> Result<Registry, String> {
        if let Some(registry) = registry_of(anchor) {
            return Ok(registry);
        }
        let module = match lookup_string_key(&self.module_cache, "datetime") {
            Some(module @ Value::Module { .. }) => module,
            _ => self.load_stdlib_module("datetime")?,
        };
        let date = self.load_attribute_value(module, "date")?;
        registry_of(&date).ok_or_else(|| "TypeError: datetime module is unavailable".to_string())
    }

    // ------------------------------------------------------------------
    // Host clock (the only place the wall clock and local zone are read).
    // ------------------------------------------------------------------

    /// The current time in microseconds since the Unix epoch.
    pub(super) fn dt_now_micros(&self) -> i64 {
        self.runtime_options.clock.now_unix_micros()
    }

    /// Offset of local time from UTC at a Unix time, in seconds.
    pub(super) fn dt_local_offset(&self, unix_seconds: i64) -> i64 {
        host_clock::local_utc_offset_seconds(unix_seconds)
    }

    // ------------------------------------------------------------------
    // Dispatch.
    // ------------------------------------------------------------------

    /// Entry point for every `datetime.*` builtin. Captured exceptions keep
    /// their objects until the calling frame dispatches.
    pub(super) fn call_datetime_builtin(
        &mut self,
        name: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let name = name.to_string();
        match self.call_with_exception_capture(|vm| vm.datetime_dispatch(&name, args, keywords))? {
            Ok(value) => Ok(value),
            Err(exception) => Err(self.propagate_captured_exception(exception)),
        }
    }

    fn datetime_dispatch(
        &mut self,
        name: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let rest = name.strip_prefix(DATETIME_PREFIX).unwrap_or(name);
        let Some((class, method)) = rest.split_once('.') else {
            return Err(format!("AttributeError: {name}"));
        };
        let Some(kind) = Kind::from_class_name(class) else {
            return Err(format!("AttributeError: {name}"));
        };
        if let Some((attr, action)) = method.split_once(':') {
            return self.dt_readonly_attribute(kind, attr, action, args);
        }
        if method == "__reduce_ex__"
            && matches!(
                kind,
                Kind::Delta | Kind::Date | Kind::TzInfo | Kind::IsoCal | Kind::StructTime
            )
        {
            // `object.__reduce_ex__`: the class's `__reduce__`.
            let mut args = args;
            let accepted: &[Kind] = match kind {
                Kind::Date => &[Kind::Date, Kind::DateTime],
                Kind::TzInfo => &[Kind::TzInfo, Kind::TimeZone],
                other => std::slice::from_ref(match other {
                    Kind::Delta => &Kind::Delta,
                    Kind::IsoCal => &Kind::IsoCal,
                    _ => &Kind::StructTime,
                }),
            };
            let receiver = self.dt_receiver(&mut args, kind, method, accepted)?;
            self.dt_exact_args("__reduce_ex__", 1, &args, &keywords)?;
            self.dt_c_int(&args[0])?;
            let reduce = self.load_attribute_value(receiver, "__reduce__")?;
            return self.call_value(reduce, Vec::new());
        }
        if method == "__init_subclass__" {
            return self.dt_error(
                "TypeError",
                format!("type '{}' is not an acceptable base type", kind.tp_name()),
            );
        }
        match kind {
            Kind::Delta => self.dt_delta_method(method, args, keywords),
            Kind::Date => self.dt_date_method(method, args, keywords),
            Kind::Time => self.dt_time_method(method, args, keywords),
            Kind::DateTime => self.dt_datetime_method(method, args, keywords),
            Kind::TzInfo | Kind::TimeZone => self.dt_tz_method(kind, method, args, keywords),
            Kind::IsoCal | Kind::StructTime => self.dt_seq_method(kind, method, args, keywords),
        }
    }

    fn dt_readonly_attribute(
        &mut self,
        kind: Kind,
        attr: &str,
        action: &str,
        args: Vec<Value>,
    ) -> Result<Value, String> {
        let owner = args
            .first()
            .map(tp_name)
            .unwrap_or_else(|| kind.tp_name().to_string());
        let _ = action;
        let owner = if kind_of(args.first().unwrap_or(&Value::None)).is_some() {
            kind.tp_name().to_string()
        } else {
            owner
        };
        Err(format!(
            "AttributeError: attribute '{attr}' of '{owner}' objects is not writable"
        ))
    }

    /// The receiver of a method: the first argument, which must be of one
    /// of `accepted` kinds.
    pub(super) fn dt_receiver(
        &mut self,
        args: &mut Vec<Value>,
        owner: Kind,
        method: &str,
        accepted: &[Kind],
    ) -> Result<Value, String> {
        if args.is_empty() {
            return Err(format!(
                "TypeError: unbound method {}.{method}() needs an argument",
                owner.class_name()
            ));
        }
        let receiver = args.remove(0);
        match kind_of(&receiver) {
            Some(kind) if accepted.contains(&kind) => Ok(receiver),
            _ => Err(format!(
                "TypeError: descriptor '{method}' for '{}' objects doesn't apply to a '{}' object",
                owner.tp_name(),
                tp_name(&receiver)
            )),
        }
    }

    /// The decoded state of a receiver already checked by `dt_receiver`.
    pub(super) fn dt_state(&mut self, value: &Value) -> Result<State, String> {
        state_of(value)
            .ok_or_else(|| format!("TypeError: '{}' object is not initialized", tp_name(value)))
    }

    /// The class argument of a classmethod.
    pub(super) fn dt_class_arg(
        &mut self,
        args: &mut Vec<Value>,
        owner: Kind,
        method: &str,
    ) -> Result<Value, String> {
        if args.is_empty() {
            return Err(format!(
                "TypeError: descriptor '{method}' of '{}' object needs an argument",
                owner.tp_name()
            ));
        }
        let class = args.remove(0);
        if kind_of_class(&class).is_none() {
            return Err(format!(
                "TypeError: descriptor '{method}' for type '{}' doesn't apply to type '{}'",
                owner.tp_name(),
                tp_name(&class)
            ));
        }
        Ok(class)
    }

    // ------------------------------------------------------------------
    // Object construction.
    // ------------------------------------------------------------------

    pub(super) fn dt_new_delta(&mut self, anchor: &Value, delta: Delta) -> Result<Value, String> {
        let registry = self.dt_registry(anchor)?;
        Ok(new_instance(&registry.delta, &State::Delta(delta)))
    }

    pub(super) fn dt_delta_from_us(
        &mut self,
        anchor: &Value,
        total: i128,
    ) -> Result<Value, String> {
        let delta = Delta::from_us(total)?;
        self.dt_new_delta(anchor, delta)
    }

    /// A new date: a native instance, or `cls(year, month, day)`
    /// for subclasses.
    pub(super) fn dt_make_date(&mut self, class: &Value, ymd: Ymd) -> Result<Value, String> {
        if class_is_native(class) {
            check_date(ymd.year, ymd.month, ymd.day)?;
            return Ok(new_instance(class, &State::Date(ymd)));
        }
        self.call_value(
            class.clone(),
            vec![num(ymd.year), num(ymd.month), num(ymd.day)],
        )
    }

    /// A new datetime: a native instance, or
    /// `cls(y, m, d, hh, mm, ss, us, tzinfo[, fold=1])` for subclasses.
    pub(super) fn dt_make_datetime(
        &mut self,
        class: &Value,
        ymd: Ymd,
        hms: Hms,
    ) -> Result<Value, String> {
        if class_is_native(class) {
            check_date(ymd.year, ymd.month, ymd.day)?;
            return Ok(new_instance(class, &State::DateTime(ymd, hms)));
        }
        let args = vec![
            num(ymd.year),
            num(ymd.month),
            num(ymd.day),
            num(hms.hour),
            num(hms.minute),
            num(hms.second),
            num(hms.us),
            hms.tz.clone(),
        ];
        let keywords = if hms.fold != 0 {
            vec![("fold".to_string(), num(hms.fold))]
        } else {
            Vec::new()
        };
        self.call_value_with_keywords(class.clone(), args, keywords)
    }

    /// A time (native, or `cls(h, m, s, us, tzinfo[, fold=1])`).
    pub(super) fn dt_make_time(&mut self, class: &Value, hms: Hms) -> Result<Value, String> {
        if class_is_native(class) {
            return Ok(new_instance(class, &State::Time(hms)));
        }
        let args = vec![
            num(hms.hour),
            num(hms.minute),
            num(hms.second),
            num(hms.us),
            hms.tz.clone(),
        ];
        let keywords = if hms.fold != 0 {
            vec![("fold".to_string(), num(hms.fold))]
        } else {
            Vec::new()
        };
        self.call_value_with_keywords(class.clone(), args, keywords)
    }

    /// `timezone(offset[, name])` for a known-good offset; UTC with no name
    /// is the `timezone.utc` singleton.
    pub(super) fn dt_make_timezone(
        &mut self,
        anchor: &Value,
        offset: Delta,
        name: Option<String>,
    ) -> Result<Value, String> {
        let registry = self.dt_registry(anchor)?;
        if offset == Delta::ZERO && name.is_none() {
            return Ok(registry.utc);
        }
        Ok(new_instance(
            &registry.timezone,
            &State::TimeZone(offset, name),
        ))
    }

    /// A Python int from an `i128`.
    pub(super) fn dt_int(value: i128) -> Value {
        match i64::try_from(value) {
            Ok(small) => Value::Number(small),
            Err(_) => Value::BigInt(BigInt::from(value)),
        }
    }

    /// `copy.copy`/`copy.deepcopy` of a datetime-family instance through the
    /// reduce protocol, as CPython's `copy` module does (so `timezone.utc`
    /// stays a singleton). `None` when the value is not one, or its class
    /// defines `__copy__`/`__deepcopy__`.
    pub(super) fn dt_copy_by_reduce(
        &mut self,
        value: &Value,
        deep: Option<(&Value, &mut HashMap<usize, Value>)>,
    ) -> Result<Option<Value>, String> {
        let hook = if deep.is_some() {
            "__deepcopy__"
        } else {
            "__copy__"
        };
        let Value::Instance {
            class_attrs,
            class_bases,
            ..
        } = value
        else {
            return Ok(None);
        };
        if kind_of(value).is_none() || find_class_attr(class_attrs, class_bases, hook).is_some() {
            return Ok(None);
        }
        let reduce_ex = self.load_attribute_value(value.clone(), "__reduce_ex__")?;
        let reduced = self.call_value(reduce_ex, vec![Value::Number(4)])?;
        let Value::Tuple(parts) = reduced else {
            return Ok(Some(value.clone()));
        };
        let (Some(function), Some(args)) = (parts.first().cloned(), parts.get(1).cloned()) else {
            return Ok(None);
        };
        let mut args = args;
        let mut state = parts.get(2).cloned().unwrap_or(Value::None);
        if let Some((memo_value, memo)) = deep {
            // The memo is keyed by addresses: keep the transient reduce
            // tuple alive (under a key no allocation can have) so its parts
            // are not mistaken for later objects at the same address.
            memo.insert(
                Rc::as_ptr(&parts) as *const u8 as usize | 1,
                Value::Tuple(parts.clone()),
            );
            let mut hook = |item: &Value, memo: &mut HashMap<usize, Value>| {
                self.call_deepcopy_hook(item, memo_value, memo)
            };
            args = deep_copy_value_with_hook(&args, memo, &mut hook)?;
            if !matches!(state, Value::None) {
                state = deep_copy_value_with_hook(&state, memo, &mut hook)?;
            }
        }
        let args = self.collect_iterable_values(args)?;
        let result = self.call_value(function, args)?;
        let (dict_state, slot_state) = match state {
            Value::Tuple(items) if items.len() == 2 => (items[0].clone(), items[1].clone()),
            other => (other, Value::None),
        };
        for part in [dict_state, slot_state] {
            if let Value::Dict(entries) = part {
                let entries = entries.borrow().entries.clone();
                for (key, item) in entries {
                    let name = self.str_value(&key)?;
                    self.store_attribute_value(result.clone(), &name, item)?;
                }
            }
        }
        Ok(Some(result))
    }

    /// Emits a `DeprecationWarning` the way the runtime surfaces warnings.
    pub(super) fn dt_deprecation(&mut self, message: &str) -> Result<(), String> {
        self.emit_deprecation_warning(message.to_string())
    }
}
