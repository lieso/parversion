use std::iter::Peekable;
use std::str::Chars;
use std::vec::IntoIter;

use super::lexer::Token;
use super::errors::Errors;

pub enum Expr {
    Integer(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Symbol(String),
    List(Vec<Expr>),
}

pub fn parse(tokens: Vec<Token>) -> Result<Expr, Errors> {
    let mut tokens = tokens.into_iter().peekable();

    let expr = parse_expr(&mut tokens)?;

    Ok(expr)
}

fn parse_expr(tokens: &mut Peekable<IntoIter<Token>>) -> Result<Expr, Errors> {
    match tokens.next() {
        None => Err(Errors::UnexpectedEndOfInput),
        Some(Token::RParen) => Err(Errors::UnexpectedCloseParen),
        Some(Token::LParen) => {
            let mut items = Vec::new();

            loop {
                match tokens.peek() {
                    None => return Err(Errors::UnexpectedEndOfInput),
                    Some(Token::RParen) => {
                        tokens.next();
                        return Ok(Expr::List(items));
                    }
                    Some(_) => items.push(parse_expr(tokens)?),
                }
            }
        }
        Some(Token::Integer(i)) => Ok(Expr::Integer(i)),
        Some(Token::Float(f)) => Ok(Expr::Float(f)),
        Some(Token::Str(s)) => Ok(Expr::Str(s)),
        Some(Token::Bool(b)) => Ok(Expr::Bool(b)),
        Some(Token::Symbol(s)) => Ok(Expr::Symbol(s)),
    }
}
