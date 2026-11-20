//! `KaleidBeat` runtime conditional-branch commands.
//!
//! Every command in this module is a `KaleidBeat` extension: none of them are
//! defined by the BMS specification or beatoraja. The evaluation itself belongs
//! to the player; this crate only types the source form.

use std::fmt;
use std::str::FromStr;

use crate::{BmsTokenAttr, ParseBmsValueError};

/// Variable compared by a `#WHEN` / `#ELSEWHEN` condition.
///
/// `KaleidBeat` extension: these live play-state values are not part of the BMS
/// specification or beatoraja.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BmsConditionVariable {
    /// EX score.
    Score,
    /// Score rate percentage.
    Rate,
    /// Current combo.
    Combo,
    /// Maximum combo.
    MaxCombo,
    /// Perfect great count.
    PGreat,
    /// Great count.
    Great,
    /// Good count.
    Good,
    /// Bad count.
    Bad,
    /// Poor count.
    Poor,
    /// Miss count (bad poor).
    Miss,
    /// Gauge percentage.
    Gauge,
    /// Best clear lamp.
    Lamp,
    /// Auto-play flag (1 when auto is on).
    Auto,
}

impl BmsConditionVariable {
    /// Return the uppercase source spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Score => "SCORE",
            Self::Rate => "RATE",
            Self::Combo => "COMBO",
            Self::MaxCombo => "MAXCOMBO",
            Self::PGreat => "PGREAT",
            Self::Great => "GREAT",
            Self::Good => "GOOD",
            Self::Bad => "BAD",
            Self::Poor => "POOR",
            Self::Miss => "MISS",
            Self::Gauge => "GAUGE",
            Self::Lamp => "LAMP",
            Self::Auto => "AUTO",
        }
    }
}

impl fmt::Display for BmsConditionVariable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Comparison operator in a `#WHEN` / `#ELSEWHEN` condition.
///
/// `KaleidBeat` extension: these operators are not part of the BMS
/// specification or beatoraja.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BmsConditionOperator {
    /// `>`.
    Greater,
    /// `>=`.
    GreaterEqual,
    /// `<`.
    Less,
    /// `<=`.
    LessEqual,
    /// `==`.
    Equal,
    /// `!=`.
    NotEqual,
}

impl BmsConditionOperator {
    /// Return the source spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Equal => "==",
            Self::NotEqual => "!=",
        }
    }
}

impl fmt::Display for BmsConditionOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One parsed `#WHEN` / `#ELSEWHEN` comparison.
///
/// `KaleidBeat` extension: this condition syntax is not part of the BMS
/// specification or beatoraja.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BmsCondition {
    /// Compared live value.
    pub variable: BmsConditionVariable,
    /// Comparison operator.
    pub operator: BmsConditionOperator,
    /// Right-hand side, a finite decimal.
    pub value: f64,
}

impl BmsCondition {
    /// Parse `VARIABLE<op>VALUE`, case-insensitive for the variable.
    ///
    /// Returns `None` for an unknown variable, a missing operator, or a
    /// non-finite value.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let trimmed = text.trim();
        for (operator, spelling) in [
            (BmsConditionOperator::GreaterEqual, ">="),
            (BmsConditionOperator::LessEqual, "<="),
            (BmsConditionOperator::Equal, "=="),
            (BmsConditionOperator::NotEqual, "!="),
            (BmsConditionOperator::Greater, ">"),
            (BmsConditionOperator::Less, "<"),
        ] {
            let Some((name, value)) = trimmed.split_once(spelling) else {
                continue;
            };
            let variable = match name.trim().to_ascii_uppercase().as_str() {
                "SCORE" => BmsConditionVariable::Score,
                "RATE" => BmsConditionVariable::Rate,
                "COMBO" => BmsConditionVariable::Combo,
                "MAXCOMBO" => BmsConditionVariable::MaxCombo,
                "PGREAT" => BmsConditionVariable::PGreat,
                "GREAT" => BmsConditionVariable::Great,
                "GOOD" => BmsConditionVariable::Good,
                "BAD" => BmsConditionVariable::Bad,
                "POOR" => BmsConditionVariable::Poor,
                "MISS" => BmsConditionVariable::Miss,
                "GAUGE" => BmsConditionVariable::Gauge,
                "LAMP" => BmsConditionVariable::Lamp,
                "AUTO" => BmsConditionVariable::Auto,
                _ => return None,
            };
            let parsed: f64 = value.trim().parse().ok()?;
            if !parsed.is_finite() {
                return None;
            }
            return Some(Self {
                variable,
                operator,
                value: parsed,
            });
        }
        None
    }
}

impl fmt::Display for BmsCondition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}{}", self.variable, self.operator, self.value)
    }
}

impl FromStr for BmsCondition {
    type Err = ParseBmsValueError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or(ParseBmsValueError("conditional"))
    }
}

/// Structure of a `KaleidBeat` runtime conditional block.
///
/// `KaleidBeat` extension: the block syntax is not part of the BMS
/// specification or beatoraja.
#[derive(Debug, Clone, PartialEq, BmsTokenAttr)]
pub enum BmsHeaderConditional {
    /// `#CONDITIONAL <measure>`: evaluation position, with a fractional measure.
    #[bms_token("#CONDITIONAL {}")]
    Conditional(f64),
    /// `#WHEN <condition>`: first branch.
    #[bms_token("#WHEN {}")]
    When(BmsCondition),
    /// `#ELSEWHEN <condition>`: subsequent branch.
    #[bms_token("#ELSEWHEN {}")]
    ElseWhen(BmsCondition),
    /// `#DEFAULT`: required fallback branch.
    #[bms_token("#DEFAULT")]
    Default,
    /// `#ENDCONDITIONAL`: closes the block.
    #[bms_token("#ENDCONDITIONAL")]
    EndConditional,
}
