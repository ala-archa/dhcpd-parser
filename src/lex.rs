use std::fmt;
use std::iter::Peekable;

use crate::leases::LeaseKeyword;
use crate::parser::ConfigKeyword;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexItem {
    Paren(char),
    Endl,
    Word(String),
    Opt(LeaseKeyword),
    Decl(ConfigKeyword),
}

impl fmt::Display for LexItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LexItem::Paren(v) => v.fmt(f),
            LexItem::Word(v) => v.fmt(f),
            LexItem::Opt(v) => write!(f, "{}", v),
            LexItem::Decl(v) => write!(f, "{}", v),
            LexItem::Endl => write!(f, ";"),
        }
    }
}

pub fn lex<S>(input: S) -> Result<Vec<LexItem>, String>
where
    S: Into<String>,
{
    let mut result = Vec::new();

    let input_str = input.into();

    let mut it = input_str.chars().peekable();
    while let Some(&c) = it.peek() {
        match c {
            '(' | ')' | '[' | ']' | '{' | '}' => {
                result.push(LexItem::Paren(c));
                it.next();
            }
            '#' => {
                while let Some(&c) = it.peek() {
                    if c == '\n' {
                        break;
                    }
                    it.next();
                }
            }
            ' ' | '\n' | '\t' => {
                it.next();
            }
            ';' => {
                result.push(LexItem::Endl);
                it.next();
            }
            _ => {
                let w = get_word(&mut it);
                let kw = ConfigKeyword::from(&w);
                if let Ok(kw) = kw {
                    result.push(LexItem::Decl(kw));
                } else {
                    let kw = LeaseKeyword::from(&w);
                    if let Ok(kw) = kw {
                        result.push(LexItem::Opt(kw));
                    } else {
                        result.push(LexItem::Word(w));
                    }
                }
            }
        }
    }
    Ok(result)
}

fn get_word<T: Iterator<Item = char>>(iter: &mut Peekable<T>) -> String {
    let mut word = String::new();

    while let Some(&nc) = iter.peek() {
        if nc.is_whitespace() || nc == ';' {
            break;
        }

        word.push(nc);
        iter.next();
    }
    word
}
