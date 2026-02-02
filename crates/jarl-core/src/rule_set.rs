use std::fmt;
use std::str::FromStr;

/// Category of a linting rule
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    /// Comments: violations related to suppression comments
    Comm,
    /// Correctness: code that is outright wrong or useless
    Corr,
    /// Suspicious: code that is most likely wrong or useless
    Susp,
    /// Performance: code that can be written to run faster
    Perf,
    /// Readability: code is correct but can be written more clearly
    Read,
    /// Testthat-specific rules
    Testthat,
}

impl Category {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Comm => "COMM",
            Self::Corr => "CORR",
            Self::Susp => "SUSP",
            Self::Perf => "PERF",
            Self::Read => "READ",
            Self::Testthat => "TESTTHAT",
        }
    }

    pub const ALL: &'static [Category] = &[
        Category::Comm,
        Category::Corr,
        Category::Susp,
        Category::Perf,
        Category::Read,
        Category::Testthat,
    ];
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Category {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "COMM" => Ok(Self::Comm),
            "CORR" => Ok(Self::Corr),
            "SUSP" => Ok(Self::Susp),
            "PERF" => Ok(Self::Perf),
            "READ" => Ok(Self::Read),
            "TESTTHAT" => Ok(Self::Testthat),
            _ => Err(format!("Unknown category: {}", s)),
        }
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

macro_rules! declare_rules {
    (
        $(
            $variant:ident => {
                name: $name:literal,
                categories: [$($category:ident),+ $(,)?],
                default: $default:ident,
                fix: $fix:ident,
                min_r_version: $min_version:expr,
            }
        ),* $(,)?
    ) => {
        /// Enum representing all available linting rules
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Rule {
            $($variant),*
        }

        impl Rule {
            /// Get the rule's string name
            pub const fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $name),*
                }
            }

            /// Get the rule's categories
            pub const fn categories(self) -> &'static [Category] {
                match self {
                    $(Self::$variant => &[$(Category::$category),+]),*
                }
            }

            /// Get the rule's default status
            pub const fn default_status(self) -> DefaultStatus {
                match self {
                    $(Self::$variant => DefaultStatus::$default),*
                }
            }

            /// Get the rule's fix status
            pub const fn fix_status(self) -> FixStatus {
                match self {
                    $(Self::$variant => FixStatus::$fix),*
                }
            }

            /// Get the minimum R version required for this rule
            pub const fn minimum_r_version(self) -> Option<(u32, u32, u32)> {
                match self {
                    $(Self::$variant => $min_version),*
                }
            }

            /// Check if the rule has a safe fix
            pub const fn has_safe_fix(self) -> bool {
                matches!(self.fix_status(), FixStatus::Safe)
            }

            /// Check if the rule has an unsafe fix
            pub const fn has_unsafe_fix(self) -> bool {
                matches!(self.fix_status(), FixStatus::Unsafe)
            }

            /// Check if the rule has no fix
            pub const fn has_no_fix(self) -> bool {
                matches!(self.fix_status(), FixStatus::None)
            }

            /// Check if the rule is enabled by default
            pub const fn is_enabled_by_default(self) -> bool {
                matches!(self.default_status(), DefaultStatus::Enabled)
            }

            /// Check if the rule is disabled by default
            pub const fn is_disabled_by_default(self) -> bool {
                matches!(self.default_status(), DefaultStatus::Disabled)
            }

            /// Check if the rule belongs to a specific category
            pub fn has_category(self, category: Category) -> bool {
                self.categories().contains(&category)
            }

            /// Parse a rule from its string name
            pub fn from_name(name: &str) -> Option<Self> {
                match name {
                    $($name => Some(Self::$variant),)*
                    _ => None,
                }
            }

            /// Get all rules as a slice
            pub const fn all() -> &'static [Rule] {
                ALL_RULES
            }
        }

        impl fmt::Display for Rule {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.name())
            }
        }

        /// Static array containing all rules
        pub const ALL_RULES: &[Rule] = &[
            $(Rule::$variant),*
        ];
    };
}

// Declare all rules with their metadata
declare_rules! {
    AllEqual => {
        name: "all_equal",
        categories: [Susp],
        default: Enabled,
        fix: Unsafe,
        min_r_version: None,
    },
    AnyDuplicated => {
        name: "any_duplicated",
        categories: [Perf],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
    AnyIsNa => {
        name: "any_is_na",
        categories: [Perf],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
    Assignment => {
        name: "assignment",
        categories: [Read],
        default: Disabled,
        fix: Safe,
        min_r_version: None,
    },
    BlanketSuppression => {
        name: "blanket_suppression",
        categories: [Comm],
        default: Enabled,
        fix: None,
        min_r_version: None,
    },
    Browser => {
        name: "browser",
        categories: [Corr],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
    ClassEquals => {
        name: "class_equals",
        categories: [Susp],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
    ComparisonNegation => {
        name: "comparison_negation",
        categories: [Read],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
    Coalesce => {
        name: "coalesce",
        categories: [Read],
        default: Enabled,
        fix: Safe,
        min_r_version: Some((4, 4, 0)),
    },
    DownloadFile => {
        name: "download_file",
        categories: [Susp],
        default: Enabled,
        fix: None,
        min_r_version: None,
    },
    DuplicatedArguments => {
        name: "duplicated_arguments",
        categories: [Susp],
        default: Enabled,
        fix: None,
        min_r_version: None,
    },
    EmptyAssignment => {
        name: "empty_assignment",
        categories: [Read],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
    EqualsNa => {
        name: "equals_na",
        categories: [Corr],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
    EqualsNaN => {
        name: "equals_nan",
        categories: [Corr],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
    EqualsNull => {
        name: "equals_null",
        categories: [Corr],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
    ExpectLength => {
        name: "expect_length",
        categories: [Testthat],
        default: Disabled,
        fix: Safe,
        min_r_version: None,
    },
    ExpectNamed => {
        name: "expect_named",
        categories: [Testthat],
        default: Disabled,
        fix: Safe,
        min_r_version: None,
    },
    ExpectNot => {
        name: "expect_not",
        categories: [Testthat],
        default: Disabled,
        fix: Safe,
        min_r_version: None,
    },
    ExpectNull => {
        name: "expect_null",
        categories: [Testthat],
        default: Disabled,
        fix: Safe,
        min_r_version: None,
    },
    ExpectS3Class => {
        name: "expect_s3_class",
        categories: [Testthat],
        default: Disabled,
        fix: Safe,
        min_r_version: None,
    },
    ExpectTrueFalse => {
        name: "expect_true_false",
        categories: [Testthat],
        default: Disabled,
        fix: Safe,
        min_r_version: None,
    },
    ExpectType => {
        name: "expect_type",
        categories: [Testthat],
        default: Disabled,
        fix: Safe,
        min_r_version: None,
    },
    FixedRegex => {
        name: "fixed_regex",
        categories: [Perf],
        default: Disabled,
        fix: Safe,
        min_r_version: None,
    },
    ForLoopIndex => {
        name: "for_loop_index",
        categories: [Read],
        default: Enabled,
        fix: None,
        min_r_version: None,
    },
    Grepv => {
        name: "grepv",
        categories: [Read],
        default: Enabled,
        fix: Safe,
        min_r_version: Some((4, 5, 0)),
    },
    IfAlwaysTrue => {
        name: "if_always_true",
        categories: [Read, Susp],
        default: Enabled,
        fix: None,
        min_r_version: None,
    },
    ImplicitAssignment => {
        name: "implicit_assignment",
        categories: [Read],
        default: Enabled,
        fix: None,
        min_r_version: None,
    },
    InternalFunction => {
        name: "internal_function",
        categories: [Susp],
        default: Enabled,
        fix: None,
        min_r_version: None,
    },
    IsNumeric => {
        name: "is_numeric",
        categories: [Read],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
    LengthLevels => {
        name: "length_levels",
        categories: [Read],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
    LengthTest => {
        name: "length_test",
        categories: [Corr],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
    Lengths => {
        name: "lengths",
        categories: [Perf, Read],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
    List2df => {
        name: "list2df",
        categories: [Perf, Read],
        default: Enabled,
        fix: Safe,
        min_r_version: Some((4, 0, 0)),
    },
    MatrixApply => {
        name: "matrix_apply",
        categories: [Perf],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
    MisplacedFileSuppression => {
        name: "misplaced_file_suppression",
        categories: [Comm],
        default: Enabled,
        fix: None,
        min_r_version: None,
    },
    MisplacedSuppression => {
        name: "misplaced_suppression",
        categories: [Comm],
        default: Enabled,
        fix: None,
        min_r_version: None,
    },
    MisnamedSuppression => {
        name: "misnamed_suppression",
        categories: [Comm],
        default: Enabled,
        fix: None,
        min_r_version: None,
    },
    NumericLeadingZero => {
        name: "numeric_leading_zero",
        categories: [Read],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
    OutdatedSuppression => {
        name: "outdated_suppression",
        categories: [Comm],
        default: Enabled,
        fix: None,
        min_r_version: None,
    },
    OuterNegation => {
        name: "outer_negation",
        categories: [Perf, Read],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
    RedundantEquals => {
        name: "redundant_equals",
        categories: [Read],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
    RedundantIfelse => {
        name: "redundant_ifelse",
        categories: [Corr, Perf, Read],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
    Repeat => {
        name: "repeat",
        categories: [Read],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
    SampleInt => {
        name: "sample_int",
        categories: [Read],
        default: Disabled,
        fix: Safe,
        min_r_version: None,
    },
    Seq => {
        name: "seq",
        categories: [Susp],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
    Seq2 => {
        name: "seq2",
        categories: [Susp],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
    Sort => {
        name: "sort",
        categories: [Perf, Read],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
    Sprintf => {
        name: "sprintf",
        categories: [Corr, Susp],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
    StringBoundary => {
        name: "string_boundary",
        categories: [Perf, Read],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
    SystemFile => {
        name: "system_file",
        categories: [Read],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
    TrueFalseSymbol => {
        name: "true_false_symbol",
        categories: [Read],
        default: Enabled,
        fix: None,
        min_r_version: None,
    },
    UnexplainedSuppression => {
        name: "unexplained_suppression",
        categories: [Comm],
        default: Enabled,
        fix: None,
        min_r_version: None,
    },
    UnmatchedRangeSuppression => {
        name: "unmatched_range_suppression",
        categories: [Comm],
        default: Enabled,
        fix: None,
        min_r_version: None,
    },
    UnnecessaryNesting => {
        name: "unnecessary_nesting",
        categories: [Read],
        default: Disabled,
        fix: Safe,
        min_r_version: None,
    },
    UnreachableCode => {
        name: "unreachable_code",
        categories: [Read, Susp],
        default: Enabled,
        fix: None,
        min_r_version: None,
    },
    VectorLogic => {
        name: "vector_logic",
        categories: [Perf],
        default: Enabled,
        fix: None,
        min_r_version: None,
    },
    WhichGrepl => {
        name: "which_grepl",
        categories: [Perf, Read],
        default: Enabled,
        fix: Safe,
        min_r_version: None,
    },
}

/// A collection of rules
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleSet {
    rules: Vec<Rule>,
}

impl RuleSet {
    /// Create an empty rule set
    pub fn empty() -> Self {
        Self { rules: Vec::new() }
    }

    /// Create a rule set from a vector of rules
    pub fn from_rules(rules: Vec<Rule>) -> Self {
        Self { rules }
    }

    /// Create a rule set containing all rules
    pub fn all() -> Self {
        Self { rules: ALL_RULES.to_vec() }
    }

    /// Get an iterator over the rules
    pub fn iter(&self) -> impl Iterator<Item = &Rule> {
        self.rules.iter()
    }

    /// Check if the rule set contains a specific rule
    pub fn contains(&self, rule: &Rule) -> bool {
        self.rules.contains(rule)
    }

    /// Check if the rule set contains a rule by name
    pub fn contains_name(&self, name: &str) -> bool {
        self.rules.iter().any(|r| r.name() == name)
    }

    /// Get the number of rules in the set
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Check if the rule set is empty
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Filter rules by a predicate
    pub fn filter<F>(self, predicate: F) -> Self
    where
        F: FnMut(&Rule) -> bool,
    {
        Self {
            rules: self.rules.into_iter().filter(predicate).collect(),
        }
    }
}

impl FromIterator<Rule> for RuleSet {
    fn from_iter<I: IntoIterator<Item = Rule>>(iter: I) -> Self {
        Self { rules: iter.into_iter().collect() }
    }
}

impl<'a> FromIterator<&'a Rule> for RuleSet {
    fn from_iter<I: IntoIterator<Item = &'a Rule>>(iter: I) -> Self {
        Self { rules: iter.into_iter().copied().collect() }
    }
}

/// Helper functions for working with rules
impl Rule {
    /// Get all rules with a specific fix status
    pub fn by_fix_status(status: FixStatus) -> impl Iterator<Item = Rule> {
        ALL_RULES
            .iter()
            .copied()
            .filter(move |r| r.fix_status() == status)
    }

    /// Get all rules in a specific category
    pub fn by_category(category: Category) -> impl Iterator<Item = Rule> {
        ALL_RULES
            .iter()
            .copied()
            .filter(move |r| r.has_category(category))
    }

    /// Get all rules enabled by default
    pub fn enabled_by_default() -> impl Iterator<Item = Rule> {
        ALL_RULES
            .iter()
            .copied()
            .filter(|r| r.is_enabled_by_default())
    }

    /// Get all rules disabled by default
    pub fn disabled_by_default() -> impl Iterator<Item = Rule> {
        ALL_RULES
            .iter()
            .copied()
            .filter(|r| r.is_disabled_by_default())
    }
}
