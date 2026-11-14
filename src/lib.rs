#![warn(clippy::pedantic)]
// violates my personal style (i learned C first)
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::cast_lossless)] // the types here are never going to change
//pub mod js_base;
pub mod json;
pub mod structs;
pub mod util;

pub mod primitives {
    use crate::{
        /*js_base::{self, JSpecType, JType, Number, parse_text},*/ structs::{List, ListCore, Map, Queue, Set, Stack, Struct}, util::{ichars_from_str, is_valid_utf16, u16_slice_as_ichars}
    };
    use std::{
        char::decode_utf16, fmt, ops::RangeBounds, slice, str::FromStr, string::String, time::{Duration, Instant}
    };
    use widestring::{U16String, Utf16String};

    #[derive(Debug, Copy, Clone)]
    pub struct Byte(pub u8);

    impl Byte {
        #[must_use]
        pub fn tolower(&self) -> Self {
            if self.0 >= 0x41 && self.0 <= 0x5A {
                return Self(self.0 + 0x20);
            }
            *self
        }
    }

    impl fmt::Display for Byte {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{:#X}", self.0)
        }
    }

    /*
     * standard specifies that this is a sequence of Bytes, but easier to
     * implement as a private Vec of u8 and convert as needed
     */

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct Byteseq {
        elems: Vec<u8>,
    }

    impl FromStr for Byteseq {
        type Err = !;
    
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let mut ret = Self::new();
            let bytes = s.bytes();

            for b in bytes {
                ret.elems.push(b);
            }
            Ok(ret)
        }
    }

    impl Byteseq {
        #[must_use]
        pub fn new() -> Self {
            Self { elems: Vec::new() }
        }

        #[must_use]
        pub fn as_bytes(&self) -> Vec<Byte> {
            let mut bvec: Vec<Byte> = Vec::new();

            for b in &self.elems {
                bvec.push(Byte(*b));
            }
            bvec
        }

        #[must_use]
        pub fn len(&self) -> usize {
            self.elems.len()
        }

        #[must_use]
        pub fn is_empty(&self) -> bool {
            self.len() == 0
        }

        #[must_use]
        pub fn blower(&mut self) -> &mut Self {
            let mut i = 0;

            while i < self.elems.len() {
                if self.elems[i] >= 0x41 && self.elems[i] <= 0x5A {
                    self.elems[i] += 0x20;
                }
                i += 1;
            }
            self
        }

        #[must_use]
        pub fn bupper(&mut self) -> &mut Self {
            let mut i = 0;

            while i < self.elems.len() {
                if self.elems[i] >= 0x61 && self.elems[i] <= 0x7A {
                    self.elems[i] -= 0x20;
                }
                i += 1;
            }
            self
        }

        #[must_use]
        pub fn starts_with(&self, prefix: &Self) -> bool {
            let mut i = 0;
            let self_bytes = self.as_bytes();
            let input_bytes = prefix.as_bytes();

            loop {
                if i >= prefix.len() {
                    return true;
                }
                if i >= self.len() {
                    return false;
                }
                if self_bytes[i].0 != input_bytes[i].0 {
                    return false;
                }
                i += 1;
            }
        }

        /*
         * note: the way this is specified in the standard implies it only
         * works correctly with byte sequences that are also ASCII strings. it
         * will split apart UTF-8 bytes and sign-extend them to 16-bit, which
         * doesn't produce a readable result (but is still encoded correctly.)
         */
        #[must_use]
        pub fn isodecode(&self) -> IString {
            let mut ret = Utf16String::new();

            for c in &self.elems {
                ret.push(*c as char);
            }
            IString::Scalar(ret)
        }

        #[must_use]
        pub fn ci_matches(&self, b: &Self) -> bool {
            self.clone().blower() == b.clone().blower()
        }

        #[must_use]
        fn is_ascii(&self) -> bool {
            for b in self.elems.as_slice() {
                if b.is_ascii() {
                    continue;
                }
                return false;
            }
            true
        }

        /// From the [Infra standard](https://infra.spec.whatwg.org/), with
        /// variable names changed to match written code:
        ///
        /// > To **ASCII decode** a byte sequence `self`, run these steps:
        /// > 1. Assert: all bytes in `self` are ASCII bytes.
        /// > <note omitted>
        /// > 2. Return the isomorphic decoding of `self`.
        ///
        /// # Panics
        ///
        /// Panics if the string is not ASCII.
        // similar to IString::ascii_encode(), i think this is kinda dumb
        #[must_use]
        pub fn ascii_decode(&self) -> IString {
            assert!(self.is_ascii());

            self.isodecode()
        }
    }

    impl fmt::Display for Byteseq {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let mut s = f.alternate();

            if s {
                for b in &self.elems {
                    if *b < 0x20 || *b > 0x7E {
                        s = false;
                        break;
                    }
                }
                if s {
                    /*
                     * we are safe here because the above byte
                     * range check guarantees that this sequence
                     * is ASCII (a subset of UTF-8)
                     */
                    return write!(f, "`{}`", unsafe {
                        String::from_utf8_unchecked(self.elems.clone())
                    });
                }
            }

            let mut ashex = String::new();

            for b in self.as_bytes() {
                ashex.push_str(b.to_string().as_str());
                ashex.push(' ');
            }
            ashex.pop(); // don't like this but it'll work for right now
            write!(f, "{}", ashex)
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum IChar {
        Surrogate(u32),
        Scalar(char),
    }

    impl IChar {
        #[must_use]
        pub fn is_leading_surrogate(&self) -> bool {
            match self {
                Self::Surrogate(c) => *c >= 0xD008 && *c <= 0xDBFF,
                Self::Scalar(_) => false,
            }
        }

        #[must_use]
        pub fn is_trailing_surrogate(&self) -> bool {
            match self {
                Self::Surrogate(c) => *c >= 0xDC00 && *c <= 0xDFFF,
                Self::Scalar(_) => false,
            }
        }

        #[must_use]
        pub fn is_surrogate(&self) -> bool {
            match self {
                Self::Surrogate(_) =>
                    self.is_leading_surrogate() || self.is_trailing_surrogate(),
                Self::Scalar(_) =>
                    false,
            }
        }

        #[must_use]
        pub fn is_scalar(&self) -> bool {
            match self {
                Self::Surrogate(_) => false,
                Self::Scalar(_) => true,
            }
        }

        #[must_use]
        pub fn is_noncharacter(&self) -> bool {
            match self {
                Self::Surrogate(_) => false,
                Self::Scalar(_c @ '\u{FDD0}'..='\u{FDEF}') => true,
                Self::Scalar(c) => *c as u16 == 0xFFFE || *c as u16 == 0xFFFF,
            }
        }

        #[must_use]
        pub fn is_ascii(&self) -> bool {
            match self {
                Self::Surrogate(_) => false,
                Self::Scalar(c) => c.is_ascii(),
            }
        }

        #[must_use]
        pub fn is_newline(&self) -> bool {
            match self {
                Self::Surrogate(_) => false,
                Self::Scalar(c) => *c == '\x0A' || *c == '\x0D',
            }
        }

        #[must_use]
        pub fn is_tab_or_newline(&self) -> bool {
            match self {
                Self::Surrogate(_) => false,
                Self::Scalar(c) => *c == '\x09' || self.is_newline(),
            }
        }

        #[must_use]
        pub fn is_ascii_whitespace(&self) -> bool {
            match self {
                Self::Surrogate(_) =>
                    false,
                Self::Scalar(c) =>
                    self.is_tab_or_newline() || *c == '\x0C' || *c == '\x20',
            }
        }

        #[must_use]
        pub fn is_c0(&self) -> bool {
            match self {
                Self::Surrogate(_) => false,
                Self::Scalar(c) => *c >= '\x00' && *c <= '\x1F',
            }
        }

        #[must_use]
        pub fn is_c0_or_space(&self) -> bool {
            match self {
                Self::Surrogate(_) => false,
                Self::Scalar(c) => self.is_c0() || *c == '\x20',
            }
        }

        #[must_use]
        pub fn is_control(&self) -> bool {
            match self {
                Self::Surrogate(_) => false,
                Self::Scalar(c) => c.is_control(),
            }
        }

        #[must_use]
        pub fn is_ascii_digit(&self) -> bool {
            match self {
                Self::Surrogate(_) => false,
                Self::Scalar(c) => c.is_ascii_digit(),
            }
        }

        #[must_use]
        pub fn is_ascii_upperhex(&self) -> bool {
            match self {
                Self::Surrogate(_) =>
                    false,
                Self::Scalar(c) => 
                    c.is_ascii_digit() ||
                    (c.is_ascii_uppercase() && c.is_ascii_hexdigit())
            }
        }

        #[must_use]
        pub fn is_ascii_lowerhex(&self) -> bool {
            match self {
                Self::Surrogate(_) =>
                    false,
                Self::Scalar(c) =>
                    c.is_ascii_digit() ||
                    (c.is_ascii_lowercase() && c.is_ascii_hexdigit())
            }
        }

        #[must_use]
        pub fn is_ascii_hexdigit(&self) -> bool {
            match self {
                Self::Surrogate(_) => false,
                Self::Scalar(c) => c.is_ascii_hexdigit(),
            }
        }

        #[must_use]
        pub fn is_ascii_upperalpha(&self) -> bool {
            match self {
                Self::Surrogate(_) => false,
                Self::Scalar(c) => c.is_ascii_uppercase(),
            }
        }

        #[must_use]
        pub fn is_ascii_loweralpha(&self) -> bool {
            match self {
                Self::Surrogate(_) => false,
                Self::Scalar(c) => c.is_ascii_lowercase(),
            }
        }

        #[must_use]
        pub fn is_ascii_alpha(&self) -> bool {
            match self {
                Self::Surrogate(_) => false,
                Self::Scalar(c) => c.is_ascii_alphabetic(),
            }
        }

        #[must_use]
        pub fn is_ascii_alphanumeric(&self) -> bool {
            self.is_ascii_digit() || self.is_ascii_alpha()
        }

        /// Create an `IChar` from a raw surrogate pair. `hi` must be a high or
        /// leading surrogate, and `lo` must be a low or trailing surrogate.
        ///
        /// # Notes
        ///
        /// High surrogates have lower values than low surrogates. Do not get
        /// them confused if you are trying to compare them.
        ///
        /// # Panics
        ///
        /// Panics if `hi` is outside the range of high surrogates, or if `lo`
        /// is outside the range of low surrogates.
        #[must_use]
        pub fn from_surrogate_pair(hi: u16, lo: u16) -> Self {
            assert!(Self::is_leading_surrogate(&Self::Surrogate(hi as u32)));
            assert!(Self::is_trailing_surrogate(&Self::Surrogate(lo as u32)));

            let hbits = ((hi - 0xD800) * 0x400) as u32;
            let lbits = (lo - 0xDC00) as u32;

            /*
             * safe because this algorithm guarantees the result
             * will never be a surrogate
             */
            IChar::Scalar(unsafe {
                char::from_u32_unchecked(hbits + lbits + 0x10000)
            })
        }

        // if it's a surrogate pair, parse them. if not, return a as an ichar
        #[must_use]
        pub fn from_u16_pair(a: u16, b: u16) -> Self {
            if Self::Surrogate(a as u32).is_leading_surrogate()
                && Self::Surrogate(b as u32).is_trailing_surrogate()
            {
                return Self::from_surrogate_pair(a, b);
            }
            Self::from(a)
        }

        #[must_use]
        pub fn len_utf16(&self) -> usize {
            match self {
                Self::Surrogate(_) => 1,
                Self::Scalar(c) => c.len_utf16(),
            }
        }

        #[must_use]
        pub fn is_in(&self, s: &str) -> bool {
            for m in ichars_from_str(s) {
                if m == *self {
                    return true;
                }
            }
            false
        }

        #[must_use]
        pub fn is_not_in(&self, s: &str) -> bool {
            !self.is_in(s)
        }

        #[allow(clippy::cast_possible_truncation)] // intentional here
        #[must_use]
        pub fn as_vec(&self) -> Vec<u16> {
            let mut ret = Vec::new();

            match self {
                Self::Surrogate(s) => ret.push(*s as u16),
                Self::Scalar(c) => {
                    let mut b = [0; 2];

                    for u in c.encode_utf16(&mut b) {
                        ret.push(*u);
                    }
                }
            }
            ret
        }

        #[must_use]
        pub fn as_u32(&self) -> u32 {
            match self {
                IChar::Surrogate(c) => *c,
                IChar::Scalar(c) => *c as u32,
            }
        }
    }

    impl fmt::Display for IChar {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                IChar::Surrogate(c) => c.fmt(f),
                IChar::Scalar(c) => c.fmt(f),
            }
        }
    }

    impl From<u16> for IChar {
        fn from(value: u16) -> Self {
            IChar::from(value as u32)
        }
    }

    impl From<u32> for IChar {
        fn from(value: u32) -> Self {
            if Self::Surrogate(value).is_surrogate() {
                return Self::Surrogate(value)
            }
            // safe because all remaining values are scalar
            Self::Scalar(unsafe {
                char::from_u32_unchecked(value)
            })
        }
    }

    impl From<char> for IChar {
        fn from(value: char) -> Self {
            Self::Scalar(value)
        }
    }

    #[derive(Eq, Debug, Clone)]

    pub enum IString {
        Icky(U16String),
        Scalar(Utf16String),
    }

    impl IString {
        /// Unwrap a scalar `IString` and reveal the underlying
        /// `Utf16String`, panicking if it is not scalar.
        ///
        /// # Panics
        ///
        /// Panics if the string is not scalar.
        /*
         * consistent with Result's unwrap() that doesn't have
         * must_use. perhaps you simply want to panic when an icky
         * string is present - something like string.clone().unwrap()
         * (though probably slower than a match statement)
         */
        #[allow(clippy::must_use_candidate)]
        pub fn unwrap(self) -> Utf16String {
            match self {
                Self::Icky(_) => panic!("unwrapped an icky infra string: {:?}", self),
                Self::Scalar(utf16) => utf16,
            }
        }

        /*
         * our from_str implementation can't fail, so implementing
         * the trait doesn't make sense here
         * today i learned about the Infallible enum. may change this
         * to return Result<Self, Infallible> but low priority
         */
        #[allow(clippy::should_implement_trait)]
        #[must_use]
        pub fn from_str(s: &str) -> Self {
            Self::Scalar(Utf16String::from_str(s))
        }

        #[must_use]
        pub fn len(&self) -> usize {
            match self {
                Self::Icky(u16_string) => u16_string.len(),
                Self::Scalar(utf16_string) => utf16_string.len(),
            }
        }

        #[must_use]
        pub fn is_empty(&self) -> bool {
            self.len() == 0
        }

        #[must_use]
        pub fn as_ichars(&self) -> Vec<IChar> {
            u16_slice_as_ichars(self.as_slice())
        }

        #[must_use]
        #[allow(clippy::cast_possible_truncation)] // intentional here
        pub fn from_ichars(v: &Vec<IChar>) -> Self {
            let mut ret: Vec<u16> = Vec::new();

            for c in v {
                match c {
                    IChar::Surrogate(val) => ret.push(*val as u16),
                    IChar::Scalar(val) => {
                        let mut b: [u16; 2] = [0; 2];
                        let enc = val.encode_utf16(&mut b);

                        for cu in enc {
                            ret.push(*cu);
                        }
                    }
                }
            }
            Self::wrap(ret)
        }

        #[must_use]
        pub fn cplen(&self) -> usize {
            self.as_ichars().len()
        }

        #[must_use]
        // doesn't compose surrogate pairs into chars but this is fine for now
        fn all_under_x(&self, x: u16) -> bool {
            let slice = self.as_slice();

            for u in slice {
                if *u > x {
                    return false;
                }
            }
            true
        }

        #[must_use]
        pub fn is_ascii(&self) -> bool {
            match self {
                Self::Icky(_) => false,
                Self::Scalar(_) => self.all_under_x(0x7F),
            }
        }

        #[must_use]
        pub fn is_isomorphic(&self) -> bool {
            match self {
                Self::Icky(_) => false,
                Self::Scalar(_) => self.all_under_x(0xFF),
            }
        }

        #[must_use]
        pub fn is_scalar(&self) -> bool {
            match self {
                Self::Icky(_) => false,
                Self::Scalar(_) => true,
            }
        }

        #[must_use]
        fn is_icky(&self) -> bool {
            match self {
                Self::Icky(_) => true,
                Self::Scalar(_) => false,
            }
        }

        /*
         * the widestring crate has a from_ustr_lossy() function that
         * should, in theory, provide the bulk of the guts of this
         * function, but at last attempt on 2/3/2026 something was
         * buggy about it and it wasn't encoding the string correctly.
         * upon attempting to use the scalarized U16String in a
         * println!() statement, an unwrap call inside the crate
         * failed and my test suite panicked. as a result, i had to
         * steal the guts from the documentation for decode_utf16().
         * it is possible to implement this with as_ichars too but i
         * am just going to leave it as-is (the current
         * implementation predates as_ichars(), and i only fixed code
         * that had a very similar while loop to the code of
         * as_ichars(). a previous implementation of scalarize() with
         * a different utf16 library did have a form of that loop.)
         * personally, an as_ichars version would be easier to
         * understand, but if it ain't broke, don't fix it.
         * TODO: report bug to crate maintainer
         */
        #[must_use]
        pub fn scalarize(self) -> Self {
            match self {
                Self::Icky(u16_string) => {
                    let vec = u16_string.as_vec();

                    Self::Scalar(
                        decode_utf16(vec.clone())
                            .map(|r| r.unwrap_or('\u{FFFD}'))
                            .collect::<Utf16String>(),
                    )
                }
                Self::Scalar(_) => self,
            }
        }

        #[must_use]
        fn as_slice(&self) -> &[u16] {
            match self {
                Self::Icky(u16_string) => u16_string.as_slice(),
                Self::Scalar(utf16_string) => utf16_string.as_slice(),
            }
        }

        #[must_use]
        pub fn starts_with(&self, prefix: &Self) -> bool {
            if prefix.len() > self.len() {
                return false;
            }

            let s_slice = self.as_slice();
            let p_slice = prefix.as_slice();
            let mut i = 0;

            loop {
                if i >= prefix.len() {
                    return true;
                }
                if p_slice[i] != s_slice[i] {
                    return false;
                }
                i += 1;
            }
        }

        #[must_use]
        pub fn ends_with(&self, suffix: &Self) -> bool {
            if suffix.len() > self.len() {
                return false;
            }

            let self_slice = self.as_slice();
            let suffix_slice = suffix.as_slice();
            let mut i = 1;

            loop {
                if i > suffix.len() {
                    return true;
                }

                let suffix_i = suffix.len() - i;
                let self_i = self.len() - i;

                if suffix_slice[suffix_i] != self_slice[self_i] {
                    return false;
                }
                i += 1;
            }
        }

        #[must_use]
        pub fn wrap(v: Vec<u16>) -> Self {
            if is_valid_utf16(&v) {
                unsafe {
                    // pre-validated, therefore safe
                    return Self::Scalar(Utf16String::from_vec_unchecked(v));
                }
            }
            Self::Icky(U16String::from_vec(v))
        }

        /// From the [Infra standard](https://infra.spec.whatwg.org/), with
        /// variable names changed to match written code:
        ///
        /// > The **code unit substring** {a/n: cu_substr family of functions}
        /// > from `start` with length `len` within a string `self` is
        /// > determined as follows:
        ///
        /// The described algorithm has been ommitted, since the code has been
        /// implemented with slices instead and does not match (but produces
        /// the same result.)
        ///
        /// In summary, returns a substring starting at the code unit index
        /// `start` and ending at `start + len`.
        ///
        /// # Panics
        ///
        /// Panics if `start + len` is greater than `self.len()`.
        #[must_use]
        pub fn cu_substr_len(&self, start: usize, len: usize) -> Self {
            assert!(start + len <= self.len());

            Self::wrap(self.as_slice()[start..start + len].to_vec())
        }

        #[must_use]
        pub fn cu_substr_end_index(&self, start: usize, end_i: usize) -> Self {
            self.cu_substr_len(start, end_i - start)
        }

        #[must_use]
        pub fn cu_substr_to_end(&self, start: usize) -> Self {
            self.cu_substr_end_index(start, self.len())
        }

        /// From the [Infra standard](https://infra.spec.whatwg.org/), with
        /// variable names changed to match written code:
        ///
        /// > The **code point substring** {a/n: cp_substr family of functions}
        /// > within a string `self` from `start` with length `len` is
        /// > determined as follows:
        ///
        /// The described algorithm has been ommitted, since the code has been
        /// implemented with slices instead and does not match (but produces
        /// the same result.)
        ///
        /// In summary, returns a substring starting at the code point index
        /// `start` and ending at `start + len`.
        ///
        /// # Panics
        ///
        /// Panics if `start + len` is greater than `self.cplen()`.
        #[must_use]
        pub fn cp_substr_len(&self, start: usize, len: usize) -> Self {
            assert!(start + len <= self.cplen());

            Self::from_ichars(&self.as_ichars()[start..start + len].to_vec())
        }

        #[must_use]
        pub fn cp_substr_end_index(&self, start: usize, end_i: usize) -> Self {
            self.cp_substr_len(start, end_i - start)
        }

        #[must_use]
        pub fn cp_substr_to_end(&self, start: usize) -> Self {
            self.cp_substr_end_index(start, self.cplen())
        }

        /// From the [Infra standard](https://infra.spec.whatwg.org/), with
        /// variable names changed to match written code:
        ///
        /// > To **isomorphic encode** an isomorphic string `self`: return a
        /// > byte sequence whose length is equal to input’s code point length
        /// > and whose bytes have the same values as the values of `self`’s
        /// > code points, in the same order.
        ///
        /// # Panics
        ///
        /// Panics if the string is not isomorphic.
        /*
         * unsure if an assert is the correct solution here, but the
         * standard assumes the provided string is isomorphic
         * i could truncate each code unit (the standard specifies
         * code points, but for isomorphic strings code points ==
         * code units) to 8 bits and use that, but this is certainly
         * not expected, or even useful, behavior.
         */
        #[must_use]
        #[allow(clippy::cast_possible_truncation)] // intentional here
        pub fn isoencode(&self) -> Byteseq {
            assert!(self.is_isomorphic());

            let mut ret = Byteseq::new();

            for u in self.as_slice() {
                ret.elems.push(*u as u8);
            }
            ret
        }

        #[must_use]
        unsafe fn as_mut_slice(&mut self) -> &mut [u16] {
            match self {
                Self::Icky(u16_string) => u16_string.as_mut_slice(),
                Self::Scalar(utf16_string) => unsafe {
                    utf16_string.as_mut_slice()
                },
            }
        }

        pub fn ascii_lower(&mut self) -> &mut Self {
            // safe because we are only modifying ascii codeunits
            let slice = unsafe { self.as_mut_slice() };

            for u in slice {
                let c = IChar::from(*u);

                if c.is_ascii_upperalpha() {
                    *u += 0x20;
                }
            }
            self
        }

        pub fn ascii_upper(&mut self) -> &mut Self {
            // safe because we are only modifying ascii codeunits
            let slice = unsafe { self.as_mut_slice() };

            for u in slice {
                let c = IChar::from(*u);

                if c.is_ascii_loweralpha() {
                    *u -= 0x20;
                }
            }
            self
        }

        #[must_use]
        pub fn ascii_ci_matches(&self, b: &IString) -> bool {
            self.clone().ascii_lower() == b.clone().ascii_lower()
        }

        /// From the [Infra standard](https://infra.spec.whatwg.org/), with
        /// variable names changed to match written code:
        ///
        /// > To **ASCII encode** an ASCII string `self`: return the isomorphic
        /// > encoding of input.
        ///
        /// # Panics
        ///
        /// Panics if the string is not ASCII.
        /*
         * i personally think this function is kinda dumb but the standard
         * specifies it. also, like isoencode(), unsure if the assert is right.
         */
        #[must_use]
        pub fn ascii_encode(&self) -> Byteseq {
            assert!(self.is_ascii());

            self.isoencode()
        }

        fn retain<F>(&mut self, mut f: F)
        where
            F: FnMut(IChar) -> bool,
        {
            let v = self.as_ichars();
            let mut keep = Vec::new();

            for c in v {
                keep.push(f(c));
                if c.len_utf16() == 2 && self.is_icky() {
                    keep.push(f(c)); // for the trailing surrogate
                }
            }

            let mut iter = keep.iter();

            match self {
                Self::Icky(u16_string) =>
                    u16_string.retain(|_| *iter.next().unwrap()),
                Self::Scalar(utf16_string) =>
                    utf16_string.retain(|_| *iter.next().unwrap()),
            }
        }

        pub fn strip_newlines(&mut self) -> &mut Self {
            self.retain(|i| i.is_not_in("\u{000A}\u{000D}"));
            self
        }

        /*
         * functions like these have made me replace the utf16 crate before.
         * let us pray that we will not need to do it again
         */
        fn replace_range<R>(&mut self, range: R, replace_with: &[u16])
        where
            R: RangeBounds<usize>,
        {
            let rw_wrapped = Self::wrap(replace_with.to_vec());

            match self {
                Self::Icky(s_raw) => match rw_wrapped {
                    Self::Icky(rw_raw) =>
                        s_raw.replace_range(range, rw_raw),
                    Self::Scalar(rw_utf) =>
                        s_raw.replace_range(range, rw_utf.as_ustr()),
                },
                Self::Scalar(s_utf) => match rw_wrapped {
                    Self::Icky(_) =>
                            s_utf.replace_range(
                            range,
                            &rw_wrapped.scalarize().unwrap()
                        ),
                    Self::Scalar(rw_utf) =>
                        s_utf.replace_range(range, &rw_utf),
                },
            }
        }

        /*
         * takes closure f, collapes everything that matches f into c
         * WARNING: only works at the codeunit level, does not compose
         * surrogate pairs into code points
         */
        fn collapse<F>(&mut self, mut f: F, c: u16) -> &mut Self
        where
            F: FnMut(IChar) -> bool,
        {
            let mut ranges: Vec<(usize, usize)> = Vec::new();
            {
                // new scope so we can drop immutables before mutating
                let slice = self.as_slice();
                let mut i = 0;

                while i < slice.len() {
                    let mut range = (0, 0);

                    range.0 = i;
                    while i < slice.len() && f(IChar::from(slice[i])) {
                        range.1 = i;
                        i += 1;
                    }
                    if range.1 != 0 {
                        ranges.push(range);
                    }

                    i += 1;
                }
            }
            for r in ranges.iter().rev() {
                self.replace_range(r.0..=r.1, slice::from_ref(&c));
            }
            self
        }

        // converts all non-unix line endings to unix, just without saying so
        pub fn normalize_nl(&mut self) -> &mut Self {
            self.collapse(|c| c.is_newline(), 0x000A)
        }

        /*
         * takes closure f, strips all lead/trail chars that match f
         * WARNING: only works at the codeunit level, does not compose
         * surrogate pairs into code points
         */
        fn shell<F>(&mut self, mut f: F) -> &mut Self
        where
            F: FnMut(IChar) -> bool,
        {
            let slice = self.as_slice();
            let mut leadi = 0;
            let mut traili = 0;
            for (i, c) in slice.iter().enumerate() {
                if f(IChar::from(*c)) {
                    continue;
                }
                leadi = i;
                break;
            }
            for (i, c) in slice.iter().rev().enumerate() {
                if f(IChar::from(*c)) {
                    continue;
                }
                traili = self.len() - i;
                break;
            }
            self.replace_range(traili..self.len(), &[]);
            self.replace_range(0..leadi, &[]);
            self
        }

        pub fn shell_whitespace(&mut self) -> &mut Self {
            self.shell(|c| c.is_ascii_whitespace())
        }

        pub fn shell_and_collapse_whitespace(&mut self) -> &mut Self {
            self.shell_whitespace();
            self.collapse(|c| c.is_ascii_whitespace(), 0x0020)
        }

        #[must_use]
        fn next_codepoint(&self, pos: &mut usize) -> IChar {
            let slice = self.as_slice();

            if *pos < self.len() - 1 {
                IChar::from_u16_pair(slice[*pos], slice[*pos + 1])
            } else {
                IChar::from(slice[*pos])
            }
        }

        /*
         * note: algorithm in standard assumes you are iterating by codepoints.
         * this is not ideal for multiple reasons i won't go into here.
         * we will instead take each codeunit two at a time and increment by
         * the correct utf16 length
         * also, the standard requires us to discard the return value of this
         * in some cases, so silence clippy for must_use.
         */
        #[allow(clippy::return_self_not_must_use)]
        pub fn collect<F>(&self, mut f: F, pos: &mut usize) -> Self
        where
            F: FnMut(IChar) -> bool,
        {
            let mut ret: Vec<u16> = Vec::new();

            while pos < &mut self.len() {
                let cp = self.next_codepoint(pos);

                if !f(cp) {
                    break;
                }

                ret.append(&mut cp.as_vec());
                *pos += cp.len_utf16();
            }
            Self::wrap(ret)
        }

        // calls collect() purely to update pos, does nothing else
        pub fn skip_ascii_whitespace(&self, pos: &mut usize) {
            self.collect(|c| c.is_ascii_whitespace(), pos);
        }

        #[must_use]
        #[allow(clippy::missing_panics_doc)] // if we panic this is a bug
        pub fn strict_split(&self, delim: IChar) -> List<IString> {
            let mut pos = 0;
            let mut tokens = List::new();
            let mut token = self.collect(|c| c != delim, &mut pos);

            tokens.append(token);
            while pos < self.len() {
                assert_eq!(self.next_codepoint(&mut pos), delim);

                pos += self.next_codepoint(&mut pos).len_utf16();
                token = self.collect(|c| c != delim, &mut pos);
                tokens.append(token);
            }
            tokens
        }
        
        #[must_use]
        pub fn split_ascii_whitespace(&self) -> List<IString> {
            let mut pos = 0;
            let mut tokens = List::new();

            self.skip_ascii_whitespace(&mut pos);
            while pos < self.len() {
                let token = self.collect(
                    |c| !c.is_ascii_whitespace(),
                    &mut pos
                );

                tokens.append(token);
                self.skip_ascii_whitespace(&mut pos);
            }
            tokens
        }

        #[must_use]
        #[allow(clippy::missing_panics_doc)] // if we panic this is a bug
        pub fn split_commas(&self) -> List<IString> {
            let mut pos = 0;
            let mut tokens = List::new();
            let comma = IChar::from(',');

            while pos < self.len() {
                let mut token = self.collect(|c| c != comma, &mut pos);

                token.shell_whitespace();
                tokens.append(token);
                if pos < self.len() {
                    assert_eq!(self.next_codepoint(&mut pos), comma);
                    pos += 1;
                }
            }
            tokens
        }
        
        /*
         * standard specifies that the separator can be optional, and defaults
         * to the empty string, but this isn't possible within Rust. pass the
         * empty string manually if you don't care
         * maybe: change sep to &Option<IString>?
         */
        #[must_use]
        pub fn concat(list: &[IString], sep: &IString) -> Self {
            let mut ret = Vec::new();
            let mut i = 1;

            if list.is_empty() {
                return Self::wrap(ret);
            }
            ret.append(&mut list[0].as_slice().to_vec());
            while i < list.len() {
                ret.append(&mut sep.as_slice().to_vec());
                ret.append(&mut list[i].as_slice().to_vec());
                i += 1;
            }
            Self::wrap(ret)
        }

        // WARNING: here be dragons (Javascript nonsense beyond this point)

        /*
         * standard goes into additional semantics, but fortunately
         * we can cheat with IString::as_ichars and massage it into a
         * new List with List::extend_vec
         */
        #[must_use]
        pub fn to_code_points(&self) -> List<IChar> {
            let mut code_points = List::new();

            code_points.extend_vec(self.as_ichars());
            code_points
        }
/*
        #[must_use]
        #[allow(clippy::missing_panics_doc)] // if we panic this is a bug
        pub fn to_number(&self) -> Number {
            match parse_text(self, old_js_base::parse_string_numeric_literal) {
                Ok(n) => n.value.unwrap().number(),
                Err(_) => Number(f64::NAN),
            }
        }*/
    }

    impl PartialEq for IString {
        fn eq(&self, other: &Self) -> bool {
            if self.len() != other.len() {
                return false;
            }

            let mut i = 0;
            let a_slice = self.as_slice();
            let b_slice = other.as_slice();

            while i < self.len() && a_slice[i] == b_slice[i] {
                i += 1;
            }
            i == self.len()
        }
    }

    impl PartialOrd for IString {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for IString {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            if self == other {
                return std::cmp::Ordering::Equal;
            }
            if self.starts_with(other) {
                return std::cmp::Ordering::Greater;
            }
            if other.starts_with(self) {
                return std::cmp::Ordering::Less;
            }
            let self_slice = self.as_slice();
            let other_slice = other.as_slice();
            let mut i = 0;

            while self_slice[i] != other_slice[i] {
                i += 1;
            }
            self_slice[i].cmp(&other_slice[i])
        }
    }

    impl fmt::Display for IString {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Icky(_) => self.clone().scalarize().fmt(f),
                Self::Scalar(utf16_string) => utf16_string.fmt(f),
            }
        }
    }

    pub enum IType<T, U, K, V>
    where
        T: 'static,
        U: Ord + 'static,
        V: Clone,
    {
        None,
        U8Int(u8),
        U16Int(u16),
        U32Int(u32),
        U64Int(u64),
        U128Int(u128),
        S8Int(i8),
        S16Int(i16),
        S32Int(i32),
        S64Int(i64),
        Byte(Byte),
        Byteseq(Byteseq),
        Codepoint(IChar),
        Moment(Instant),
        Duration(Duration),
        Stack(Stack<T>),
        Queue(Queue<T>),
        Set(Set<U>),
        Map(Map<K, V>),
        Struct(Struct<T, U, K, V>),
    }

    impl<T, U, K, V> Clone for IType<T, U, K, V>
    where
        T: 'static,
        U: Ord + 'static,
        V: Clone,
    {
        fn clone(&self) -> Self {
            match self {
                Self::None => Self::None,
                Self::U8Int(arg0) => Self::U8Int(*arg0),
                Self::U16Int(arg0) => Self::U16Int(*arg0),
                Self::U32Int(arg0) => Self::U32Int(*arg0),
                Self::U64Int(arg0) => Self::U64Int(*arg0),
                Self::U128Int(arg0) => Self::U128Int(*arg0),
                Self::S8Int(arg0) => Self::S8Int(*arg0),
                Self::S16Int(arg0) => Self::S16Int(*arg0),
                Self::S32Int(arg0) => Self::S32Int(*arg0),
                Self::S64Int(arg0) => Self::S64Int(*arg0),
                Self::Byte(arg0) => Self::Byte(*arg0),
                Self::Byteseq(arg0) => Self::Byteseq(arg0.clone()),
                Self::Codepoint(arg0) => Self::Codepoint(*arg0),
                Self::Moment(arg0) => Self::Moment(*arg0),
                Self::Duration(arg0) => Self::Duration(*arg0),
                Self::Stack(arg0) => Self::Stack(arg0.clone()),
                Self::Queue(arg0) => Self::Queue(arg0.clone()),
                Self::Set(arg0) => Self::Set(arg0.clone()),
                Self::Map(arg0) => Self::Map(arg0.clone()),
                Self::Struct(arg0) => Self::Struct(arg0.clone()),
                //Self::JType(arg0) => Self::JType(arg0.clone()),
                //Self::JSpecType(arg0) => Self::JSpecType(arg0.clone()),
            }
        }
    }

    impl<T, U, K, V> std::fmt::Debug for IType<T, U, K, V>
    where
        T: 'static,
        U: Ord + 'static,
        V: Clone,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::None => write!(f, "None"),
                Self::U8Int(arg0) => f.debug_tuple("U8Int").field(arg0).finish(),
                Self::U16Int(arg0) => f.debug_tuple("U16Int").field(arg0).finish(),
                Self::U32Int(arg0) => f.debug_tuple("U32Int").field(arg0).finish(),
                Self::U64Int(arg0) => f.debug_tuple("U64Int").field(arg0).finish(),
                Self::U128Int(arg0) => f.debug_tuple("U128Int").field(arg0).finish(),
                Self::S8Int(arg0) => f.debug_tuple("S8Int").field(arg0).finish(),
                Self::S16Int(arg0) => f.debug_tuple("S16Int").field(arg0).finish(),
                Self::S32Int(arg0) => f.debug_tuple("S32Int").field(arg0).finish(),
                Self::S64Int(arg0) => f.debug_tuple("S64Int").field(arg0).finish(),
                Self::Byte(arg0) => f.debug_tuple("Byte").field(arg0).finish(),
                Self::Byteseq(arg0) => f.debug_tuple("Byteseq").field(arg0).finish(),
                Self::Codepoint(arg0) => f.debug_tuple("Codepoint").field(arg0).finish(),
                Self::Moment(arg0) => f.debug_tuple("Moment").field(arg0).finish(),
                Self::Duration(arg0) => f.debug_tuple("Duration").field(arg0).finish(),
                Self::Stack(_) => write!(f, "some Stack"),
                Self::Queue(_) => write!(f, "some Queue"),
                Self::Set(_) => write!(f, "some Set"),
                Self::Map(_) => write!(f, "some Map"),
                Self::Struct(_) => write!(f, "some Struct"),
                //Self::JType(arg0) => f.debug_tuple("JType").field(arg0).finish(),
                //Self::JSpecType(arg0) => f.debug_tuple("JSpecType").field(arg0).finish()
            }
        }
    } 
}
