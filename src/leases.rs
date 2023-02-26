use std::collections::HashSet;
use std::iter::Peekable;
use std::ops::Index;

use crate::common::Date;
use crate::lex::LexItem;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingState {
    Active,
    Free,
    Abandoned,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeaseKeyword {
    ClientHostname,
    Ends,
    Hardware,
    Hostname,
    Starts,
    Uid,
    Tstp,
    Tsfp,
    Atsfp,
    Cltt,
    Binding,
    State,
    Next,
    Rewind,
    Set,
}

impl LeaseKeyword {
    pub fn to_string(&self) -> String {
        match self {
            LeaseKeyword::ClientHostname => "client-hostname".to_owned(),
            LeaseKeyword::Ends => "ends".to_owned(),
            LeaseKeyword::Hardware => "hardware".to_owned(),
            LeaseKeyword::Hostname => "hostname".to_owned(),
            LeaseKeyword::Starts => "starts".to_owned(),
            LeaseKeyword::Uid => "uid".to_owned(),
            LeaseKeyword::Tstp => "tstp".to_owned(),
            LeaseKeyword::Tsfp => "tsfp".to_owned(),
            LeaseKeyword::Atsfp => "atsfp".to_owned(),
            LeaseKeyword::Cltt => "cltt".to_owned(),
            LeaseKeyword::Binding => "binding".to_owned(),
            LeaseKeyword::State => "state".to_owned(),
            LeaseKeyword::Next => "next".to_owned(),
            LeaseKeyword::Rewind => "rewind".to_owned(),
            LeaseKeyword::Set => "set".to_owned(),
        }
    }

    pub fn from(s: &str) -> Result<LeaseKeyword, String> {
        match s {
            "client-hostname" => Ok(LeaseKeyword::ClientHostname),
            "ends" => Ok(LeaseKeyword::Ends),
            "hardware" => Ok(LeaseKeyword::Hardware),
            "hostname" => Ok(LeaseKeyword::Hostname),
            "starts" => Ok(LeaseKeyword::Starts),
            "tstp" => Ok(LeaseKeyword::Tstp),
            "tsfp" => Ok(LeaseKeyword::Tsfp),
            "atsfp" => Ok(LeaseKeyword::Atsfp),
            "cltt" => Ok(LeaseKeyword::Cltt),
            "uid" => Ok(LeaseKeyword::Uid),
            "binding" => Ok(LeaseKeyword::Binding),
            "state" => Ok(LeaseKeyword::State),
            "next" => Ok(LeaseKeyword::Next),
            "rewind" => Ok(LeaseKeyword::Rewind),
            "set" => Ok(LeaseKeyword::Set),
            _ => Err(format!("'{}' is not a recognized lease option", s)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeaseDates {
    pub starts: Option<Date>,
    pub ends: Option<Date>,
    pub tstp: Option<Date>,
    pub tsfp: Option<Date>,
    pub atsfp: Option<Date>,
    pub cltt: Option<Date>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hardware {
    pub h_type: String,
    pub mac: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LeasesField {
    ClientHostname,
    Hostname,
    LeasedIP,
    MAC,
}

impl LeasesField {
    fn value_getter(&self) -> Box<dyn Fn(&Lease) -> Option<String>> {
        match &self {
            LeasesField::ClientHostname => {
                Box::new(|l: &Lease| -> Option<String> { l.client_hostname.clone() })
            }
            LeasesField::Hostname => Box::new(|l: &Lease| -> Option<String> { l.hostname.clone() }),
            LeasesField::LeasedIP => Box::new(|l: &Lease| -> Option<String> { Some(l.ip.clone()) }),
            LeasesField::MAC => Box::new(|l: &Lease| -> Option<String> {
                match &l.hardware {
                    Some(h) => Some(h.mac.clone()),
                    None => None,
                }
            }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Leases(Vec<Lease>);

impl Index<usize> for Leases {
    type Output = Lease;

    fn index(&self, i: usize) -> &Self::Output {
        &self.0[i]
    }
}

pub trait LeasesMethods {
    fn all(&self) -> Vec<Lease>;

    #[deprecated(since = "0.4.3", note = "any filtering logic should be done by user")]
    fn active_by<S: AsRef<str>>(
        &self,
        field_name: LeasesField,
        value: S,
        active_at: Date,
    ) -> Option<Lease>;

    #[deprecated(since = "0.4.3", note = "any filtering logic should be done by user")]
    fn by_leased<S: AsRef<str>>(&self, ip: S) -> Option<Lease>;
    #[deprecated(since = "0.4.3", note = "any filtering logic should be done by user")]
    fn by_leased_all<S: AsRef<str>>(&self, ip: S) -> Vec<Lease>;

    #[deprecated(since = "0.4.3", note = "any filtering logic should be done by user")]
    fn by_mac<S: AsRef<str>>(&self, mac: S) -> Option<Lease>;
    #[deprecated(since = "0.4.3", note = "any filtering logic should be done by user")]
    fn by_mac_all<S: AsRef<str>>(&self, mac: S) -> Vec<Lease>;

    #[deprecated(since = "0.4.3", note = "any filtering logic should be done by user")]
    fn active_by_hostname<S: AsRef<str>>(&self, hostname: S, active_at: Date) -> Option<Lease>;
    #[deprecated(since = "0.4.3", note = "any filtering logic should be done by user")]
    fn by_hostname_all<S: AsRef<str>>(&self, hostname: S) -> Vec<Lease>;

    #[deprecated(since = "0.4.3", note = "any filtering logic should be done by user")]
    fn active_by_client_hostname<S: AsRef<str>>(
        &self,
        hostname: S,
        active_at: Date,
    ) -> Option<Lease>;
    #[deprecated(since = "0.4.3", note = "any filtering logic should be done by user")]
    fn by_client_hostname_all<S: AsRef<str>>(&self, hostname: S) -> Vec<Lease>;

    fn new() -> Leases;
    fn push(&mut self, l: Lease);
    fn hostnames(&self) -> HashSet<String>;
    fn client_hostnames(&self) -> HashSet<String>;
}

impl LeasesMethods for Leases {
    fn all(&self) -> Vec<Lease> {
        self.0.clone()
    }

    /// Returns a lease by some field and it's value if it exists.
    ///
    /// The lease has to be active:
    ///
    /// - `active_at` is between it's `starts` and `ends` datetime
    /// - is not `abandoned`
    /// - no active leases that match the field value exist after it
    fn active_by<S: AsRef<str>>(
        &self,
        field: LeasesField,
        value: S,
        active_at: Date,
    ) -> Option<Lease> {
        let expected_val = value.as_ref();
        let get_val = field.value_getter();

        let mut ls = self.0.clone();
        ls.reverse();

        for l in ls {
            if l.is_active_at(active_at) && l.binding_state != BindingState::Abandoned {
                let val = get_val(&l);
                if val.is_some() && val.unwrap() == expected_val {
                    return Some(l);
                }
            }
        }

        None
    }

    fn by_leased<S: AsRef<str>>(&self, ip: S) -> Option<Lease> {
        let mut ls = self.0.clone();
        ls.reverse();

        for l in ls {
            if l.ip == ip.as_ref() {
                return Some(l);
            }
        }

        None
    }

    fn by_leased_all<S: AsRef<str>>(&self, ip: S) -> Vec<Lease> {
        let mut result = Vec::new();
        let ls = self.0.clone();

        for l in ls {
            if l.ip == ip.as_ref() {
                result.push(l);
            }
        }

        return result;
    }

    fn by_mac<S: AsRef<str>>(&self, mac: S) -> Option<Lease> {
        let mut ls = self.0.clone();
        ls.reverse();

        for l in ls {
            let hw = l.hardware.as_ref();
            if hw.is_some() && hw.unwrap().mac == mac.as_ref() {
                return Some(l);
            }
        }

        None
    }

    fn by_mac_all<S: AsRef<str>>(&self, mac: S) -> Vec<Lease> {
        let mut result = Vec::new();
        let ls = self.0.clone();

        for l in ls {
            let hw = l.hardware.as_ref();
            if hw.is_some() && hw.unwrap().mac == mac.as_ref() {
                result.push(l);
            }
        }

        return result;
    }

    fn active_by_hostname<S: AsRef<str>>(&self, hostname: S, active_at: Date) -> Option<Lease> {
        #[allow(deprecated)]
        self.active_by(LeasesField::Hostname, hostname, active_at)
    }

    fn by_hostname_all<S: AsRef<str>>(&self, hostname: S) -> Vec<Lease> {
        let mut res = Vec::new();
        let ls = self.0.clone();
        let hn_s = hostname.as_ref();

        for l in ls {
            let hn = l.hostname.as_ref();
            if hn.is_some() && hn.unwrap() == hn_s {
                res.push(l);
            }
        }

        res
    }

    fn active_by_client_hostname<S: AsRef<str>>(
        &self,
        hostname: S,
        active_at: Date,
    ) -> Option<Lease> {
        #[allow(deprecated)]
        self.active_by(LeasesField::ClientHostname, hostname, active_at)
    }

    fn by_client_hostname_all<S: AsRef<str>>(&self, hostname: S) -> Vec<Lease> {
        let mut res = Vec::new();
        let ls = self.0.clone();
        let hn_s = hostname.as_ref();

        for l in ls {
            let hn = l.client_hostname.as_ref();
            if hn.is_some() && hn.unwrap() == hn_s {
                res.push(l);
            }
        }

        res
    }

    fn new() -> Leases {
        Leases(Vec::new())
    }

    fn push(&mut self, l: Lease) {
        self.0.push(l);
    }

    fn hostnames(&self) -> HashSet<String> {
        let mut res = HashSet::new();
        let ls = self.0.clone();

        for l in ls {
            if l.hostname.is_some() {
                res.insert(l.hostname.unwrap());
            }
        }

        return res;
    }

    fn client_hostnames(&self) -> HashSet<String> {
        let mut res = HashSet::new();
        let ls = self.0.clone();

        for l in ls {
            if l.client_hostname.is_some() {
                res.insert(l.client_hostname.unwrap());
            }
        }

        return res;
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lease {
    pub ip: String,
    pub dates: LeaseDates,
    pub hardware: Option<Hardware>,
    pub uid: Option<String>,
    pub client_hostname: Option<String>,
    pub hostname: Option<String>,
    pub binding_state: BindingState,
    pub next_binding_state: Option<BindingState>,
    pub rewind_binding_state: Option<BindingState>,
    pub vendor_class_identifier: Option<String>,
}

impl Lease {
    pub fn new() -> Lease {
        Lease {
            ip: "localhost".to_owned(),
            dates: LeaseDates {
                starts: None,
                ends: None,
                tstp: None,
                tsfp: None,
                atsfp: None,
                cltt: None,
            },
            hardware: None,
            uid: None,
            client_hostname: None,
            hostname: None,
            binding_state: BindingState::Free,
            next_binding_state: None,
            rewind_binding_state: None,
            vendor_class_identifier: None,
        }
    }

    pub fn is_active_at(&self, when: Date) -> bool {
        if self.dates.starts.is_some() && self.dates.starts.unwrap() > when {
            return false;
        }

        if self.dates.ends.is_some() && self.dates.ends.unwrap() < when {
            return false;
        }

        return true;
    }
}

pub fn parse_date<'l, T: Iterator<Item = &'l LexItem>>(
    iter: &mut Peekable<T>,
    name: &str,
) -> Result<crate::common::Date, String> {
    let weekday = match iter.peek() {
        Some(v) => v.to_string(),
        None => return Err(format!("Weekday for {:?} date expected", name)),
    };
    iter.next();
    let date = match iter.peek() {
        Some(v) => v.to_string(),
        None => return Err(format!("Date for {:?} date expected", name)),
    };
    iter.next();
    let time = match iter.peek() {
        Some(v) => v.to_string(),
        None => return Err(format!("Time for {:?} date expected", name)),
    };
    iter.next();
    let tz = match iter.peek() {
        Some(v) => v.to_string(),
        None => return Err(format!("Timezone for {:?} date expected", name)),
    };
    if tz != LexItem::Endl.to_string() {
        iter.next();
        match iter.peek() {
            None => {
                return Err(format!(
                    "Semicolon after timezone for {:?} date expected",
                    name
                ))
            }
            Some(LexItem::Endl) => (),
            Some(s) => {
                return Err(format!(
                    "Expected semicolon after timezone for {:?} date, found {:?}",
                    name, s
                ))
            }
        }
    }

    Date::from(weekday, date, time)
}

pub fn parse_binding_state<'l, T: Iterator<Item = &'l LexItem>>(
    iter: &mut Peekable<T>,
) -> Result<BindingState, String> {
    iter.next();
    if iter.peek() != Some(&&LexItem::Opt(LeaseKeyword::State)) {
        return Err("Expected 'state' after 'binding'".to_owned());
    }

    iter.next();
    let r = if let Some(LexItem::Word(w)) = iter.peek() {
        match w.as_str() {
            "active" => BindingState::Active,
            "free" => BindingState::Free,
            "abandoned" => BindingState::Abandoned,
            _ => return Err(format!("Expected binding value, found {:?}", iter.peek())),
        }
    } else {
        return Err(format!("Expected binding value, found {:?}", iter.peek()));
    };

    iter.next();
    if iter.peek() != Some(&&LexItem::Endl) {
        return Err(format!("Expected semisolon, found {:?}", iter.peek()));
    }

    Ok(r)
}

pub fn parse_lease<'l, T: Iterator<Item = &'l LexItem>>(
    lease: &mut Lease,
    iter: &mut Peekable<T>,
) -> Result<(), String> {
    while let Some(&nc) = iter.peek() {
        match nc {
            LexItem::Opt(LeaseKeyword::Starts) => {
                iter.next();
                lease.dates.starts.replace(parse_date(iter, "start")?);
            }
            LexItem::Opt(LeaseKeyword::Ends) => {
                iter.next();
                lease.dates.ends.replace(parse_date(iter, "end")?);
            }
            LexItem::Opt(LeaseKeyword::Tstp) => {
                iter.next();
                lease.dates.tstp.replace(parse_date(iter, "tstp")?);
            }
            LexItem::Opt(LeaseKeyword::Tsfp) => {
                iter.next();
                lease.dates.tsfp.replace(parse_date(iter, "tsfp")?);
            }
            LexItem::Opt(LeaseKeyword::Atsfp) => {
                iter.next();
                lease.dates.atsfp.replace(parse_date(iter, "atsfp")?);
            }
            LexItem::Opt(LeaseKeyword::Cltt) => {
                iter.next();
                lease.dates.cltt.replace(parse_date(iter, "cltt")?);
            }
            LexItem::Opt(LeaseKeyword::Hardware) => {
                iter.next();
                let h_type = match iter.peek() {
                    Some(v) => v.to_string(),
                    None => return Err("Hardware type expected".to_owned()),
                };
                iter.next();
                let mac = match iter.peek() {
                    Some(v) => v.to_string(),
                    None => return Err("MAC address expected".to_owned()),
                };
                iter.next();
                if iter.peek() != Some(&&LexItem::Endl) {
                    return Err("Semicolon expected authoring-byte-order".to_owned());
                }

                lease.hardware.replace(Hardware { h_type, mac });
            }
            LexItem::Opt(LeaseKeyword::Uid) => {
                iter.next();
                let v = match iter.peek() {
                    Some(v) => v.to_string(),
                    None => return Err("Client identifier expected".to_owned()),
                };
                lease.uid.replace(v);

                iter.next();
                if iter.peek() != Some(&&LexItem::Endl) {
                    return Err("Semicolon expected authoring-byte-order".to_owned());
                }
            }
            LexItem::Opt(LeaseKeyword::ClientHostname) => {
                iter.next();
                let v = match iter.peek() {
                    Some(v) => v.to_string(),
                    None => return Err("Client hostname expected".to_owned()),
                };
                lease.client_hostname.replace(unquote(v));

                iter.next();
                if iter.peek() != Some(&&LexItem::Endl) {
                    return Err("Semicolon expected authoring-byte-order".to_owned());
                }
            }
            LexItem::Opt(LeaseKeyword::Binding) => lease.binding_state = parse_binding_state(iter)?,
            LexItem::Opt(LeaseKeyword::Next) => {
                iter.next();
                if iter.peek() == Some(&&LexItem::Opt(LeaseKeyword::Binding)) {
                    lease.next_binding_state = Some(parse_binding_state(iter)?)
                } else {
                    return Err(format!("Expected 'binding' after 'next'"));
                }
            }
            LexItem::Opt(LeaseKeyword::Rewind) => {
                iter.next();
                if iter.peek() == Some(&&LexItem::Opt(LeaseKeyword::Binding)) {
                    lease.rewind_binding_state = Some(parse_binding_state(iter)?)
                } else {
                    return Err(format!("Expected 'binding' after 'rewind'"));
                }
            }
            LexItem::Opt(LeaseKeyword::Hostname) => {
                iter.next();
                let v = match iter.peek() {
                    Some(v) => v.to_string(),
                    None => return Err("Hostname expected".to_owned()),
                };
                lease.hostname.replace(unquote(v));

                iter.next();
                if iter.peek() != Some(&&LexItem::Endl) {
                    return Err("Semicolon expected authoring-byte-order".to_owned());
                }
            }
            LexItem::Opt(LeaseKeyword::Set) => {
                iter.next();
                let name = if let Some(LexItem::Word(w)) = iter.peek() {
                    w
                } else {
                    return Err(format!("Value name expected after 'set'"));
                };

                iter.next();
                if Some(&&LexItem::Word("=".to_string())) != iter.peek() {
                    return Err(format!("'=' expected after 'set VALUE'"));
                }

                iter.next();
                let value = if let Some(LexItem::Word(w)) = iter.peek() {
                    w
                } else {
                    return Err(format!("Value name expected after '='"));
                };

                iter.next();
                if iter.peek() != Some(&&LexItem::Endl) {
                    return Err(format!("Expected semisolon, found {:?}", iter.peek()));
                }

                match name.as_str() {
                    "vendor-class-identifier" => {
                        let _ = lease.vendor_class_identifier.replace(value.clone());
                    }
                    // Skip unknown values
                    _ => (),
                }
            }
            LexItem::Paren('}') => {
                return Ok(());
            }
            _ => {
                return Err(format!(
                    "Unexpected option '{}'",
                    iter.peek().unwrap().to_string()
                ));
            }
        }
        iter.next();
    }

    Ok(())
}

fn unquote(hn: String) -> String {
    hn.replace("\"", "")
}
