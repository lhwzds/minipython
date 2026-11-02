// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Modules/mathmodule.c, Objects/exceptions.c, Objects/unicodeobject.c, Objects/complexobject.c (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! VM code translated from CPython.
//!
//! The modules below hold the parts of the VM that follow CPython's own
//! algorithms: `math.fsum` and its exact-partials helpers, complex
//! multiplication and division, the `ExceptionGroup` splitting and `except*`
//! machinery, and the Unicode decimal digits.

pub(crate) mod complex_arith;
pub(crate) mod exception_group;
pub(crate) mod fsum;
pub(crate) mod unicode_digits;
