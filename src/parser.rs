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

impl std::fmt::Display for ConfigKeyword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigKeyword::Lease => write!(f, "lease"),
        }
    }
}

impl ConfigKeyword {
    pub fn from(s: &str) -> Result<ConfigKeyword, String> {
        match s {
            "lease" => Ok(ConfigKeyword::Lease),
            _ => Err(format!("'{}' declaration is not supported", s)),
        }
    }
}

fn parse_config(tokens: Vec<LexItem>) -> Result<ParserResult, String> {
    let mut leases = Leases::new();
    let lease = Lease::default();

    let mut it = tokens.iter().peekable();

    while let Some(token) = it.peek() {
        match token {
            LexItem::Decl(ConfigKeyword::Lease) => {
                if lease != Lease::default() {
                    leases.push(lease.clone());
                }

                let mut lease = Lease::default();
                // ip-address
                it.next();
                lease.ip = match it.peek() {
                    Some(v) => v.to_string(),
                    None => return Err("IP address expected".to_owned()),
                };

                // left curly brace
                it.next();
                assert_eq!(*it.peek().unwrap(), &LexItem::Paren('{'));

                // statements for the lease
                it.next();
                parse_lease(&mut lease, &mut it)?;

                // right curly brace
                if it.peek().is_none() || *it.peek().unwrap() != &LexItem::Paren('}') {
                    return Err(format!(
                        "Expected end of section with '}}', got '{:?}'",
                        it.peek(),
                    ));
                }

                leases.push(lease.clone());
                it.next();
            }
            LexItem::Word(w) => match w.as_str() {
                "authoring-byte-order" | "server-duid" => {
                    it.next();
                    if it.peek().is_none() {
                        return Err("Value".to_owned());
                    }
                    it.next();
                    if it.peek() != Some(&&LexItem::Endl) {
                        return Err("Semicolon expected".to_owned());
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
    parse_config(tokens)
}
