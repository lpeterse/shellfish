use crate::util::assume;
use std::net::IpAddr;

/// Classless Inter-Domain Routing (CIDR) subnet calculation.
///
/// Shall wrap a string in CIDR notation (i.e. '10.0.0.0/16' or 'fe80::/96').
#[derive(Debug)]
pub struct Cidr<'a>(pub &'a str);

impl<'a> Cidr<'a> {
    /// Test whether the given address is part of the given subnet.
    ///
    /// Returns `true` iff the CIDR string is valid and the address is within the subnet.
    pub fn contains(&self, addr: &IpAddr) -> bool {
        fn contains_(cidr: &str, addr: &IpAddr) -> Option<()> {
            // TODO: Use split_once when available
            let mut x = cidr.split('/');
            let net = x.next().map(|s| s.parse::<IpAddr>().ok()).flatten()?;
            let sfx = x.next().map(|s| s.parse::<u8>().ok()).flatten()?;
            assume(x.next().is_none())?;
            match (net, addr) {
                (IpAddr::V4(net), IpAddr::V4(adr)) if sfx <= 32 => {
                    let msk = u32::MAX.checked_shl(32 - u32::from(sfx)).unwrap_or(0);
                    let net = u32::from_be_bytes(net.octets());
                    let adr = u32::from_be_bytes(adr.octets());
                    assume(adr & msk == net & msk)
                }
                (IpAddr::V6(net), IpAddr::V6(adr)) if sfx <= 128 => {
                    let msk = u128::MAX.checked_shl(128 - u32::from(sfx)).unwrap_or(0);
                    let net = u128::from_be_bytes(net.octets());
                    let adr = u128::from_be_bytes(adr.octets());
                    assume(adr & msk == net & msk)
                }
                _ => None,
            }
        };
        contains_(self.0, addr).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cidr_invalid() {
        assert!(!Cidr("").contains(&"0.0.0.0".parse().unwrap()));
        assert!(!Cidr("").contains(&"255.255.255.255".parse().unwrap()));
        assert!(!Cidr("").contains(&"::1".parse().unwrap()));
    }

    #[test]
    fn cidr_v4_slash_0() {
        assert!(Cidr("0.0.0.0/0").contains(&"0.0.0.0".parse().unwrap()));
        assert!(Cidr("0.0.0.0/0").contains(&"127.0.0.1".parse().unwrap()));
        assert!(Cidr("0.0.0.0/0").contains(&"255.255.255.255".parse().unwrap()));
        assert!(Cidr("127.0.0.0/0").contains(&"0.0.0.0".parse().unwrap()));
        assert!(Cidr("127.0.0.0/0").contains(&"127.0.0.1".parse().unwrap()));
        assert!(Cidr("127.0.0.0/0").contains(&"255.255.255.255".parse().unwrap()));
    }

    #[test]
    fn cidr_v4_slash_1() {
        assert!(!Cidr("129.0.0.0/1").contains(&"0.0.0.0".parse().unwrap()));
        assert!(!Cidr("129.0.0.0/1").contains(&"127.255.255.255".parse().unwrap()));
        assert!(Cidr("129.0.0.0/1").contains(&"128.0.0.0".parse().unwrap()));
        assert!(Cidr("129.0.0.0/1").contains(&"255.255.255.255".parse().unwrap()));
    }

    #[test]
    fn cidr_v4_slash_24() {
        assert!(!Cidr("10.0.0.0/24").contains(&"9.255.255.255".parse().unwrap()));
        assert!(Cidr("10.0.0.0/24").contains(&"10.0.0.0".parse().unwrap()));
        assert!(Cidr("10.0.0.0/24").contains(&"10.0.0.255".parse().unwrap()));
        assert!(!Cidr("10.0.0.0/24").contains(&"10.0.1.0".parse().unwrap()));
        assert!(!Cidr("10.0.0.0/24").contains(&"10.0.1.1".parse().unwrap()));
    }

    #[test]
    fn cidr_v4_slash_31() {
        assert!(!Cidr("10.0.0.16/31").contains(&"10.0.0.15".parse().unwrap()));
        assert!(Cidr("10.0.0.16/31").contains(&"10.0.0.16".parse().unwrap()));
        assert!(Cidr("10.0.0.16/31").contains(&"10.0.0.17".parse().unwrap()));
        assert!(!Cidr("10.0.0.16/31").contains(&"10.0.0.18".parse().unwrap()));

        assert!(!Cidr("10.1.2.3/31").contains(&"10.1.2.1".parse().unwrap()));
        assert!(Cidr("10.1.2.3/31").contains(&"10.1.2.2".parse().unwrap()));
        assert!(Cidr("10.1.2.3/31").contains(&"10.1.2.3".parse().unwrap()));
        assert!(!Cidr("10.1.2.3/31").contains(&"10.1.2.4".parse().unwrap()));
    }

    #[test]
    fn cidr_v4_slash_32() {
        assert!(!Cidr("10.1.2.3/32").contains(&"10.1.2.2".parse().unwrap()));
        assert!(Cidr("10.1.2.3/32").contains(&"10.1.2.3".parse().unwrap()));
        assert!(!Cidr("10.1.2.3/32").contains(&"10.1.2.4".parse().unwrap()));
    }

    #[test]
    fn cidr_v4_slash_33() {
        assert!(!Cidr("10.1.2.3/33").contains(&"10.1.2.2".parse().unwrap()));
        assert!(!Cidr("10.1.2.3/33").contains(&"10.1.2.3".parse().unwrap()));
        assert!(!Cidr("10.1.2.3/33").contains(&"10.1.2.4".parse().unwrap()));
    }

    #[test]
    fn cidr_v6_slash_96() {
        assert!(Cidr("fe80::5bb0:b6ba:ce05:d258/96")
            .contains(&"fe80:0:0:0:5bb0:b6ba:0:0".parse().unwrap()));
        assert!(Cidr("fe80::5bb0:b6ba:ce05:d258/96")
            .contains(&"fe80::5bb0:b6ba:ce05:d258".parse().unwrap()));
        assert!(Cidr("fe80::5bb0:b6ba:ce05:d258/96")
            .contains(&"fe80:0:0:0:5bb0:b6ba:ffff:ffff".parse().unwrap()));
        assert!(!Cidr("fe80::5bb0:b6ba:ce05:d258/96")
            .contains(&"fe80:0:0:0:5bb0:b6bb:0:0".parse().unwrap()));
    }
}
