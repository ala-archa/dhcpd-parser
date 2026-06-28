use std::iter::Peekable;

use crate::leases::parse_lease;
use crate::leases::Lease;
use crate::leases::LeaseKeyword;
use crate::leases::Leases;
pub use crate::leases::LeasesMethods;
use crate::lex::lex;
use crate::lex::LexItem;

/// A `host` declaration (static reservation), e.g.
/// `host name { hardware ethernet aa:bb:..; fixed-address 10.0.0.1; }`.
///
/// Host declarations appear both in `dhcpd.conf` and — when created via OMAPI —
/// in `dhcpd.leases`, so the parser must extract them without choking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Host {
    pub name: String,
    pub mac: Option<String>,
    /// `fixed-address` may list several addresses; dhcpd picks the one on the
    /// matching subnet.
    pub fixed_addresses: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserResult {
    pub leases: Leases,
    pub hosts: Vec<Host>,
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

/// Consume tokens until (and including) the matching `}` of an already-opened
/// block, honoring nesting. Used to skip declarations we don't model.
fn skip_braces<'l, T>(it: &mut Peekable<T>) -> Result<(), String>
where
    T: Iterator<Item = &'l LexItem>,
{
    let mut depth = 1usize;
    while depth > 0 {
        match it.next() {
            None => return Err("Unexpected EOF inside block".to_owned()),
            Some(LexItem::Paren('{')) => depth += 1,
            Some(LexItem::Paren('}')) => depth -= 1,
            _ => {}
        }
    }
    Ok(())
}

/// Skip a single unknown statement inside a `host` block: either up to the
/// terminating `;` or over a nested `{...}` block.
fn skip_host_statement<'l, T>(it: &mut Peekable<T>) -> Result<(), String>
where
    T: Iterator<Item = &'l LexItem>,
{
    loop {
        match it.peek().copied() {
            None => return Err("Unexpected EOF inside host block".to_owned()),
            Some(LexItem::Endl) => {
                it.next();
                return Ok(());
            }
            Some(LexItem::Paren('{')) => {
                it.next();
                return skip_braces(it);
            }
            Some(LexItem::Paren('}')) => return Ok(()),
            Some(_) => {
                it.next();
            }
        }
    }
}

fn parse_host<'l, T>(it: &mut Peekable<T>) -> Result<Host, String>
where
    T: Iterator<Item = &'l LexItem>,
{
    it.next(); // "host"
    let name = match it.next() {
        Some(LexItem::Word(w)) => w.clone(),
        other => return Err(format!("Expected host name, got {:?}", other)),
    };
    match it.next() {
        Some(LexItem::Paren('{')) => {}
        other => return Err(format!("Expected '{{' after host name, got {:?}", other)),
    }

    let mut mac = None;
    let mut fixed_addresses = Vec::new();

    loop {
        match it.peek().copied() {
            None => return Err("Unexpected EOF inside host block".to_owned()),
            Some(LexItem::Paren('}')) => {
                it.next();
                break;
            }
            Some(LexItem::Opt(LeaseKeyword::Hardware)) => {
                it.next(); // "hardware"
                it.next(); // hardware type ("ethernet")
                if let Some(LexItem::Word(m)) = it.peek().copied() {
                    mac = Some(m.to_lowercase());
                    it.next();
                }
                // Consume the rest up to the terminating ';'.
                while let Some(t) = it.peek().copied() {
                    let end = matches!(t, LexItem::Endl);
                    it.next();
                    if end {
                        break;
                    }
                }
            }
            Some(LexItem::Word(w)) if w.as_str() == "fixed-address" => {
                it.next();
                loop {
                    match it.peek().copied() {
                        None => return Err("Unexpected EOF in fixed-address".to_owned()),
                        Some(LexItem::Endl) => {
                            it.next();
                            break;
                        }
                        Some(LexItem::Word(x)) => {
                            for part in x.split(',') {
                                let p = part.trim();
                                if !p.is_empty() {
                                    fixed_addresses.push(p.to_owned());
                                }
                            }
                            it.next();
                        }
                        Some(_) => {
                            it.next();
                        }
                    }
                }
            }
            Some(_) => skip_host_statement(it)?,
        }
    }

    Ok(Host {
        name,
        mac,
        fixed_addresses,
    })
}

fn parse_lease_decl<'l, T>(it: &mut Peekable<T>, leases: &mut Leases) -> Result<(), String>
where
    T: Iterator<Item = &'l LexItem>,
{
    it.next(); // "lease"
    let ip = match it.next() {
        Some(v) => v.to_string(),
        None => return Err("IP address expected".to_owned()),
    };
    match it.next() {
        Some(LexItem::Paren('{')) => {}
        other => return Err(format!("Expected '{{' after lease IP, got {:?}", other)),
    }

    let mut lease = Lease {
        ip,
        ..Lease::default()
    };
    parse_lease(&mut lease, it)?;

    match it.peek().copied() {
        Some(LexItem::Paren('}')) => {
            it.next();
        }
        other => {
            return Err(format!("Expected '}}' to close lease, got '{:?}'", other));
        }
    }

    leases.push(lease);
    Ok(())
}

/// Tolerantly parse a sequence of declarations. Recognized: `lease` and `host`.
/// Anything else (subnet, group, shared-network, option, single statements…) is
/// skipped; `{...}` blocks are recursed into so nested `host` declarations are
/// still collected. With `in_braces`, returns when the matching `}` is consumed.
fn parse_declarations<'l, T>(
    it: &mut Peekable<T>,
    leases: &mut Leases,
    hosts: &mut Vec<Host>,
    in_braces: bool,
) -> Result<(), String>
where
    T: Iterator<Item = &'l LexItem>,
{
    loop {
        match it.peek().copied() {
            None => {
                if in_braces {
                    return Err("Unexpected EOF: unclosed '{'".to_owned());
                }
                return Ok(());
            }
            Some(LexItem::Paren('}')) => {
                it.next();
                if in_braces {
                    return Ok(());
                }
                // Stray closing brace at top level: ignore and continue.
            }
            Some(LexItem::Decl(ConfigKeyword::Lease)) => parse_lease_decl(it, leases)?,
            Some(LexItem::Word(w)) if w.as_str() == "host" => {
                let host = parse_host(it)?;
                hosts.push(host);
            }
            Some(LexItem::Endl) => {
                it.next();
            }
            _ => {
                // Unknown statement or block: skip its header up to ';' or '{'.
                // On '{', recurse so nested `host`/`lease` declarations are kept.
                loop {
                    match it.peek().copied() {
                        None => return Ok(()),
                        Some(LexItem::Endl) => {
                            it.next();
                            break;
                        }
                        Some(LexItem::Paren('{')) => {
                            it.next();
                            parse_declarations(it, leases, hosts, true)?;
                            break;
                        }
                        Some(LexItem::Paren('}')) => break,
                        Some(_) => {
                            it.next();
                        }
                    }
                }
            }
        }
    }
}

fn parse_config(tokens: Vec<LexItem>) -> Result<ParserResult, String> {
    let mut leases = Leases::new();
    let mut hosts = Vec::new();

    let mut it = tokens.iter().peekable();
    parse_declarations(&mut it, &mut leases, &mut hosts, false)?;

    Ok(ParserResult { leases, hosts })
}

pub fn parse<S>(input: S) -> Result<ParserResult, String>
where
    S: Into<String>,
{
    let tokens = lex(input).map_err(|err| format!("Lexer error: {err}"))?;
    parse_config(tokens)
}
