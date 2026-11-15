//! Number, date, and unit normalization.
//!
//! Numbers become words, dates become unambiguous, and measurements become a
//! unit the listener expects. See
//! [`../../../docs/NORMALIZE.md`](../../../docs/NORMALIZE.md).
//!
//! Four rules hold:
//!
//! - **Money is spoken the way people say it.** Twelve hundred, not one
//!   thousand two hundred.
//! - **A date is unambiguous or it is rejected.** `2026-11-16` and `16/11/2026`
//!   parse; `03/04/2026` does not, because the month and the day cannot be
//!   told apart.
//! - **Rounding never changes meaning.** Rounding is explicit, in
//!   [`round_for_speech`], and the caller chooses a precision that preserves
//!   the quantity.
//! - **Nothing is retained.** Every function is pure; the text it reads is not
//!   stored.

use crate::jargon::Locale;

/// The most decimal places [`spell_number`] and [`round_for_speech`] accept.
pub const MAX_DECIMALS: u32 = 12;

const ONES: [&str; 20] = [
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
];

const TENS: [&str; 10] = [
    "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
];

const SCALES: [&str; 6] = [
    "",
    "thousand",
    "million",
    "billion",
    "trillion",
    "quadrillion",
];

/// Spells an integer in words.
pub fn spell_integer(value: i64) -> String {
    if value == 0 {
        return "zero".to_string();
    }
    let negative = value < 0;
    let mut remaining = value.unsigned_abs();
    let mut parts: Vec<String> = Vec::new();
    let mut scale = 0_usize;
    while remaining > 0 {
        let chunk = remaining % 1_000;
        if chunk > 0 {
            let mut text = under_1000(chunk);
            if !SCALES[scale].is_empty() {
                text.push(' ');
                text.push_str(SCALES[scale]);
            }
            parts.push(text);
        }
        remaining /= 1_000;
        scale += 1;
    }
    parts.reverse();
    let text = parts.join(" ");
    if negative {
        format!("minus {text}")
    } else {
        text
    }
}

/// Spells a number under one thousand.
fn under_1000(value: u64) -> String {
    let hundreds = value / 100;
    let rest = value % 100;
    let mut text = String::new();
    if hundreds > 0 {
        text.push_str(ONES[hundreds as usize]);
        text.push_str(" hundred");
    }
    match rest {
        0 => {}
        1..=19 => {
            if hundreds > 0 {
                text.push(' ');
            }
            text.push_str(ONES[rest as usize]);
        }
        _ => {
            if hundreds > 0 {
                text.push(' ');
            }
            text.push_str(TENS[(rest / 10) as usize]);
            let unit = rest % 10;
            if unit > 0 {
                text.push('-');
                text.push_str(ONES[unit as usize]);
            }
        }
    }
    text
}

/// The word for a single digit.
fn digit_word(digit: char) -> &'static str {
    match digit {
        '0' => "zero",
        '1' => "one",
        '2' => "two",
        '3' => "three",
        '4' => "four",
        '5' => "five",
        '6' => "six",
        '7' => "seven",
        '8' => "eight",
        _ => "nine",
    }
}

/// Spells a number with a fixed number of decimal places.
///
/// Trailing zeros in the fraction are dropped, so `3.50` is "three point five".
pub fn spell_number(value: f64, decimals: u32) -> String {
    let decimals = decimals.min(MAX_DECIMALS);
    let negative = value < 0.0;
    let factor = 10_f64.powi(i32::try_from(decimals).unwrap_or(i32::MAX));
    let scaled = (value.abs() * factor).round() as u64;
    let divisor = 10_u64.pow(decimals);
    let whole = scaled / divisor;
    let fraction = scaled % divisor;

    let mut text = spell_integer(whole as i64);
    if decimals > 0 && fraction > 0 {
        let mut digits = format!("{fraction:0width$}", width = decimals as usize);
        while digits.ends_with('0') {
            digits.pop();
        }
        text.push_str(" point ");
        let mut spoken = String::new();
        for digit in digits.chars() {
            spoken.push_str(digit_word(digit));
            spoken.push(' ');
        }
        spoken.pop();
        text.push_str(&spoken);
    }
    if negative {
        format!("minus {text}")
    } else {
        text
    }
}

/// Rounds a number for speech.
///
/// The caller chooses the precision. Rounding to a whole unit of the same
/// quantity keeps the meaning; rounding away a unit changes it, so the choice
/// belongs to the caller and is never made implicitly.
pub fn round_for_speech(value: f64, decimals: u32) -> f64 {
    let decimals = decimals.min(MAX_DECIMALS);
    let factor = 10_f64.powi(i32::try_from(decimals).unwrap_or(i32::MAX));
    (value * factor).round() / factor
}

/// A currency the normalizer can spell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CurrencyCode {
    /// United States dollar.
    Usd,
    /// Euro.
    Eur,
    /// British pound.
    Gbp,
}

impl CurrencyCode {
    /// Every supported currency.
    pub const ALL: [CurrencyCode; 3] = [Self::Usd, Self::Eur, Self::Gbp];

    /// The symbol.
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Usd => "$",
            Self::Eur => "€",
            Self::Gbp => "£",
        }
    }

    /// The ISO code.
    pub fn code(self) -> &'static str {
        match self {
            Self::Usd => "USD",
            Self::Eur => "EUR",
            Self::Gbp => "GBP",
        }
    }

    /// The singular name.
    pub fn singular(self) -> &'static str {
        match self {
            Self::Usd => "dollar",
            Self::Eur => "euro",
            Self::Gbp => "pound",
        }
    }

    /// The plural name.
    pub fn plural(self) -> &'static str {
        match self {
            Self::Usd => "dollars",
            Self::Eur => "euros",
            Self::Gbp => "pounds",
        }
    }
}

/// An amount of money.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Money {
    amount: f64,
    code: CurrencyCode,
}

impl Money {
    /// Creates an amount.
    pub fn new(amount: f64, code: CurrencyCode) -> Self {
        Self { amount, code }
    }

    /// Parses an amount with a currency symbol or code.
    pub fn parse(input: &str) -> Option<Self> {
        let trimmed = input.trim();
        for code in CurrencyCode::ALL {
            if let Some(rest) = trimmed.strip_prefix(code.symbol()) {
                return parse_amount(rest).map(|amount| Self::new(amount, code));
            }
        }
        let upper = trimmed.to_uppercase();
        for code in CurrencyCode::ALL {
            if let Some(rest) = upper.strip_suffix(code.code()) {
                return parse_amount(rest).map(|amount| Self::new(amount, code));
            }
            if let Some(rest) = upper.strip_prefix(code.code()) {
                return parse_amount(rest).map(|amount| Self::new(amount, code));
            }
        }
        None
    }

    /// The numeric amount.
    pub fn amount(&self) -> f64 {
        self.amount
    }

    /// The currency.
    pub fn code(&self) -> CurrencyCode {
        self.code
    }

    /// The amount spoken in words.
    pub fn speak(&self) -> String {
        let negative = self.amount < 0.0;
        let magnitude = self.amount.abs();
        let name = if magnitude == 1.0 {
            self.code.singular()
        } else {
            self.code.plural()
        };
        let amount = if magnitude.fract() == 0.0
            && (100.0..10_000.0).contains(&magnitude)
            && (magnitude / 100.0).fract() == 0.0
        {
            format!("{} hundred", spell_integer((magnitude / 100.0) as i64))
        } else if magnitude.fract() == 0.0 {
            spell_integer(magnitude as i64)
        } else {
            spell_number(magnitude, 2)
        };
        let text = format!("{amount} {name}");
        if negative {
            format!("minus {text}")
        } else {
            text
        }
    }
}

/// Parses a number, ignoring commas and whitespace.
fn parse_amount(input: &str) -> Option<f64> {
    let cleaned: String = input
        .chars()
        .filter(|ch| *ch != ',' && !ch.is_whitespace())
        .collect();
    let value = cleaned.parse::<f64>().ok()?;
    value.is_finite().then_some(value)
}

/// A calendar date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Date {
    year: i32,
    month: u32,
    day: u32,
}

impl Date {
    /// Creates a date, rejecting an impossible day.
    pub fn new(year: i32, month: u32, day: u32) -> Option<Self> {
        if !(1..=9_999).contains(&year) || !(1..=12).contains(&month) {
            return None;
        }
        if day == 0 || day > days_in_month(year, month) {
            return None;
        }
        Some(Self { year, month, day })
    }

    /// Parses a date. Ambiguous numeric dates are rejected.
    ///
    /// Accepted: `2026-11-16`, `2026/11/16`, `16/11/2026`, `11/16/2026`, and the
    /// same with `.` separators. `03/04/2026` is rejected because the day and
    /// month cannot be told apart.
    pub fn parse(input: &str) -> Option<Self> {
        let parts: Vec<&str> = input.trim().split(['-', '/', '.']).collect();
        if parts.len() != 3 {
            return None;
        }
        let first = parts[0].parse::<u32>().ok()?;
        let second = parts[1].parse::<u32>().ok()?;
        let third = parts[2].parse::<u32>().ok()?;
        if first > 31 {
            // Year first: YYYY-MM-DD.
            return Self::new(i32::try_from(first).ok()?, second, third);
        }
        if third > 31 {
            // Year last. Only one order is unambiguous.
            if first > 12 {
                return Self::new(i32::try_from(third).ok()?, second, first);
            }
            if second > 12 {
                return Self::new(i32::try_from(third).ok()?, first, second);
            }
            return None;
        }
        None
    }

    /// The year.
    pub fn year(&self) -> i32 {
        self.year
    }

    /// The month, `1..=12`.
    pub fn month(&self) -> u32 {
        self.month
    }

    /// The day of the month.
    pub fn day(&self) -> u32 {
        self.day
    }

    /// The date spoken unambiguously, for example `16 November 2026`.
    pub fn speak(&self) -> String {
        format!(
            "{} {} {}",
            self.day,
            month_name(self.month).unwrap_or("unknown"),
            self.year
        )
    }
}

/// The English month name.
pub fn month_name(month: u32) -> Option<&'static str> {
    Some(match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => return None,
    })
}

/// Days in a month, with a leap-year February.
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Whether a year is a leap year.
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// The unit system a locale expects.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum MeasurementSystem {
    /// Metres, kilograms, litres, Celsius.
    #[default]
    Metric,
    /// Feet, pounds, gallons, Fahrenheit.
    Imperial,
}

impl MeasurementSystem {
    /// The system a locale expects. Only `en-US` is imperial.
    pub fn for_locale(locale: &Locale) -> Self {
        if locale.as_str() == "en-us" {
            Self::Imperial
        } else {
            Self::Metric
        }
    }
}

/// What a unit measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Quantity {
    /// Length.
    Length,
    /// Mass.
    Mass,
    /// Volume.
    Volume,
    /// Temperature.
    Temperature,
}

/// A unit the normalizer can speak.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Unit {
    /// Millimetre.
    Millimetre,
    /// Centimetre.
    Centimetre,
    /// Metre.
    Metre,
    /// Kilometre.
    Kilometre,
    /// Inch.
    Inch,
    /// Foot.
    Foot,
    /// Mile.
    Mile,
    /// Gram.
    Gram,
    /// Kilogram.
    Kilogram,
    /// Pound.
    Pound,
    /// Millilitre.
    Millilitre,
    /// Litre.
    Litre,
    /// Gallon.
    Gallon,
    /// Degree Celsius.
    Celsius,
    /// Degree Fahrenheit.
    Fahrenheit,
}

impl Unit {
    /// Every supported unit.
    pub const ALL: [Unit; 15] = [
        Self::Millimetre,
        Self::Centimetre,
        Self::Metre,
        Self::Kilometre,
        Self::Inch,
        Self::Foot,
        Self::Mile,
        Self::Gram,
        Self::Kilogram,
        Self::Pound,
        Self::Millilitre,
        Self::Litre,
        Self::Gallon,
        Self::Celsius,
        Self::Fahrenheit,
    ];

    /// What the unit measures.
    pub fn quantity(self) -> Quantity {
        match self {
            Self::Millimetre | Self::Centimetre | Self::Metre | Self::Kilometre => Quantity::Length,
            Self::Inch | Self::Foot | Self::Mile => Quantity::Length,
            Self::Gram | Self::Kilogram | Self::Pound => Quantity::Mass,
            Self::Millilitre | Self::Litre | Self::Gallon => Quantity::Volume,
            Self::Celsius | Self::Fahrenheit => Quantity::Temperature,
        }
    }

    /// The singular name.
    pub fn singular(self) -> &'static str {
        match self {
            Self::Millimetre => "millimetre",
            Self::Centimetre => "centimetre",
            Self::Metre => "metre",
            Self::Kilometre => "kilometre",
            Self::Inch => "inch",
            Self::Foot => "foot",
            Self::Mile => "mile",
            Self::Gram => "gram",
            Self::Kilogram => "kilogram",
            Self::Pound => "pound",
            Self::Millilitre => "millilitre",
            Self::Litre => "litre",
            Self::Gallon => "gallon",
            Self::Celsius => "degree Celsius",
            Self::Fahrenheit => "degree Fahrenheit",
        }
    }

    /// The plural name.
    pub fn plural(self) -> &'static str {
        match self {
            Self::Millimetre => "millimetres",
            Self::Centimetre => "centimetres",
            Self::Metre => "metres",
            Self::Kilometre => "kilometres",
            Self::Inch => "inches",
            Self::Foot => "feet",
            Self::Mile => "miles",
            Self::Gram => "grams",
            Self::Kilogram => "kilograms",
            Self::Pound => "pounds",
            Self::Millilitre => "millilitres",
            Self::Litre => "litres",
            Self::Gallon => "gallons",
            Self::Celsius => "degrees Celsius",
            Self::Fahrenheit => "degrees Fahrenheit",
        }
    }

    /// The multiplier from this unit to the metric base of its quantity.
    pub fn metric_factor(self) -> f64 {
        match self {
            Self::Millimetre | Self::Gram | Self::Millilitre => 0.001,
            Self::Centimetre => 0.01,
            Self::Metre | Self::Kilogram | Self::Litre | Self::Celsius => 1.0,
            Self::Kilometre => 1_000.0,
            Self::Inch => 0.0254,
            Self::Foot => 0.3048,
            Self::Mile => 1_609.344,
            Self::Pound => 0.453_592_37,
            Self::Gallon => 3.785_411_784,
            Self::Fahrenheit => 1.0,
        }
    }

    /// The metric unit for the quantity.
    pub fn metric_unit(self) -> Unit {
        match self.quantity() {
            Quantity::Length => Unit::Metre,
            Quantity::Mass => Unit::Kilogram,
            Quantity::Volume => Unit::Litre,
            Quantity::Temperature => Unit::Celsius,
        }
    }

    /// The imperial unit for the quantity.
    pub fn imperial_unit(self) -> Unit {
        match self.quantity() {
            Quantity::Length => Unit::Foot,
            Quantity::Mass => Unit::Pound,
            Quantity::Volume => Unit::Gallon,
            Quantity::Temperature => Unit::Fahrenheit,
        }
    }

    /// The target unit for a measurement system.
    pub fn target_unit(self, system: MeasurementSystem) -> Unit {
        match system {
            MeasurementSystem::Metric => self.metric_unit(),
            MeasurementSystem::Imperial => self.imperial_unit(),
        }
    }
}

/// A value in a unit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measurement {
    value: f64,
    unit: Unit,
}

impl Measurement {
    /// Creates a measurement.
    pub fn new(value: f64, unit: Unit) -> Self {
        Self { value, unit }
    }

    /// The numeric value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// The unit.
    pub fn unit(&self) -> Unit {
        self.unit
    }

    /// The measurement spoken in the target unit system.
    pub fn speak(&self, system: MeasurementSystem) -> String {
        let target = self.unit.target_unit(system);
        let value = if self.unit.quantity() == Quantity::Temperature {
            convert_temperature(self.value, self.unit, target)
        } else {
            self.value * self.unit.metric_factor() / target.metric_factor()
        };
        let name = if value == 1.0 {
            target.singular()
        } else {
            target.plural()
        };
        format!("{} {}", spell_number(value, 2), name)
    }

    /// The measurement spoken in the unit system a locale expects.
    pub fn speak_for_locale(&self, locale: &Locale) -> String {
        self.speak(MeasurementSystem::for_locale(locale))
    }
}

/// Converts a temperature between Celsius and Fahrenheit.
fn convert_temperature(value: f64, from: Unit, to: Unit) -> f64 {
    let celsius = match from {
        Unit::Fahrenheit => (value - 32.0) * 5.0 / 9.0,
        _ => value,
    };
    match to {
        Unit::Fahrenheit => celsius * 9.0 / 5.0 + 32.0,
        _ => celsius,
    }
}

#[cfg(test)]
mod tests;
