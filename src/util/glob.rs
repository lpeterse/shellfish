use std::ops::Deref;

/// A glob is a string matching pattern that contains wildcards.
///
/// `?` matches a single character whereas `*` matches any number of characters.
///
/// This implementation has a `O(n*m)` worst-case runtime for degenerated cases, but should be
/// `O(n+m)` for most real-world input. It operates on Unicode codepoints, should always terminate
/// and never panic.
///
/// Example: `Glob("*.example.???").test("foobar.example.com") == true`
#[derive(Debug, Clone)]
pub struct Glob<T: Deref<Target = str> = String>(pub T);

impl<T: Deref<Target = str>> Glob<T> {
    const ONE: char = '?';
    const MANY: char = '*';

    pub fn test(&self, input: &str) -> bool {
        let mut patterns = self.0.split(Self::MANY);
        let mut it = input.chars();
        // If the first/last pattern is not empty, the string must start/end on them.
        if let Some(pt) = patterns.next() {
            if !pt.is_empty() && Self::strip(pt, &mut it, |x| x.next()) != Some(true) {
                return false;
            }
        }
        if let Some(pt) = patterns.next_back() {
            if !pt.is_empty() && Self::strip(pt, &mut it, |x| x.next_back()) != Some(true) {
                return false;
            }
        } else {
            return it.next().is_none();
        }
        // All remaining patterns may freely float (in order) over the remaining input.
        // Shift the patterns over the input and remove them on match until none are left.
        // All pattern boundaries represent '*' which allow the patterns to float.
        'pattern: while let Some(pt) = patterns.next() {
            let mut remaining = it.clone();
            'backtrack: loop {
                match Self::strip(pt, &mut it, |x| x.next()) {
                    Some(true) => continue 'pattern,
                    Some(false) => return false,
                    None => {
                        let _ = remaining.next();
                        it = remaining.clone();
                        continue 'backtrack;
                    }
                }
            }
        }
        // The glob either ended on '*' or at least contained one left of the suffix that has
        // already been removed. By construction, the remaining input is matched by that '*'.
        true
    }

    fn strip(
        pattern: &str,
        input: &mut std::str::Chars,
        next: fn(&mut std::str::Chars) -> Option<char>,
    ) -> Option<bool> {
        let mut it = input.clone();
        let mut pt = pattern.chars();
        while let Some(p) = next(&mut pt) {
            if let Some(i) = next(&mut it) {
                if i == p || p == Self::ONE {
                    continue;
                }
                return None;
            }
            return Some(false);
        }
        *input = it;
        Some(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_test_01() {
        let glob = Glob("");
        assert!(glob.test(""));
        assert!(!glob.test("a"));
    }

    #[test]
    fn test_glob_test_02() {
        let glob = Glob("a");
        assert!(glob.test("a"));
        assert!(!glob.test(""));
        assert!(!glob.test("b"));
        assert!(!glob.test("ab"));
        assert!(!glob.test("ba"));
        assert!(!glob.test("aba"));
        assert!(!glob.test("bab"));
    }

    #[test]
    fn test_glob_test_03() {
        let glob = Glob("?");
        assert!(glob.test("a"));
        assert!(glob.test("?"));
        assert!(!glob.test(""));
        assert!(!glob.test("ab"));
    }

    #[test]
    fn test_glob_test_04() {
        let glob = Glob("*");
        assert!(glob.test(""));
        assert!(glob.test("a"));
        assert!(glob.test("ab"));
    }

    #[test]
    fn test_glob_test_05() {
        let glob = Glob("a*");
        assert!(glob.test("a"));
        assert!(glob.test("ab"));
        assert!(glob.test("abc"));
        assert!(!glob.test(""));
        assert!(!glob.test("b"));
        assert!(!glob.test("bc"));
    }

    #[test]
    fn test_glob_test_06() {
        let glob = Glob("*c");
        assert!(glob.test("c"));
        assert!(glob.test("bc"));
        assert!(glob.test("abc"));
        assert!(!glob.test(""));
        assert!(!glob.test("b"));
        assert!(!glob.test("ab"));
    }

    #[test]
    fn test_glob_test_07() {
        let glob = Glob("a*cd");
        assert!(glob.test("acd"));
        assert!(glob.test("abcd"));
        assert!(glob.test("accd"));
        assert!(!glob.test(""));
        assert!(!glob.test("a"));
        assert!(!glob.test("cd"));
    }

    #[test]
    fn test_glob_test_08() {
        let glob = Glob("a**d");
        assert!(glob.test("ad"));
        assert!(glob.test("abd"));
        assert!(glob.test("acdd"));
        assert!(glob.test("add"));
        assert!(glob.test("addd"));
        assert!(!glob.test(""));
        assert!(!glob.test("a"));
        assert!(!glob.test("d"));
        assert!(!glob.test("ab"));
        assert!(!glob.test("ade"));
    }

    #[test]
    fn test_glob_test_09() {
        let glob = Glob("abc*def*hij");
        assert!(glob.test("abcdefhhihihij"));
        assert!(glob.test("abcdeXdefYZhij"));
        assert!(!glob.test("abchij"));
    }

    #[test]
    fn test_glob_test_10() {
        let glob = Glob("*.example.???");
        assert!(glob.test("foobar.example.com"));
        assert!(glob.test("foobar.example.net"));
        assert!(glob.test("foobar.example.example.net"));
        assert!(!glob.test("foobar.example.de"));
    }
}
