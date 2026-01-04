#[derive(Debug, Clone, Default)]
pub struct Rule {
    pub name: String,
    pub categories: Vec<String>,
    pub default_status: DefaultStatus,
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
    pub fn is_enabled_by_default(&self) -> bool {
        self.default_status == DefaultStatus::Enabled
    }
    pub fn is_disabled_by_default(&self) -> bool {
        self.default_status == DefaultStatus::Disabled
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DefaultStatus {
    #[default]
    Enabled,
    Disabled,
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
    pub rules: Vec<Rule>,
}
impl RuleTable {
    /// Creates a new empty rule table.
    pub fn empty() -> Self {
        Self { rules: Vec::new() }
    }

    /// Enables the given rule.
    #[inline]
    pub fn add_rule(
        &mut self,
        rule: &str,
        categories: &str,
        default_status: DefaultStatus,
        fix_status: FixStatus,
        minimum_r_version: Option<(u32, u32, u32)>,
    ) {
        self.rules.push(Rule {
            name: rule.to_string(),
            categories: categories.split(',').map(|s| s.to_string()).collect(),
            default_status,
            fix_status,
            minimum_r_version,
        });
    }

    /// Returns an iterator over the rules.
    pub fn iter(&self) -> std::slice::Iter<'_, Rule> {
        self.rules.iter()
    }
}

impl FromIterator<Rule> for RuleTable {
    fn from_iter<I: IntoIterator<Item = Rule>>(iter: I) -> Self {
        let rules: Vec<Rule> = iter.into_iter().collect();
        RuleTable { rules }
    }
}
