// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Objects/complexobject.c (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! Complex multiplication and division.

use super::super::*;

pub(crate) fn complex_divide_values(
    left_real: f64,
    left_imag: f64,
    right_real: f64,
    right_imag: f64,
) -> Result<Value, String> {
    if right_real == 0.0 && right_imag == 0.0 {
        return Err("complex division by zero".to_string());
    }

    let abs_right_real = if right_real < 0.0 {
        -right_real
    } else {
        right_real
    };
    let abs_right_imag = if right_imag < 0.0 {
        -right_imag
    } else {
        right_imag
    };

    let (mut real, mut imag) = if abs_right_real >= abs_right_imag {
        let ratio = right_imag / right_real;
        let denom = right_real + right_imag * ratio;
        (
            (left_real + left_imag * ratio) / denom,
            (left_imag - left_real * ratio) / denom,
        )
    } else if abs_right_imag >= abs_right_real {
        let ratio = right_real / right_imag;
        let denom = right_real * ratio + right_imag;
        (
            (left_real * ratio + left_imag) / denom,
            (left_imag * ratio - left_real) / denom,
        )
    } else {
        (f64::NAN, f64::NAN)
    };

    if real.is_nan() && imag.is_nan() {
        if (left_real.is_infinite() || left_imag.is_infinite())
            && right_real.is_finite()
            && right_imag.is_finite()
        {
            let x = if left_real.is_infinite() {
                1.0_f64
            } else {
                0.0_f64
            }
            .copysign(left_real);
            let y = if left_imag.is_infinite() {
                1.0_f64
            } else {
                0.0_f64
            }
            .copysign(left_imag);
            real = f64::INFINITY * (x * right_real + y * right_imag);
            imag = f64::INFINITY * (y * right_real - x * right_imag);
        } else if (abs_right_real.is_infinite() || abs_right_imag.is_infinite())
            && left_real.is_finite()
            && left_imag.is_finite()
        {
            let x = if right_real.is_infinite() {
                1.0_f64
            } else {
                0.0_f64
            }
            .copysign(right_real);
            let y = if right_imag.is_infinite() {
                1.0_f64
            } else {
                0.0_f64
            }
            .copysign(right_imag);
            real = 0.0 * (left_real * x + left_imag * y);
            imag = 0.0 * (left_imag * x - left_real * y);
        }
    }

    Ok(complex_value(real, imag))
}

pub(crate) fn complex_divide_by_real_value(
    left_real: f64,
    left_imag: f64,
    right: f64,
) -> Result<Value, String> {
    if right == 0.0 {
        return Err("complex division by zero".to_string());
    }
    Ok(complex_value(left_real / right, left_imag / right))
}

pub(crate) fn real_divide_by_complex_value(
    left: f64,
    right_real: f64,
    right_imag: f64,
) -> Result<Value, String> {
    if right_real == 0.0 && right_imag == 0.0 {
        return Err("complex division by zero".to_string());
    }

    let abs_right_real = if right_real < 0.0 {
        -right_real
    } else {
        right_real
    };
    let abs_right_imag = if right_imag < 0.0 {
        -right_imag
    } else {
        right_imag
    };

    let (mut real, mut imag) = if abs_right_real >= abs_right_imag {
        let ratio = right_imag / right_real;
        let denom = right_real + right_imag * ratio;
        (left / denom, (-left * ratio) / denom)
    } else if abs_right_imag >= abs_right_real {
        let ratio = right_real / right_imag;
        let denom = right_real * ratio + right_imag;
        ((left * ratio) / denom, (-left) / denom)
    } else {
        (f64::NAN, f64::NAN)
    };

    if real.is_nan()
        && imag.is_nan()
        && left.is_finite()
        && (abs_right_real.is_infinite() || abs_right_imag.is_infinite())
    {
        let x = if right_real.is_infinite() {
            1.0_f64
        } else {
            0.0_f64
        }
        .copysign(right_real);
        let y = if right_imag.is_infinite() {
            1.0_f64
        } else {
            0.0_f64
        }
        .copysign(right_imag);
        real = 0.0 * (left * x);
        imag = 0.0 * (-left * y);
    }

    Ok(complex_value(real, imag))
}

pub(crate) fn multiply_complex_parts(
    left_real: f64,
    left_imag: f64,
    right_real: f64,
    right_imag: f64,
) -> (f64, f64) {
    let mut a = left_real;
    let mut b = left_imag;
    let mut c = right_real;
    let mut d = right_imag;
    let ac = a * c;
    let bd = b * d;
    let ad = a * d;
    let bc = b * c;
    let mut real = ac - bd;
    let mut imag = ad + bc;

    if real.is_nan() && imag.is_nan() {
        let mut recalc = false;

        if a.is_infinite() || b.is_infinite() {
            a = if a.is_infinite() { 1.0_f64 } else { 0.0_f64 }.copysign(a);
            b = if b.is_infinite() { 1.0_f64 } else { 0.0_f64 }.copysign(b);
            if c.is_nan() {
                c = 0.0_f64.copysign(c);
            }
            if d.is_nan() {
                d = 0.0_f64.copysign(d);
            }
            recalc = true;
        }
        if c.is_infinite() || d.is_infinite() {
            c = if c.is_infinite() { 1.0_f64 } else { 0.0_f64 }.copysign(c);
            d = if d.is_infinite() { 1.0_f64 } else { 0.0_f64 }.copysign(d);
            if a.is_nan() {
                a = 0.0_f64.copysign(a);
            }
            if b.is_nan() {
                b = 0.0_f64.copysign(b);
            }
            recalc = true;
        }
        if !recalc && (ac.is_infinite() || bd.is_infinite() || ad.is_infinite() || bc.is_infinite())
        {
            if a.is_nan() {
                a = 0.0_f64.copysign(a);
            }
            if b.is_nan() {
                b = 0.0_f64.copysign(b);
            }
            if c.is_nan() {
                c = 0.0_f64.copysign(c);
            }
            if d.is_nan() {
                d = 0.0_f64.copysign(d);
            }
            recalc = true;
        }
        if recalc {
            real = f64::INFINITY * (a * c - b * d);
            imag = f64::INFINITY * (a * d + b * c);
        }
    }

    (real, imag)
}
