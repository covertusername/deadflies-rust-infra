use infra::{primitives::{Byte, Byteseq, IChar, IString}};
//use infra::old_js_base;
//use infra::old_js_base::parse_text;
use widestring::{U16String, Utf16String};

fn main() {
    let bval = Byte(0xFF);

    println!("***BYTE TESTS***");
    println!("Byte construction test: {}", bval);
    assert!(bval.to_string() == "0xFF");

    let bsval = Byteseq::from_str("Hello, world!");

    println!("\n***BYTESEQ TESTS***");
    assert!(
        bsval.to_string() == "0x48 0x65 0x6C 0x6C 0x6F 0x2C 0x20 0x77 0x6F \
        0x72 0x6C 0x64 0x21"
    );

    println!("Byteseq from str, as str: {:#}", bsval);
    println!(
        "Byteseq from str, as str, with format fail: {:#}",
        Byteseq::from_str("☝︎✌︎💧︎❄︎☜︎☼︎")
    );
    println!("Byteseq from str, as bytes: '{}'", bsval);

    assert!(bsval.len() == 13);
    println!("Length of Byteseq: {}", bsval.len());

    let mut bsval = Byteseq::from_str("HELLO, WORLD!");

    assert!(
        bsval.blower().to_string() == "0x68 0x65 0x6C 0x6C 0x6F 0x2C 0x20 \
        0x77 0x6F 0x72 0x6C 0x64 0x21"
    );
    println!("Byteseq, lowercased: '{}'", bsval);

    assert!(
        bsval.bupper().to_string() == "0x48 0x45 0x4C 0x4C 0x4F 0x2C 0x20 \
        0x57 0x4F 0x52 0x4C 0x44 0x21"
    );
    println!("Byteseq, uppercased: '{}'", bsval);

    let mut bsvaltb = bsval.clone();
    let lower = bsvaltb.blower();
    if lower.clone().ci_matches(&bsval.clone()) {
        println!(
            "Byte sequences {:#} and {:#} are case-insensitive matches",
            lower,
            bsval
        );
    } 
    else {
        println!(
            "TEST FAILURE: Byte sequences {:#} and {:#} aren't \
            case-insensitive matches",
            lower,
            bsval
        );
    }

    let bsval = Byteseq::from_str("Hello, world!");
    if bsval.starts_with(&Byteseq::from_str("Hello")) {
        println!("`Hello, world!` starts with `Hello`")
    }
    else {
        println!("TEST FAILURE: `Hello, world!` does not start with `Hello`")
    }

    let bsval = Byteseq::from_str("Goober");
    if bsval.starts_with(&Byteseq::from_str("Foo")) {
        println!("TEST FAILURE: `Goober` should not start with `Foo`")
    }
    else {
        println!("`Goober` does not start with `Foo`")
    }

    if Byteseq::from_str("Hello, world") != Byteseq::from_str("Hello, world") {
        println!("TEST FAILURE: Equivalency isn't working!")
    }
    else {
        println!("Equivalency works.")
    }

    if Byteseq::from_str("Hello, world!") == Byteseq::from_str("Goodbye!") {
        println!("TEST FAILURE: Inequality isn't working!")
    }
    else {
        println!("Inequality works.")
    }

    if Byteseq::from_str("hello, world") < Byteseq::from_str("hello, worlc") {
        println!("TEST FAILURE: Less-than isn't working!")
    }
    else {
        println!("Less-than works.")
    }

    if Byteseq::from_str("hello, world") > Byteseq::from_str("hello, worle") {
        println!("TEST FAILURE: Greater-than isn't working!")
    }
    else {
        println!("Greater-than works.")
    }

    if Byteseq::from_str("hello, world") < Byteseq::from_str("hello, world") {
        println!("TEST FAILURE: Equivalent byte sequences are less than each \
            other!")
    }
    else {
        println!("Equivalent strings are not less than each other.")
    }

    let bsval = Byteseq::from_str("Hello, world!");
    let binding = bsval.clone().isodecode();
    let utf16 = binding.unwrap();
    if bsval.len() != utf16.len() {
        println!("CRITICAL TEST FAILURE: Math error: Byte sequence length \
            changed on isomorphic decode!");
        println!(
            "bsval len: {}, bsdecoded len: {}",
            bsval.len(),
            utf16.len()
        )
    }
    else {
        println!("Byte sequence {:#}, when isomorphically decoded, maintains \
            its character length.", bsval);
    
        let mut iter = utf16.chars();
        let mut failed = false;

        for b in bsval.as_bytes() {
            if b.0 as char != iter.next().unwrap() {
                println!("TEST FAILURE: Subunit equivalency not maintained on \
                    isomorphic decode!");
                failed = true;
                break
            }
        }
        if !failed {
            println!("Subunit equivalency maintained on isomorphic decode.")
        }
    }

    let bsval = Byteseq::from_str("☝︎✌︎💧︎❄︎☜︎☼︎");
    let binding = bsval.clone().isodecode();
    let utf16 = binding.unwrap();
    println!(
        "{:#} len: {}, {} len: {}", 
        bsval, 
        bsval.len(), 
        utf16, 
        utf16.len()
    );
    let mut iter = utf16.chars();
    let mut failed = false;
    
    for b in bsval.as_bytes() {
        if b.0 as char != iter.next().unwrap() {
            println!("TEST FAILURE: Subunit equivalency not maintained on \
                isomorphic decode for {:#}!", bsval);
            failed = true;
            break
        }
    }
    if !failed {
        println!("Subunit equivalency maintained for UTF-8 sequences too.")
    }

    println!("\n***ICHAR TESTS***");

    println!(
        "Tab is tab or newline: {}",
        IChar::Scalar('\x09').is_tab_or_newline()
    );
    println!(
        "Line feed is tab or newline: {}",
        IChar::Scalar('\x0A').is_tab_or_newline()
    );
    println!(
        "Carriage return is tab or newline: {}", 
        IChar::Scalar('\x09').is_tab_or_newline()
    );
    println!(
        "Space is tab or newline: {}",
        IChar::Scalar(' ').is_tab_or_newline()
    );
    println!("Nul is c0: {}", IChar::Scalar('\x00').is_c0());
    println!("Space is c0: {}", IChar::Scalar(' ').is_c0());

    println!("\n***ISTRING TESTS***");

    println!("Scalarized output: {} (should be 💩��)", IString::Icky(
        U16String::from_vec(vec![0xD83D, 0xDCA9, 0xD800, 0xD801])
    ).scalarize().unwrap());

    println!(
        "Code unit substring from index 1 of length 3 in 'Hello, world!': {}",
            IString::Scalar(Utf16String::from_str("Hello, world!"))
            .cu_substr_len(1, 3)
    );

    println!(
        "Code point substring from index 1 of length 4 in '☝︎✌︎💧︎❄︎☜︎☼︎': {:?}",
            IString::Scalar(Utf16String::from_str("☝︎✌︎💧︎❄︎☜︎☼︎"))
            .cp_substr_len(1, 4)
    );

    let lower = IString::from_str("hello, world");
    let upper = IString::from_str("HELLO, WORLD");

    println!(
        "Strings {} and {} match (ignoring case): {}",
        lower,
        upper,
        lower.ascii_ci_matches(&upper)
    );

    let mut nls = IString::from_str("Hello, \nWorld\n!");
    println!("String, with newlines: {}", nls);
    println!("String, with newlines stripped: {}", nls.strip_newlines());

    let mut binding = IString::from_str("Multiple\rnasty\rlines\r");
    let cr = binding.normalize_nl();

    println!("String with newlines normalized: {:?}", cr);

    let mut binding = IString::from_str("      Hello, world!     ");
    let stripped = binding.shell_whitespace();

    println!("String shelled out of whitespace: {:?}", stripped);
    
    println!("***JAVASCRIPT TESTS***");
    /*println!(
        "parse_text (string numeric literal): {:#?}",
        parse_text(
            &IString::from_str("-3.14e-1"),
            old_js_base::parse_string_numeric_literal
        )
    );*/

    /*
    println!("Random Rust tests follow here.");
    println!("binary three: {}", 0b11);
    println!("octal nine: {}", 0o11);
    println!("hex seventeen: {}", 0x11);
    */
}
