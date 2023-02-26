use crate::leases::parse_lease;
use crate::leases::Lease;
use crate::leases::Leases;
pub use crate::leases::LeasesMethods;
use crate::lex::lex;
use crate::lex::LexItem;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserResult {
    pub leases: Leases,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigKeyword {
    Lease,
}

impl ConfigKeyword {
    pub fn to_string(&self) -> String {
        match self {
            &ConfigKeyword::Lease => "lease".to_owned(),
        }
    }

    pub fn from(s: &str) -> Result<ConfigKeyword, String> {
        match s {
            "lease" => Ok(ConfigKeyword::Lease),
            _ => Err(format!("'{}' declaration is not supported", s)),
        }
    }
}

fn parse_config(tokens: Vec<LexItem>) -> Result<ParserResult, String> {
    let mut leases = Leases::new();
    let lease = Lease::new();

    let mut it = tokens.iter().peekable();

    while let Some(token) = it.peek() {
        match token {
            LexItem::Decl(ConfigKeyword::Lease) => {
                if lease != Lease::new() {
                    leases.push(lease.clone());
                }

                let mut lease = Lease::new();
                // ip-address
                it.next();
                lease.ip = match it.peek() {
                    Some(v) => v.to_string(),
                    None => return Err(format!("IP address expected")),
                };

                // left curly brace
                it.next();
                assert_eq!(it.peek().unwrap().to_owned(), &LexItem::Paren('{'));

                // statements for the lease
                it.next();
                parse_lease(&mut lease, &mut it)?;

                // right curly brace
                if it.peek().is_none() || it.peek().unwrap().to_owned() != &LexItem::Paren('}') {
                    return Err(format!(
                        "Expected end of section with '}}', got '{:?}'",
                        it.peek(),
                    ));
                }

                leases.push(lease.clone());
                it.next();
            }
            LexItem::Word(w) => match w.as_str() {
                "authoring-byte-order" => {
                    it.next();
                    if it.peek().is_none() {
                        return Err("Value for authoring-byte-order".to_owned());
                    }
                    it.next();
                    if it.peek() != Some(&&LexItem::Endl) {
                        return Err("Semicolon expected authoring-byte-order".to_owned());
                    }
                    it.next();
                }
                _ => {
                    return Err(format!("Unexpected {:?}", it.peek()));
                }
            },
            _ => {
                return Err(format!("Unexpected {:?}", it.peek()));
            }
        }
    }

    Ok(ParserResult { leases })
}

pub fn parse<S>(input: S) -> Result<ParserResult, String>
where
    S: Into<String>,
{
    let tokens = lex(input).unwrap();
    return parse_config(tokens);
}
