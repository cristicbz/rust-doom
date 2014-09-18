use std::{char};
use std::vec::Vec;
use std::string::String;

pub fn shlex(input : &str) -> Result<Vec<String>, String> {
    let mut argv = Vec::<String>::new();
    let mut buf = String::new();
    let mut state = Normal;

    for input_char in input.chars() {
        let (new_state, action) = match state {
            Normal => shlex_normal(input_char),
            SingleQuote => shlex_single_quote(input_char),
            DoubleQuote => shlex_double_quote(input_char),
            _ => (match state {
                     NormalEscape => Normal,
                     SingleQuoteEscape => SingleQuote,
                     DoubleQuoteEscape => DoubleQuote,
                     _ => unreachable!(),
                  }, Append(try!(shlex_escape(input_char))))
        };

        match action {
            Skip => {},
            Append(new_char) => buf.push_char(new_char),
            _ => {
                if action == SplitAlways || !buf.is_empty()  {
                    argv.push(buf);
                    buf = String::new();
                }
            }
        };
        state = new_state;
    }

    match state {
        Normal => {
            if !buf.is_empty() { argv.push(buf); }
            Ok(argv)
        },
        NormalEscape | SingleQuoteEscape |  DoubleQuoteEscape => {
            Err(String::from_str("Unfinished escape sequence."))
        },
        SingleQuote | DoubleQuote => {
            Err(String::from_str("Unclosed single or double quote."))
        },
    }
}

enum ShlexState {
    Normal,
    NormalEscape,
    SingleQuote,
    SingleQuoteEscape,
    DoubleQuote,
    DoubleQuoteEscape,
}

#[deriving(PartialEq)]
enum ShlexAction {
    Append(char),
    SplitNonEmpty,
    SplitAlways,
    Skip
}

fn shlex_normal(input_char : char) -> (ShlexState, ShlexAction) {
    if char::is_whitespace(input_char) {
        (Normal, SplitNonEmpty)
    } else if input_char == '\\' {
        (NormalEscape, Skip)
    } else if input_char == '\'' {
        (SingleQuote, Skip)
    } else if input_char == '\"' {
        (DoubleQuote, Skip)
    } else {
        (Normal, Append(input_char))
    }
}

fn shlex_single_quote(input_char : char) -> (ShlexState, ShlexAction) {
    if input_char == '\'' {
        (Normal, Skip)
    } else if input_char == '\\' {
        (SingleQuoteEscape, Skip)
    } else {
        (SingleQuote, Append(input_char))
    }
}

fn shlex_double_quote(input_char : char) -> (ShlexState, ShlexAction) {
    if input_char == '\"' {
        (Normal, Skip)
    } else if input_char == '\\' {
        (DoubleQuoteEscape, Skip)
    } else {
        (DoubleQuote, Append(input_char))
    }
}

fn shlex_escape(input_char : char) -> Result<char, String> {
    match input_char {
        't'  => Ok('\t'),
        'n'  => Ok('\n'),
        ' '  => Ok(' '),
        '\\' => Ok('\\'),
        '"'  => Ok('"'),
        '\'' => Ok('\''),
        _    => Err(format!("Unknown escape sequence '\\{}'.",
                            input_char))
    }
}

