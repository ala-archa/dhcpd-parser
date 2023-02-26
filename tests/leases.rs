extern crate dhcpd_parser;

use crate::dhcpd_parser::common::Date;
use crate::dhcpd_parser::parser;
use crate::dhcpd_parser::parser::LeasesMethods;

#[test]
fn basic_test() {
    let res = parser::parse(
        "
    lease 192.0.0.2 {

    }"
        .to_string(),
    );
    assert!(res.is_ok());
}

#[test]
fn dates_test() {
    let res = parser::parse(
        "lease 255.254.253.252 {
        starts 2 2019/01/01 22:00:00 UTC;
        ends 2 2019/01/01 22:00:00 UTC;
    }"
        .to_string(),
    );
    assert!(res.is_ok());
}

#[test]
fn all_options_test() {
    let res = parser::parse(
        "
    lease 192.168.0.2 {
        starts 2 2019/01/01 22:00:00 UTC;
        ends 2 2019/01/01 22:00:00 UTC;
        hardware type 11:11:11:11:11:11;
        uid Client1;
        client-hostname \"CLIENTHOSTNAME\";
        hostname \"TESTHOSTNAME\";
    }",
    );

    assert!(res.is_ok());
}

#[test]
fn multiple_leases_test() {
    let res = parser::parse(
        "
    lease 192.168.0.2 {
        starts 2 2019/01/01 22:00:00 UTC;
        ends 2 2019/01/01 22:00:00 UTC;
        hardware type 11:11:11:11:11:11;
        uid Client1;
        client-hostname \"CLIENTHOSTNAME\";
        hostname \"TESTHOSTNAME\";
    }

    lease 192.168.0.3 {
        starts 1 1985/01/01 00:00:00 UTC;
        hardware type 22:22:22:22:22:22;
        uid Client2;
        hostname \"TESTHOSTNAME\";
    }
    ",
    );

    assert!(res.is_ok());

    let leases = res.unwrap().leases;
    assert_eq!(leases[0].hostname.as_ref().unwrap(), "TESTHOSTNAME");
    assert_eq!(
        leases[1].dates.starts.unwrap().to_string(),
        "Monday 1985/01/01 00:00:00"
    );
    assert!(leases[1].dates.ends.is_none());
}

#[test]
fn invalid_format_test() {
    let res = parser::parse(
        "
    lease 192.0.0.2 {

    ",
    );
    assert!(res.is_err());
}

#[test]
fn invalid_date_format_test() {
    let res = parser::parse(
        "
    lease 192.0.0.2 {
        starts 2 2019-01-02 00:00:00;
    }",
    );
    assert!(res.is_err());
}

#[test]
fn is_active_test() {
    let res = parser::parse(
        "
    lease 192.168.0.2 {
        starts 2 2019/01/01 22:00:00 UTC;
        ends 2 2019/01/01 23:00:00 UTC;
        hardware type 11:11:11:11:11:11;
        uid Client1;
        client-hostname \"CLIENTHOSTNAME\";
        hostname \"TESTHOSTNAME\";
    }

    lease 192.168.0.3 {
        starts 1 1985/01/02 00:00:00 UTC;
        hardware type 22:22:22:22:22:22;
        uid Client2;
        hostname \"TESTHOSTNAME\";
    }
    ",
    );

    let leases = res.unwrap().leases;

    assert!(leases[0].is_active_at(Date::from("2", "2019/01/01", "22:30:00").unwrap()));

    assert!(!leases[1].is_active_at(Date::from("1", "1985/01/01", "22:30:00").unwrap()),);

    assert!(!leases[0].is_active_at(Date::from("2", "2019/01/01", "21:59:00").unwrap()),);

    assert!(!leases[0].is_active_at(
        Date::from(
            "2".to_string(),
            "2019/01/01".to_string(),
            "23:59:00".to_string()
        )
        .unwrap()
    ),);
}

#[test]
fn hostnames_test() {
    let res = parser::parse(
        "
    lease 192.168.0.2 {
        starts 2 2019/01/01 22:00:00 UTC;
        ends 2 2019/01/01 23:00:00 UTC;
        hardware type 11:11:11:11:11:11;
        uid Client1;
        client-hostname \"CLIENTHOSTNAME\";
        hostname \"TESTHOSTNAME\";
    }

    lease 192.168.0.3 {
        starts 1 1985/01/02 00:00:00 UTC;
        ends 1 1985/01/02 02:00:00 UTC;
        hardware type 22:22:22:22:22:22;
        uid Client2;
        hostname \"TESTHOSTNAME\";
    }
    ",
    );

    let leases = res.unwrap().leases;

    assert_eq!(
        leases.hostnames(),
        ["TESTHOSTNAME".to_owned()].iter().cloned().collect()
    );
}

#[test]
fn client_hostnames_test() {
    let res = parser::parse(
        "
    lease 192.168.0.2 {
        starts 2 2019/01/01 22:00:00 UTC;
        ends 2 2019/01/01 23:00:00 UTC;
        hardware type 11:11:11:11:11:11;
        uid Client1;
        client-hostname \"CLIENTHOSTNAME\";
        hostname \"TESTHOSTNAME\";
    }

    lease 192.168.0.3 {
        starts 1 1985/01/02 00:00:00 UTC;
        ends 1 1985/01/02 02:00:00 UTC;
        hardware type 22:22:22:22:22:22;
        uid Client2;
        hostname \"TESTHOSTNAME\";
        client-hostname \"HN\";
    }

    lease 192.168.0.3 {
        starts 1 1986/01/02 00:00:00 UTC;
        ends 1 1986/12/02 02:00:00 UTC;
        hardware type 22:22:22:22:22:22;
        uid Client2;
        client-hostname \"HN\";
    }
    ",
    );

    let leases = res.unwrap().leases;

    assert_eq!(
        leases.client_hostnames(),
        ["CLIENTHOSTNAME".to_owned(), "HN".to_owned()]
            .iter()
            .cloned()
            .collect()
    );
}

#[test]
fn client_hostnames_with_comments_test() {
    let res = parser::parse(
        "
    # comment
    lease 192.168.0.2 { # comment
        starts 2 2019/01/01 22:00:00 UTC;
        #comment
        ends 2 2019/01/01 23:00:00 UTC;
        hardware type 11:11:11:11:11:11;
        uid Client1;
        client-hostname \"CLIENTHOSTNAME\";
        hostname \"TESTHOSTNAME\"; # comment
    }
    # comment",
    );

    let leases = res.unwrap().leases;

    assert_eq!(
        leases.client_hostnames(),
        ["CLIENTHOSTNAME".to_owned()].iter().cloned().collect()
    );
}

#[test]
fn real_world_test() {
    let res = parser::parse(
        r#"
# The format of this file is documented in the dhcpd.leases(5) manual page.
# This lease file was written by isc-dhcp-4.4.1

# authoring-byte-order entry is generated, DO NOT DELETE
authoring-byte-order little-endian;

lease 10.11.4.50 {
  starts 3 2023/02/22 21:15:36;
  ends 4 2023/02/23 09:15:36;
  tstp 4 2023/02/23 09:15:36;
  cltt 3 2023/02/22 21:23:36;
  binding state free;
  hardware ethernet 5a:64:bf:76:34:58;
  uid "\001Zd\277v4X";
  set vendor-class-identifier = "android-dhcp-13";
}
lease 10.11.4.55 {
  starts 4 2023/02/23 16:20:27;
  ends 5 2023/02/24 04:20:27;
  tstp 5 2023/02/24 04:20:27;
  cltt 4 2023/02/23 18:50:26;
  binding state free;
  hardware ethernet 12:48:21:93:f9:83;
  uid "\001\022H!\223\371\203";
}
lease 10.11.4.57 {
  starts 5 2023/02/24 07:51:09;
  ends 5 2023/02/24 19:51:09;
  tstp 5 2023/02/24 19:51:09;
  cltt 5 2023/02/24 07:51:09;
  binding state free;
  hardware ethernet 1c:15:1f:aa:29:36;
  uid "\001\034\025\037\252)6";
  set vendor-class-identifier = "HUAWEI:android:ALP";
}
lease 10.11.4.56 {
  starts 5 2023/02/24 12:07:59;
  ends 6 2023/02/25 00:07:59;
  tstp 6 2023/02/25 00:07:59;
  cltt 5 2023/02/24 12:07:59;
  binding state free;
  hardware ethernet 3e:34:59:df:2e:aa;
  uid "\001>4Y\337.\252";
  set vendor-class-identifier = "android-dhcp-13";
}
lease 10.11.4.52 {
  starts 0 2023/02/26 07:52:20;
  ends 0 2023/02/26 19:52:20;
  cltt 0 2023/02/26 07:52:20;
  binding state active;
  next binding state free;
  rewind binding state free;
  hardware ethernet 6c:6a:77:f9:cc:93;
  uid "\001ljw\371\314\223";
  client-hostname "mailbook";
}"#,
    );

    let _ = res.unwrap();
}
