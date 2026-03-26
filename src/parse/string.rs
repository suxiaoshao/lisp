//! String parsing with escape sequence support.
//!
//! This module provides a parser for Lisp string literals that handles
//! common escape sequences. It uses nom combinators to build a zero-copy
//! parser that processes strings character by character.
//!
//! # Supported Escape Sequences
//! - `\n` → newline (LF)
//! - `\r` → carriage return
//! - `\t` → horizontal tab
//! - `\b` → backspace
//! - `\f` → form feed
//! - `\\` → backslash
//! - `\/` → forward slash
//! - `\"` → double quote
//! - `\u{XXXX}` → Unicode character (hexadecimal, 1-6 digits)

use nom::branch::alt;
use nom::bytes::streaming::{is_not, take_while_m_n};
use nom::character::streaming::{char, multispace1};
use nom::combinator::{map, map_opt, map_res, value, verify};
use nom::error::{FromExternalError, ParseError};
use nom::multi::fold;
use nom::sequence::{delimited, preceded};
use nom::{IResult, Parser};

// parser combinators are constructed from the bottom up:
// first we write parsers for the smallest elements (escaped characters),
// then combine them into larger parsers.

/// Parse a Unicode escape sequence of the form `\u{XXXX}`.
///
/// The `XXXX` is 1 to 6 hexadecimal numerals representing a Unicode code point.
/// This function parses the `u{...}` syntax and converts the hex value to a `char`.
///
/// # Arguments
/// - `input`: Input string starting with `u{`
///
/// # Returns
/// - `Ok((remaining, char))` with the parsed Unicode character
/// - `Err` if the format is invalid or code point is not a valid Unicode scalar value
pub fn parse_unicode<'a, E>(input: &'a str) -> IResult<&'a str, char, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::num::ParseIntError>,
{
    // `take_while_m_n` parses between `m` and `n` bytes (inclusive) that match
    // a predicate. `parse_hex` here parses between 1 and 6 hexadecimal numerals.
    let parse_hex = take_while_m_n(1, 6, |c: char| c.is_ascii_hexdigit());

    // `preceded` takes a prefix parser, and if it succeeds, returns the result
    // of the body parser. In this case, it parses u{XXXX}.
    let parse_delimited_hex = preceded(
        char('u'),
        // `delimited` is like `preceded`, but it parses both a prefix and a suffix.
        // It returns the result of the middle parser. In this case, it parses
        // {XXXX}, where XXXX is 1 to 6 hex numerals, and returns XXXX
        delimited(char('{'), parse_hex, char('}')),
    );

    // `map_res` takes the result of a parser and applies a function that returns
    // a Result. In this case we take the hex bytes from parse_hex and attempt to
    // convert them to a u32.
    let parse_u32 = map_res(parse_delimited_hex, move |hex| u32::from_str_radix(hex, 16));

    // map_opt is like map_res, but it takes an Option instead of a Result. If
    // the function returns None, map_opt returns an error. In this case, because
    // not all u32 values are valid unicode code points, we have to fallibly
    // convert to char with from_u32.
    map_opt(parse_u32, std::char::from_u32).parse(input)
}

/// Parse an escaped character after a backslash.
///
/// Handles common escape sequences: `\n`, `\r`, `\t`, `\b`, `\f`,
/// `\\`, `\/`, `\"`, and Unicode escapes `\u{XXXX}`.
///
/// # Arguments
/// - `input`: Input string starting with `\`
///
/// # Returns
/// - `Ok((remaining, char))` with the parsed character
/// - `Err` if escape sequence is invalid
fn parse_escaped_char<'a, E>(input: &'a str) -> IResult<&'a str, char, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::num::ParseIntError>,
{
    preceded(
        char('\\'),
        // `alt` tries each parser in sequence, returning the result of
        // the first successful match
        alt((
            parse_unicode,
            // The `value` parser returns a fixed value (the first argument) if its
            // parser (the second argument) succeeds. In these cases, it looks for
            // the marker characters (n, r, t, etc) and returns the matching
            // character (\n, \r, \t, etc).
            value('\n', char('n')),
            value('\r', char('r')),
            value('\t', char('t')),
            value('\u{08}', char('b')),
            value('\u{0C}', char('f')),
            value('\\', char('\\')),
            value('/', char('/')),
            value('"', char('"')),
        )),
    )
    .parse(input)
}

/// Parse escaped whitespace (backslash followed by whitespace).
///
/// Whitespace after a backslash is ignored in strings. This parser
/// consumes `\` followed by one or more whitespace characters and
/// discards them.
///
/// # Arguments
/// - `input`: Input string
///
/// # Returns
/// - `Ok((remaining, whitespace_string))` with the matched whitespace
/// - `Err` if input doesn't start with `\` followed by whitespace
fn parse_escaped_whitespace<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, &'a str, E> {
    preceded(char('\\'), multispace1).parse(input)
}

/// Parse a literal (non-escaped) text segment.
///
/// Matches a non-empty sequence of characters that are not `\` or `"`.
/// This is the "normal" text in a string that doesn't contain escape sequences.
///
/// # Arguments
/// - `input`: Input string
///
/// # Returns
/// - `Ok((remaining, literal_text))` with the non-empty literal
/// - `Err` if no literal found (empty or starts with `\` or `"`)
fn parse_literal<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    // `is_not` parses a string of 0 or more characters that aren't one of the
    // given characters.
    let not_quote_slash = is_not("\"\\");

    // `verify` runs a parser, then runs a verification function on the output of
    // the parser. The verification function accepts out output only if it
    // returns true. In this case, we want to ensure that the output of is_not
    // is non-empty.
    verify(not_quote_slash, |s: &str| !s.is_empty()).parse(input)
}

/// A fragment of a string being parsed.
///
/// During string parsing, the input is broken into fragments of three types:
///
/// # Variants
///
/// - `Literal(&'a str)`: A non-empty run of non-escaped characters
/// - `EscapedChar(char)`: A single character from an escape sequence (e.g., `\n` → `\n`)
/// - `EscapedWS`: Escaped whitespace (backslash + whitespace, which is discarded)
///
/// These fragments are accumulated to build the final parsed string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StringFragment<'a> {
    /// A literal text segment with no escapes.
    Literal(&'a str),
    /// A character parsed from an escape sequence.
    EscapedChar(char),
    /// Escaped whitespace that should be discarded.
    EscapedWS,
}

/// Parse a single string fragment.
///
/// Attempts to parse one of:
/// - A literal (using `parse_literal`)
/// - An escaped character (using `parse_escaped_char`)
/// - Escaped whitespace (using `parse_escaped_whitespace`)
///
/// # Arguments
/// - `input`: Input string to parse
///
/// # Returns
/// - `Ok((remaining, fragment))` with the parsed fragment
/// - `Err` if no fragment could be parsed
fn parse_fragment<'a, E>(input: &'a str) -> IResult<&'a str, StringFragment<'a>, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::num::ParseIntError>,
{
    alt((
        // The `map` combinator runs a parser, then applies a function to the output
        // of that parser.
        map(parse_literal, StringFragment::Literal),
        map(parse_escaped_char, StringFragment::EscapedChar),
        value(StringFragment::EscapedWS, parse_escaped_whitespace),
    ))
    .parse(input)
}

/// Parse a complete string literal with escape sequence support.
///
/// Parses a string enclosed in double quotes, processing escape sequences
/// and accumulating fragments into the final `String` result.
///
/// # Syntax
///
/// Valid Lisp string literals:
/// ```lisp
/// "literal text"
/// "text with \n newlines"
/// "quotes: \"hello\""
/// "unicode: \u{2764}"
/// ```
///
/// # Arguments
/// - `input`: Input string starting with `"` (double quote)
///
/// # Returns
/// - `Ok((remaining, parsed_string))` with the parsed and unescaped string
/// - `Err` if the string is malformed or escape sequences are invalid
///
/// # Example
/// ```
/// use lisp::parse_string::parse_string;
/// # use nom::IResult;
/// let result = parse_string::<nom::error::Error<&str>>("\"hello\\nworld\"");
/// assert!(result.is_ok());
/// let (rem, s) = result.unwrap();
/// assert!(rem.is_empty());
/// assert_eq!(s, "hello\nworld");
/// ```
pub fn parse_string<'a, E>(input: &'a str) -> IResult<&'a str, String, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::num::ParseIntError>,
{
    // fold is the equivalent of iterator::fold. It runs a parser in a loop,
    // and for each output value, calls a folding function on each output value.
    let build_string = fold(
        0..,
        // Our parser function – parses a single string fragment
        parse_fragment,
        // Our init value, an empty string
        String::new,
        // Our folding function. For each fragment, append the fragment to the
        // string.
        |mut string, fragment| {
            match fragment {
                StringFragment::Literal(s) => string.push_str(s),
                StringFragment::EscapedChar(c) => string.push(c),
                StringFragment::EscapedWS => {}
            }
            string
        },
    );

    // Finally, parse the string. Note that, if `build_string` could accept a raw
    // " character, the closing delimiter " would never match. When using
    // `delimited` with a looping parser (like fold), be sure that the
    // loop won't accidentally match your closing delimiter!
    delimited(char('"'), build_string, char('"')).parse(input)
}
