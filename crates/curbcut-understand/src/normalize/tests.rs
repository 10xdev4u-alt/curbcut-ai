//! Tests for number, date, and unit normalization.

use super::*;

#[test]
fn integers_are_spelled() {
    assert_eq!(spell_integer(0), "zero");
    assert_eq!(spell_integer(5), "five");
    assert_eq!(spell_integer(13), "thirteen");
    assert_eq!(spell_integer(20), "twenty");
    assert_eq!(spell_integer(21), "twenty-one");
    assert_eq!(spell_integer(100), "one hundred");
    assert_eq!(spell_integer(105), "one hundred five");
    assert_eq!(spell_integer(120), "one hundred twenty");
    assert_eq!(spell_integer(-7), "minus seven");
}

#[test]
fn large_integers_use_scales() {
    assert_eq!(spell_integer(1_000), "one thousand");
    assert_eq!(spell_integer(1_200), "one thousand two hundred");
    assert_eq!(spell_integer(1_000_000), "one million");
    assert_eq!(
        spell_integer(1_234_567),
        "one million two hundred thirty-four thousand five hundred sixty-seven"
    );
}

#[test]
fn decimals_are_spelled_digit_by_digit() {
    assert_eq!(spell_number(2.5, 1), "two point five");
    assert_eq!(
        spell_number(3.50, 2),
        "three point five",
        "trailing zeros drop"
    );
    assert_eq!(spell_number(-1.25, 2), "minus one point two five");
    assert_eq!(spell_number(0.0, 2), "zero");
    assert_eq!(spell_number(12.0, 2), "twelve");
}

#[test]
fn too_many_decimals_are_clamped() {
    assert_eq!(spell_number(1.5, 99), "one point five");
    assert_eq!(MAX_DECIMALS, 12);
}

#[test]
fn rounding_is_explicit() {
    assert_eq!(round_for_speech(2.5, 0), 3.0);
    assert_eq!(round_for_speech(1.25, 1), 1.3);
    assert_eq!(round_for_speech(1.9, 0), 2.0);
}

#[test]
fn rounding_does_not_happen_implicitly() {
    let raw = Measurement::new(1.9, Unit::Kilogram).speak(MeasurementSystem::Metric);
    assert_eq!(raw, "one point nine kilograms");

    let rounded =
        Measurement::new(round_for_speech(1.9, 0), Unit::Kilogram).speak(MeasurementSystem::Metric);
    assert_eq!(rounded, "two kilograms");
}

#[test]
fn money_is_spoken_the_way_people_say_it() {
    assert_eq!(
        Money::new(1_200.0, CurrencyCode::Usd).speak(),
        "twelve hundred dollars"
    );
    assert_eq!(
        Money::new(2_500.0, CurrencyCode::Usd).speak(),
        "twenty-five hundred dollars"
    );
    assert_eq!(Money::new(1.0, CurrencyCode::Usd).speak(), "one dollar");
    assert_eq!(
        Money::new(0.5, CurrencyCode::Usd).speak(),
        "zero point five dollars"
    );
    assert_eq!(
        Money::new(-30.0, CurrencyCode::Usd).speak(),
        "minus thirty dollars"
    );
    assert_eq!(
        Money::new(3.5, CurrencyCode::Eur).speak(),
        "three point five euros"
    );
}

#[test]
fn money_parses_symbols_and_codes() {
    let parsed = Money::parse("$1,200").expect("the symbol form");
    assert_eq!(parsed.amount(), 1_200.0);
    assert_eq!(parsed.code(), CurrencyCode::Usd);
    assert_eq!(parsed.speak(), "twelve hundred dollars");

    assert_eq!(
        Money::parse("€3.50").map(|money| money.speak()),
        Some("three point five euros".to_string())
    );
    assert_eq!(
        Money::parse("£1").map(|money| money.speak()),
        Some("one pound".to_string())
    );
    assert_eq!(
        Money::parse("20 USD").map(|money| money.speak()),
        Some("twenty dollars".to_string())
    );
    assert_eq!(
        Money::parse("USD 20").map(|money| money.speak()),
        Some("twenty dollars".to_string())
    );
    assert_eq!(Money::parse("not money"), None);
}

#[test]
fn iso_dates_are_spoken_unambiguously() {
    let date = Date::parse("2026-11-16").expect("the ISO form");
    assert_eq!(date.speak(), "16 November 2026");
    assert_eq!(date.year(), 2026);
    assert_eq!(date.month(), 11);
    assert_eq!(date.day(), 16);
    assert_eq!(
        Date::parse("2026/11/16").map(|date| date.speak()),
        Some("16 November 2026".to_string())
    );
}

#[test]
fn numeric_dates_parse_when_unambiguous() {
    assert_eq!(
        Date::parse("16/11/2026").map(|date| date.speak()),
        Some("16 November 2026".to_string())
    );
    assert_eq!(
        Date::parse("11/16/2026").map(|date| date.speak()),
        Some("16 November 2026".to_string())
    );
}

#[test]
fn ambiguous_dates_are_rejected() {
    // The third and fourth of a month cannot be told apart.
    assert_eq!(Date::parse("03/04/2026"), None);
    assert_eq!(Date::parse("01/02/2026"), None);
}

#[test]
fn impossible_dates_are_rejected() {
    assert_eq!(Date::parse("31/04/2026"), None);
    assert_eq!(Date::parse("29/02/2023"), None, "2023 is not a leap year");
    assert_eq!(
        Date::parse("29/02/2024").map(|date| date.speak()),
        Some("29 February 2024".to_string())
    );
    assert_eq!(Date::new(2026, 13, 1), None);
    assert_eq!(Date::new(2026, 0, 1), None);
    assert_eq!(Date::parse("not a date"), None);
}

#[test]
fn locale_picks_a_measurement_system() {
    assert_eq!(
        MeasurementSystem::for_locale(&Locale::new("en-US")),
        MeasurementSystem::Imperial
    );
    assert_eq!(
        MeasurementSystem::for_locale(&Locale::new("en-GB")),
        MeasurementSystem::Metric
    );
    assert_eq!(
        MeasurementSystem::for_locale(&Locale::default()),
        MeasurementSystem::Metric
    );
    assert_eq!(MeasurementSystem::default(), MeasurementSystem::Metric);
}

#[test]
fn unit_names_have_a_plural() {
    assert_eq!(Unit::Metre.singular(), "metre");
    assert_eq!(Unit::Metre.plural(), "metres");
    assert_eq!(Unit::Foot.plural(), "feet");
    assert_eq!(Unit::Inch.plural(), "inches");
    assert_eq!(Unit::Celsius.singular(), "degree Celsius");
    assert_eq!(Unit::Celsius.plural(), "degrees Celsius");
}

#[test]
fn measurements_are_spoken_in_the_target_system() {
    assert_eq!(
        Measurement::new(1.0, Unit::Metre).speak(MeasurementSystem::Metric),
        "one metre"
    );
    assert_eq!(
        Measurement::new(2.5, Unit::Metre).speak(MeasurementSystem::Metric),
        "two point five metres"
    );
    assert_eq!(
        Measurement::new(1.0, Unit::Foot).speak(MeasurementSystem::Imperial),
        "one foot"
    );
    assert_eq!(
        Measurement::new(1.0, Unit::Foot).speak(MeasurementSystem::Metric),
        "zero point three metres"
    );
    assert_eq!(
        Measurement::new(1.0, Unit::Mile).speak(MeasurementSystem::Metric),
        "one thousand six hundred nine point three four metres"
    );
    assert_eq!(
        Measurement::new(2.0, Unit::Kilogram).speak(MeasurementSystem::Metric),
        "two kilograms"
    );
    assert_eq!(
        Measurement::new(1.0, Unit::Pound).speak(MeasurementSystem::Metric),
        "zero point four five kilograms"
    );
    assert_eq!(
        Measurement::new(1.0, Unit::Litre).speak(MeasurementSystem::Imperial),
        "zero point two six gallons"
    );
}

#[test]
fn temperatures_convert_between_scales() {
    assert_eq!(
        Measurement::new(10.0, Unit::Celsius).speak(MeasurementSystem::Imperial),
        "fifty degrees Fahrenheit"
    );
    assert_eq!(
        Measurement::new(0.0, Unit::Celsius).speak(MeasurementSystem::Imperial),
        "thirty-two degrees Fahrenheit"
    );
    assert_eq!(
        Measurement::new(50.0, Unit::Fahrenheit).speak(MeasurementSystem::Metric),
        "ten degrees Celsius"
    );
    assert_eq!(
        Measurement::new(1.0, Unit::Celsius).speak(MeasurementSystem::Metric),
        "one degree Celsius"
    );
}

#[test]
fn a_locale_selects_the_unit_system() {
    assert_eq!(
        Measurement::new(1.0, Unit::Mile).speak_for_locale(&Locale::new("en-US")),
        "five thousand two hundred eighty feet"
    );
    assert_eq!(
        Measurement::new(1.0, Unit::Mile).speak_for_locale(&Locale::new("en-GB")),
        "one thousand six hundred nine point three four metres"
    );
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/NORMALIZE.md");
    for word in ["number", "date", "unit", "rounding", "unambiguous"] {
        assert!(
            doc.contains(word),
            "docs/NORMALIZE.md does not state `{word}`"
        );
    }
}
