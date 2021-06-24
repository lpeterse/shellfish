use crate::util::check;
use crate::util::glob::Glob;
use hmac::crypto_mac::NewMac;
use hmac::{Hmac, Mac};
use sha1::Sha1;

pub struct KnownHostsPattern<'a>(pub &'a str);

impl<'a> KnownHostsPattern<'a> {
    pub fn test(&self, name: &str) -> bool {
        if self.0.starts_with('|') {
            self.test_hash(name).is_some()
        } else {
            self.test_globs(name)
        }
    }

    fn test_hash(&self, name: &str) -> Option<()> {
        const ALG_HMAC_SHA1: usize = 1;
        let mut known = self.0.split('|');
        let _ = known.next().filter(|s| s.is_empty())?; // empty iff starts with pipe
        let alg: usize = known.next()?.parse().ok()?;
        if alg == ALG_HMAC_SHA1 {
            let key = known.next()?;
            let mac = known.next()?;
            // 16 bytes information shall make 28 chars in base64
            check(key.len() == 28).map(drop)?;
            check(mac.len() == 28).map(drop)?;
            // 28 base64 chars cannot contain more than 28 byte information,
            // but might contain more than 16 bytes
            let mut k: [u8; 28] = [0; 28];
            let mut m: [u8; 28] = [0; 28];
            let klen = base64::decode_config_slice(key, base64::STANDARD, &mut k).ok()?;
            let mlen = base64::decode_config_slice(mac, base64::STANDARD, &mut m).ok()?;
            let mut hmac = Hmac::<Sha1>::new_from_slice(k.get(..klen)?).ok()?;
            hmac.update(name.as_ref());
            hmac.verify(&m[..mlen]).ok()
        } else {
            None
        }
    }

    fn test_globs(&self, name: &str) -> bool {
        // Test all globs. Stop immediately on negated match or try all.
        let mut result = false;
        for glob in self.0.split(',') {
            if let Some(glob) = glob.strip_prefix('!') {
                if Self::test_glob(glob, name) {
                    // Return early: This line must not be used for this host
                    return false;
                }
            } else if Self::test_glob(glob, name) {
                // DO NOT return early: Negated match might follow!
                result = true
            }
        }
        result
    }

    fn test_glob(glob: &str, name: &str) -> bool {
        let valid = |c: char| c.is_ascii_alphanumeric() || ":.-*?".contains(c);
        !glob.is_empty() && glob.chars().all(valid) && Glob(glob).test(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_host(pattern: &str, name: &str) -> bool {
        KnownHostsPattern(pattern).test(name)
    }

    #[test]
    fn host_name_test_01() {
        let pattern = "*.example.com,10.0.0.?,!bar.example.com";

        assert_eq!(test_host(pattern, "foobar.example.com"), true);
        assert_eq!(test_host(pattern, "10.0.0.1"), true);
        assert_eq!(test_host(pattern, "example.com"), false);
        assert_eq!(test_host(pattern, "bar.example.com"), false);
        assert_eq!(test_host(pattern, "10.0.0.11"), false);
        assert_eq!(test_host(pattern, ""), false);
    }

    #[test]
    fn host_name_test_02() {
        let pattern = ",,,";

        assert_eq!(test_host(pattern, "foobar.example.com"), false);
        assert_eq!(test_host(pattern, "10.0.0.1"), false);
        assert_eq!(test_host(pattern, "example.com"), false);
        assert_eq!(test_host(pattern, "bar.example.com"), false);
        assert_eq!(test_host(pattern, "10.0.0.11"), false);
        assert_eq!(test_host(pattern, ""), false); // sic!
    }

    #[test]
    fn host_name_test_03() {
        let pattern = "|1|F1E1KeoE/eEWhi10WpGv4OdiO6Y=|3988QV0VE8wmZL7suNrYQLITLCg=";

        assert_eq!(test_host(pattern, "192.168.1.61"), true);
        assert_eq!(test_host(pattern, "192.168.1.1"), false);
    }

    #[test]
    fn host_name_test_04() {
        let pattern = "|1|F1E1KeoE/eEWhi10WpGv4OdiO6Y= |3988QV0VE8wmZL7suNrYQLITLCg=";

        assert_eq!(test_host(pattern, "192.168.1.61"), false);
        assert_eq!(test_host(pattern, "192.168.1.1"), false);
    }

    #[test]
    fn host_name_test_05() {
        let pattern = "1|F1E1KeoE/eEWhi10WpGv4OdiO6Y=|3988QV0VE8wmZL7suNrYQLITLCg=";

        assert_eq!(test_host(pattern, "192.168.1.61"), false);
        assert_eq!(test_host(pattern, "192.168.1.1"), false);
    }

    #[test]
    fn host_name_test_06() {
        let pattern = "|2|F1E1KeoE/eEWhi10WpGv4OdiO6Y=|3988QV0VE8wmZL7suNrYQLITLCg=";

        assert_eq!(test_host(pattern, "192.168.1.61"), false);
        assert_eq!(test_host(pattern, "192.168.1.1"), false);
    }
}
