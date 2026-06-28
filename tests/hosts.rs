extern crate dhcpd_parser;

use crate::dhcpd_parser::parser;
use crate::dhcpd_parser::parser::LeasesMethods;

#[test]
fn leases_with_omapi_host_blocks() {
    // OMAPI writes `host` blocks into dhcpd.leases interleaved with leases.
    // The parser must not choke and must extract both.
    let res = parser::parse(
        r#"
authoring-byte-order little-endian;
server-duid "abc";

host omapi-client {
  dynamic;
  hardware ethernet AA:BB:CC:DD:EE:FF;
  fixed-address 10.11.5.100;
}

lease 10.11.5.50 {
  starts 2 2019/01/01 22:00:00 UTC;
  hardware ethernet 11:22:33:44:55:66;
  binding state active;
}
"#,
    );

    assert!(res.is_ok(), "parse failed: {:?}", res.err());
    let result = res.unwrap();

    assert_eq!(result.leases.all().len(), 1);
    assert_eq!(result.leases.all()[0].ip, "10.11.5.50");

    assert_eq!(result.hosts.len(), 1);
    let host = &result.hosts[0];
    assert_eq!(host.name, "omapi-client");
    assert_eq!(host.mac.as_deref(), Some("aa:bb:cc:dd:ee:ff"));
    assert_eq!(host.fixed_addresses, vec!["10.11.5.100".to_owned()]);
}

#[test]
fn dhcpd_conf_subnet_with_nested_hosts() {
    // dhcpd.conf nests host declarations inside subnet blocks and uses many
    // statements the parser doesn't model — all must be tolerated.
    let res = parser::parse(
        r#"
server-name "ratzek";
option routers 10.11.4.1;

subnet 10.11.5.0 netmask 255.255.255.0 {
  interface eth0;
  range 10.11.5.50 10.11.5.255;
  option routers 10.11.5.1;
  host evgenii-hp-probook {
    hardware ethernet 4c:d5:77:88:cc:3b;
    fixed-address 10.11.5.222;
  }
  host dual-subnet {
    hardware ethernet 88:32:9b:07:0a:46;
    fixed-address 10.11.4.223, 10.11.5.223;
  }
}

host top-level-infra {
  hardware ethernet 30:de:4b:03:a9:89;
  fixed-address 10.11.4.1;
}
"#,
    );

    assert!(res.is_ok(), "parse failed: {:?}", res.err());
    let hosts = res.unwrap().hosts;
    assert_eq!(hosts.len(), 3);

    let probook = hosts.iter().find(|h| h.name == "evgenii-hp-probook").unwrap();
    assert_eq!(probook.mac.as_deref(), Some("4c:d5:77:88:cc:3b"));
    assert_eq!(probook.fixed_addresses, vec!["10.11.5.222".to_owned()]);

    let dual = hosts.iter().find(|h| h.name == "dual-subnet").unwrap();
    assert_eq!(
        dual.fixed_addresses,
        vec!["10.11.4.223".to_owned(), "10.11.5.223".to_owned()]
    );

    assert!(hosts.iter().any(|h| h.name == "top-level-infra"));
}
