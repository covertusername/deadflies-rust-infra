// the types here are never going to change
#![allow(clippy::cast_lossless)]

use crate::{primitives::IChar, structs::ArcRw};
use std::{sync::Arc, ptr::addr_eq};

#[must_use]
pub fn u16_slice_as_ichars(s: &[u16]) -> Vec<IChar> {
    let mut i = 0;
    let mut ret: Vec<IChar> = Vec::new();

    while i < s.len() {
        if IChar::Surrogate(s[i] as u32).is_leading_surrogate() {
            if i < s.len()
                && IChar::Surrogate(s[i + 1] as u32).is_trailing_surrogate() {
                    ret.push(IChar::from_surrogate_pair(s[i], s[i + 1]));
                    i += 2;
                    continue;
                }
            ret.push(IChar::Surrogate(s[i] as u32));
            i += 1;
            continue;
        } else if IChar::Surrogate(s[i] as u32).is_trailing_surrogate() {
            ret.push(IChar::Surrogate(s[i] as u32));
            i += 1;
            continue;
        }
        // safe because all remaining values are valid for char
        ret.push(IChar::Scalar(unsafe {
            char::from_u32_unchecked(s[i] as u32)
        }));
        i += 1;
    }
    ret
}

#[must_use]
pub fn ichars_from_str(s: &str) -> Vec<IChar> {
    let mut ret: Vec<IChar> = Vec::new();

    for c in s.chars() {
        ret.push(IChar::Scalar(c));
    }
    ret
}

#[must_use]
pub fn ichars_to_string(chars: &[IChar]) -> String {
    let mut ret = String::new();

    for c in chars {
        match c {
            IChar::Surrogate(_) => ret.push('\u{FFFD}'),
            IChar::Scalar(c) => ret.push(*c),
        }
    }
    ret
}

#[must_use]
pub fn is_valid_utf16(v: &Vec<u16>) -> bool {
    for c in u16_slice_as_ichars(v.as_slice()) {
        match c {
            IChar::Surrogate(_) => return false,
            IChar::Scalar(_) => ()
        }
    }
    true
}

#[must_use]
// if we panic this is a bug
#[allow(clippy::missing_panics_doc)]
pub fn is_eq_arcslice<T>(a: &[ArcRw<T>], b: &[ArcRw<T>]) -> bool
where
    T: PartialEq
{
    // i am who i am
    for (i, am) in a.iter().enumerate() {
        if !addr_eq(Arc::as_ptr(am), Arc::as_ptr(&b[i])) && *am.read_arc() != *b[i].read_arc() {
            return false
        }
    }
    true
}

#[must_use]
// if we panic this is a bug
#[allow(clippy::missing_panics_doc)]
pub fn unwrap_arcslice<T>(slice: &[ArcRw<T>]) -> Vec<T>
where
    T: Clone,
{
    let mut ret = Vec::new();

    for item in slice {
        ret.push(item.read_arc().clone());
    }
    ret
}
