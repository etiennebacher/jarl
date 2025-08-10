#[derive(Debug, Clone, Default)]
pub struct Rule {
    pub name: String,
    pub should_fix: bool,
    pub minimum_r_version: Option<(u32, u32)>,
}

#[derive(Debug, Clone, Default)]
pub struct RuleTable {
    pub enabled: Vec<Rule>,
}
impl RuleTable {
    /// Creates a new empty rule table.
    pub fn empty() -> Self {
        Self { enabled: Vec::new() }
    }

    /// Enables the given rule.
    #[inline]
    pub fn enable(&mut self, rule: &str, should_fix: bool, minimum_r_version: Option<(u32, u32)>) {
        self.enabled.push(Rule {
            name: rule.to_string(),
            should_fix: should_fix,
            minimum_r_version: minimum_r_version,
        });
    }

    /// Returns an iterator over the rules.
    pub fn iter(&self) -> std::slice::Iter<'_, Rule> {
        self.enabled.iter()
    }
}

impl FromIterator<Rule> for RuleTable {
    fn from_iter<I: IntoIterator<Item = Rule>>(iter: I) -> Self {
        let enabled: Vec<Rule> = iter.into_iter().collect();
        RuleTable { enabled }
    }
}
