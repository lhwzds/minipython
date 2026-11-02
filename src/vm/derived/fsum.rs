// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Modules/mathmodule.c (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! `math.fsum` and the exact-partials helpers that the float path of
//! `math.sumprod` shares with it.

use super::super::*;

pub(crate) fn call_math_fsum(vm: &mut Vm, args: Vec<Value>) -> Result<Value, String> {
    let [iterable] = args.as_slice() else {
        return Err(format!(
            "TypeError: math.fsum() takes exactly one argument ({} given)",
            args.len()
        ));
    };

    let values = vm.collect_iterable_values_propagating(iterable.clone())?;
    let mut partials = Vec::new();
    let mut saw_nan = false;
    let mut saw_positive_inf = false;
    let mut saw_negative_inf = false;

    for value in values {
        let value = math_real_number_as_f64(vm, value)?;
        if value.is_nan() {
            saw_nan = true;
            continue;
        }
        if value.is_infinite() {
            if value.is_sign_negative() {
                saw_negative_inf = true;
            } else {
                saw_positive_inf = true;
            }
            continue;
        }
        math_fsum_add_partial(&mut partials, value)?;
    }

    if saw_positive_inf && saw_negative_inf {
        return Err("ValueError: -inf + inf in fsum".to_string());
    }
    if saw_nan {
        return Ok(float_value(f64::NAN));
    }
    if saw_positive_inf {
        return Ok(float_value(f64::INFINITY));
    }
    if saw_negative_inf {
        return Ok(float_value(f64::NEG_INFINITY));
    }

    let mut result = math_fsum_finish(partials)?;
    if result == 0.0 {
        result = 0.0;
    }
    Ok(float_value(result))
}

pub(crate) fn math_fsum_add_partial(partials: &mut Vec<f64>, mut x: f64) -> Result<(), String> {
    let mut next = 0;
    for index in 0..partials.len() {
        let mut y = partials[index];
        if x.abs() < y.abs() {
            std::mem::swap(&mut x, &mut y);
        }

        let hi = x + y;
        if x.is_finite() && y.is_finite() && hi.is_infinite() {
            return Err("OverflowError: intermediate overflow in fsum".to_string());
        }
        let yr = hi - x;
        let lo = y - yr;
        if lo != 0.0 {
            partials[next] = lo;
            next += 1;
        }
        x = hi;
    }
    partials.truncate(next);
    if x != 0.0 {
        partials.push(x);
    }
    Ok(())
}

pub(crate) fn math_fsum_finish(mut partials: Vec<f64>) -> Result<f64, String> {
    let Some(mut hi) = partials.pop() else {
        return Ok(0.0);
    };

    let mut lo = 0.0;
    while let Some(x) = partials.pop() {
        let next_hi = hi + x;
        if hi.is_finite() && x.is_finite() && next_hi.is_infinite() {
            return Err("OverflowError: intermediate overflow in fsum".to_string());
        }
        let yr = next_hi - hi;
        lo = x - yr;
        hi = next_hi;
        if lo != 0.0 {
            break;
        }
    }

    if lo != 0.0 {
        if let Some(next) = partials.last()
            && ((lo < 0.0 && *next < 0.0) || (lo > 0.0 && *next > 0.0))
        {
            let y = lo * 2.0;
            let next_hi = hi + y;
            if y == next_hi - hi {
                hi = next_hi;
            }
        }
    }

    Ok(hi)
}
