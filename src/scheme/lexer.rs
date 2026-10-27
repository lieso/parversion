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
        if c.is_whitespace() || c == '(' || c == ')' || c == '"' {
            break;
        }
        atom.push(c);
        chars.next();
    }

    atom
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
