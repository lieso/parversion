use std::iter::Peekable;
use std::str::Chars;

use super::errors::Errors;

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    LParen,
    RParen,
    Str(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
    Symbol(String),
    Quote,
    Char(char),
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, Errors> {
    let mut tokens = Vec::new();
    let mut chars = source.chars().peekable();

    while let Some(&c) = chars.peek() {
        match c {
            c if c.is_whitespace() => {
                chars.next();
            }
            '(' => {
                chars.next();
                tokens.push(Token::LParen);
            }
            ')' => {
                chars.next();
                tokens.push(Token::RParen);
            }
            '"' => {
                chars.next();
                tokens.push(Token::Str(read_string(&mut chars)?));
            }
            '\'' => {
                chars.next();
                tokens.push(Token::Quote);
            }
            '#' if is_char_literal(&chars) => {
                tokens.push(Token::Char(read_char(&mut chars)?));
            }
            _ => {
                tokens.push(classify(read_atom(&mut chars)));
            }
        }
    }

    Ok(tokens)
}

fn read_string(chars: &mut Peekable<Chars<'_>>) -> Result<String, Errors> {
    let mut text = String::new();

    loop {
        match chars.next() {
            None => return Err(Errors::UnterminatedString),
            Some('"') => return Ok(text),
            Some(c) => text.push(c),
        }
    }
}

fn read_atom(chars: &mut Peekable<Chars<'_>>) -> String {
    let mut atom = String::new();

    while let Some(&c) = chars.peek() {
        if is_delimiter(c) {
            break;
        }
        atom.push(c);
        chars.next();
    }

    atom
}

fn is_delimiter(c: char) -> bool {
    c.is_whitespace() || matches!(c, '(' | ')' | '"' | '\'')
}

fn classify(atom: String) -> Token {
    match atom.as_str() {
        "#t" => return Token::Bool(true),
        "#f" => return Token::Bool(false),
        _ => {}
    }

    if let Ok(i) = atom.parse::<i64>() {
        return Token::Integer(i);
    }

    if let Ok(f) = atom.parse::<f64>() {
        return Token::Float(f);
    }

    Token::Symbol(atom)
}

fn is_char_literal(chars: &Peekable<Chars<'_>>) -> bool {
    let mut ahead = chars.clone();
    ahead.next();
    ahead.next() == Some('\\')
}

fn read_char(chars: &mut Peekable<Chars<'_>>) -> Result<char, Errors> {
    chars.next(); // '#'
    chars.next(); // '\'

    // The first character is taken unconditionally, so #\( #\) #\" #\' work.
    let first = chars.next().ok_or(Errors::UnexpectedEndOfInput)?;
    let mut name = String::from(first);

    while let Some(&c) = chars.peek() {
        if is_delimiter(c) {
            break;
        }
        name.push(c);
        chars.next();
    }

    let mut it = name.chars();
    match (it.next(), it.next()) {
        (Some(c), None) => Ok(c),
        _ => named_char(&name),
    }
}

fn named_char(name: &str) -> Result<char, Errors> {
    match name {
        "space" => Ok(' '),
        "newline" => Ok('\n'),
        "tab" => Ok('\t'),
        "return" => Ok('\r'),
        "nul" | "null" => Ok('\0'),
        "alarm" => Ok('\x07'),
        "backspace" => Ok('\x08'),
        "delete" => Ok('\x7f'),
        "escape" => Ok('\x1b'),
        _ => Err(Errors::InvalidCharLiteral(name.to_string())),
    }
}
