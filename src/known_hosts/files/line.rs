use super::pattern::*;
use super::KnownHostsError;
use crate::auth::*;

/// A single line of a `known_hosts` file.
pub struct KnownHostsLine<'a>(pub &'a str);

impl<'a> KnownHostsLine<'a> {
    /// Test whether a line matches the given name and identities.
    ///
    ///   - Returns `Ok(true)` iff the hostname matches and either the `id` key matches or the `ca`
    ///     key matches (in case line starts with `@cert-authority).
    ///   - Returns `Ok(false)` iff the line is syntactically incorrect or does not match.
    ///   - Returns `Err(Revoked)` iff the line starts with `@revoked` and the key matches either
    ///     the given `id` or `ca` key. The hostname is ignored in this case.
    pub fn test(
        &self,
        host_name: &str,
        host_key: &PublicKey,
        host_ca_key: Option<&PublicKey>,
    ) -> Result<bool, KnownHostsError> {
        // Split the line by whitespace. Complication is introduced by optional @-marker.
        let mut ws = self.0.split_whitespace();
        let w1 = ws.next().unwrap_or("");
        let marker;
        let pattern;
        let algo;
        let key;
        if w1.starts_with('@') {
            marker = Some(w1);
            pattern = ws.next().unwrap_or("");
            algo = ws.next().unwrap_or("");
            key = ws.next().unwrap_or("");
        } else {
            marker = None; // internal use only
            pattern = w1;
            algo = ws.next().unwrap_or("");
            key = ws.next().unwrap_or("");
        }
        // Reject the key if it has been revoked. For a certificate is it sufficient if either the
        // signed key or the signing key has been marked as revoked. The hostname is not checked.
        if marker == Some("@revoked") {
            if let Ok(key) = base64::decode(key) {
                let key = PublicKey::from(key);
                if &key == host_key || Some(&key) == host_ca_key {
                    return Err(KnownHostsError::KeyRevoked);
                }
            }
            return Ok(false);
        }
        // Stop if the hostname does not match the pattern (hash or glob) on this line.
        if !KnownHostsPattern(pattern).test(host_name) {
            return Ok(false);
        }
        // Test key against `host_key` if no marker is present.
        if marker == None && algo == host_key.algorithm() {
            if let Ok(key) = base64::decode(key) {
                let key = PublicKey::from(key);
                if &key == host_key {
                    return Ok(true);
                }
            }
            return Ok(false);
        }
        // Test key against `host_ca_key` if marker is `@cert-authority`.
        if marker == Some("@cert-authority") && Some(algo) == host_ca_key.map(|x| x.algorithm()) {
            if let Ok(key) = base64::decode(key) {
                let key = PublicKey::from(key);
                if Some(&key) == host_ca_key {
                    return Ok(true);
                }
            }
            return Ok(false);
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pubkey_match() {
        let id = PublicKey::from(vec![
            0x00, 0x00, 0x00, 0x07, 0x73, 0x73, 0x68, 0x2d, 0x72, 0x73, 0x61, 0x00, 0x00, 0x00,
            0x00,
        ]);
        let line = KnownHostsLine("localhost ssh-rsa AAAAB3NzaC1yc2EAAAAA");
        assert_eq!(line.test("localhost", &id, None).unwrap(), true);
    }

    #[test]
    fn test_pubkey_wrong_host() {
        let id = PublicKey::from(vec![
            0x00, 0x00, 0x00, 0x07, 0x73, 0x73, 0x68, 0x2d, 0x72, 0x73, 0x61, 0x00, 0x00, 0x00,
            0x00,
        ]);
        let line = KnownHostsLine("localhost ssh-rsa AAAAB3NzaC1yc2EAAAAA");
        assert_eq!(line.test("XXX", &id, None).unwrap(), false);
    }

    #[test]
    fn test_pubkey_wrong_algorithm() {
        let id = PublicKey::from(vec![
            0x00, 0x00, 0x00, 0x07, 0x73, 0x73, 0x68, 0x2d, 0x72, 0x73, 0x61, 0x00, 0x00, 0x00,
            0x00,
        ]);
        let line = KnownHostsLine("localhost ssh-XXX AAAAB3NzaC1yc2EAAAAA");
        assert_eq!(line.test("localhost", &id, None).unwrap(), false);
    }

    #[test]
    fn test_pubkey_wrong_key() {
        let id = PublicKey::from(vec![
            0x00, 0x00, 0x00, 0x07, 0x73, 0x73, 0x68, 0x2d, 0x72, 0x73, 0x61, 0x00, 0x00, 0x00,
            0xFF, // <- !!!
        ]);
        let line = KnownHostsLine("localhost ssh-rsa AAAAB3NzaC1yc2EAAAAA");
        assert_eq!(line.test("localhost", &id, None).unwrap(), false);
    }

    #[test]
    fn test_cert_match() {
        let id = PublicKey::from(vec![]);
        let ca = PublicKey::from(vec![
            0x00, 0x00, 0x00, 0x07, 0x73, 0x73, 0x68, 0x2d, 0x72, 0x73, 0x61, 0x00, 0x00, 0x00,
            0x00,
        ]);
        let line = KnownHostsLine("@cert-authority localhost ssh-rsa AAAAB3NzaC1yc2EAAAAA");
        assert_eq!(line.test("localhost", &id, Some(&ca)).unwrap(), true);
    }

    #[test]
    fn test_cert_dont_match_id() {
        let id = PublicKey::from(vec![
            0x00, 0x00, 0x00, 0x07, 0x73, 0x73, 0x68, 0x2d, 0x72, 0x73, 0x61, 0x00, 0x00, 0x00,
            0x00,
        ]);
        let ca = PublicKey::from(vec![]);
        let line = KnownHostsLine("@cert-authority localhost ssh-rsa AAAAB3NzaC1yc2EAAAAA");
        assert_eq!(line.test("localhost", &id, Some(&ca)).unwrap(), false);
    }

    #[test]
    fn test_cert_wrong_host() {
        let id = PublicKey::from(vec![]);
        let ca = PublicKey::from(vec![
            0x00, 0x00, 0x00, 0x07, 0x73, 0x73, 0x68, 0x2d, 0x72, 0x73, 0x61, 0x00, 0x00, 0x00,
            0x00,
        ]);
        let line = KnownHostsLine("@cert-authority localhost ssh-rsa AAAAB3NzaC1yc2EAAAAA");
        assert_eq!(line.test("XXX", &id, Some(&ca)).unwrap(), false);
    }

    #[test]
    fn test_cert_wrong_algorithm() {
        let id = PublicKey::from(vec![]);
        let ca = PublicKey::from(vec![
            0x00, 0x00, 0x00, 0x07, 0x73, 0x73, 0x68, 0x2d, 0x72, 0x73, 0x61, 0x00, 0x00, 0x00,
            0x00,
        ]);
        let line = KnownHostsLine("@cert-authority localhost ssh-XXX AAAAB3NzaC1yc2EAAAAA");
        assert_eq!(line.test("localhost", &id, Some(&ca)).unwrap(), false);
    }

    #[test]
    fn test_cert_wrong_key() {
        let id = PublicKey::from(vec![]);
        let ca = PublicKey::from(vec![
            0x00, 0x00, 0x00, 0x07, 0x73, 0x73, 0x68, 0x2d, 0x72, 0x73, 0x61, 0x00, 0x00, 0x00,
            0xFF, // <- !!!
        ]);
        let line = KnownHostsLine("@cert-authority localhost ssh-rsa AAAAB3NzaC1yc2EAAAAA");
        assert_eq!(line.test("localhost", &id, Some(&ca)).unwrap(), false);
    }

    #[test]
    fn test_revoked_id() {
        let id = PublicKey::from(vec![
            0x00, 0x00, 0x00, 0x07, 0x73, 0x73, 0x68, 0x2d, 0x72, 0x73, 0x61, 0x00, 0x00, 0x00,
            0x00,
        ]);
        let line = KnownHostsLine("@revoked localhost ssh-rsa AAAAB3NzaC1yc2EAAAAA");
        match line.test("XXX", &id, None) {
            Err(KnownHostsError::KeyRevoked) => (),
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn test_revoked_ca() {
        let id = PublicKey::from(vec![]);
        let ca = PublicKey::from(vec![
            0x00, 0x00, 0x00, 0x07, 0x73, 0x73, 0x68, 0x2d, 0x72, 0x73, 0x61, 0x00, 0x00, 0x00,
            0x00,
        ]);
        let line = KnownHostsLine("@revoked localhost ssh-rsa AAAAB3NzaC1yc2EAAAAA");
        match line.test("XXX", &id, Some(&ca)) {
            Err(KnownHostsError::KeyRevoked) => (),
            other => panic!("{:?}", other),
        }
    }
}
