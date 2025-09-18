#[derive(Debug, Clone, Default)]
pub struct Rule {
    pub name: String,
    pub categories: Vec<String>,
    pub fix_status: FixStatus,
    pub minimum_r_version: Option<(u32, u32, u32)>,
}

impl Rule {
    pub fn has_safe_fix(&self) -> bool {
        self.fix_status == FixStatus::Safe
    }
    pub fn has_unsafe_fix(&self) -> bool {
        self.fix_status == FixStatus::Unsafe
    }
    pub fn has_no_fix(&self) -> bool {
        self.fix_status == FixStatus::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FixStatus {
    #[default]
    None,
    Safe,
    Unsafe,
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
    pub fn enable(
        &mut self,
        rule: &str,
        categories: &str,
        fix_status: FixStatus,
        minimum_r_version: Option<(u32, u32, u32)>,
    ) {
        self.enabled.push(Rule {
            name: rule.to_string(),
            categories: categories.split(',').map(|s| s.to_string()).collect(),
            fix_status,
            minimum_r_version,
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
