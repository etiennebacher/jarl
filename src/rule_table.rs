use std::fmt::Display;
use std::fmt::Formatter;

#[derive(Debug, Clone, Default)]
pub struct RuleTable {
    pub enabled: Vec<String>,
    pub should_fix: Vec<String>,
}
impl RuleTable {
    /// Creates a new empty rule table.
    pub fn empty() -> Self {
        Self { enabled: Vec::new(), should_fix: Vec::new() }
    }

    /// Returns whether the given rule should be checked.
    #[inline]
    pub fn enabled(&self, rule: &str) -> bool {
        self.enabled.contains(&rule.to_string())
    }

    /// Returns whether any of the given rules should be checked.
    #[inline]
    pub fn any_enabled(&self, rules: Vec<&str>) -> bool {
        self.enabled.iter().any(|r| rules.contains(&r.as_str()))
    }

    /// Returns whether violations of the given rule should be fixed.
    #[inline]
    pub fn should_fix(&self, rule: &str) -> bool {
        self.should_fix.contains(&rule.to_string())
    }

    /// Returns an iterator over all enabled rules.
    // pub fn iter_enabled(&self) -> RuleSetIterator {
    //     self.enabled.iter()
    // }

    /// Enables the given rule.
    #[inline]
    pub fn enable(&mut self, rule: &str, should_fix: bool) {
        self.enabled.push(rule.to_string());

        if should_fix {
            self.should_fix.push(rule.to_string());
        }
    }

    /// Disables the given rule.
    #[inline]
    pub fn disable(&mut self, rule: &str) {
        self.enabled.retain(|x| x != rule);
        self.should_fix.retain(|x| x != rule);
    }
}

impl Display for RuleTable {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Enabled rules: {}\nRules with fix: {}",
            self.enabled.join(", "),
            self.should_fix.join(", ")
        )
    }
}

// impl FromIterator<Rule> for RuleTable {
//     fn from_iter<T: IntoIterator<Item = Rule>>(iter: T) -> Self {
//         let rules = RuleSet::from_iter(iter);
//         Self { enabled: rules.clone(), should_fix: rules }
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut rt = RuleTable::empty();
        assert!(!rt.enabled("foo"));

        rt.enable("foo", true);
        assert!(rt.enabled("foo"));
        assert!(rt.should_fix("foo"));

        rt.enable("bar", false);
        assert!(rt.enabled("bar"));
        assert!(!rt.should_fix("bar"));

        assert!(rt.any_enabled(["bar", "baz"].to_vec()));
        assert!(!rt.any_enabled(["baz", "baz2"].to_vec()));

        rt.disable("bar");
        assert!(!rt.enabled("bar"));
        assert!(!rt.should_fix("bar"));
    }
}
