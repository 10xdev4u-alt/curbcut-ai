# Number, date, and unit normalization

`crates/curbcut-understand/src/normalize.rs` turns numbers, dates, and
measurements into speakable forms. `CC-044`.

## Rules

1. **Money is spoken the way people say it.** `1200` USD is "twelve hundred
   dollars", not "one thousand two hundred dollars". Whole amounts under ten
   thousand that are an exact number of hundreds use the hundreds form;
   everything else uses the ordinary scale words.
2. **A date is unambiguous or it is rejected.** `2026-11-16`, `2026/11/16`,
   `16/11/2026`, and `11/16/2026` parse. `03/04/2026` does not, because the day
   and the month cannot be told apart. `Date::parse` returns `None` rather than
   guessing an order.
3. **Rounding never changes meaning.** Rounding is explicit in
   `round_for_speech(value, decimals)`. The caller picks a precision that
   preserves the quantity; `Measurement::speak` does not round on its own.
4. **Nothing is retained.** Every function is pure. The text it reads is not
   stored.

## Numbers

- `spell_integer(i64)` — `0` is "zero", `21` is "twenty-one", `1234567` is
  "one million two hundred thirty-four thousand five hundred sixty-seven".
  Negatives are prefixed with "minus".
- `spell_number(value, decimals)` — the integer part in words, then "point" and
  the fraction digit by digit. Trailing zeros drop, so `3.50` is "three point
  five". `MAX_DECIMALS` is 12.
- `round_for_speech(value, decimals)` — explicit rounding for speech.

## Money

`Money::parse` accepts a leading symbol (`$1,200`, `€3.50`, `£1`) or a currency
code before or after the amount (`20 USD`, `USD 20`). `CurrencyCode::{Usd, Eur,
Gbp}` provides the symbol, the ISO code, and the singular and plural names.
Commas and whitespace are ignored; the amount must be finite.

## Dates

`Date::parse` splits on `-`, `/`, or `.`. A field greater than 31 is the year.
With the year last, a field greater than 12 settles the order; when both are 12
or less the date is ambiguous and is rejected. `Date::new` validates the month
and the day, including a leap-year February. `Date::speak` is
`16 November 2026`.

## Measurements

`Unit` covers length, mass, volume, and temperature. `Measurement::speak(system)`
converts to the target unit and spells the value with the singular or plural
name. `MeasurementSystem::for_locale` is imperial only for `en-US`; every other
locale is metric.

| Quantity | Metric | Imperial |
| --- | --- | --- |
| Length | millimetre, centimetre, metre, kilometre | inch, foot, mile |
| Mass | gram, kilogram | pound |
| Volume | millilitre, litre | gallon |
| Temperature | Celsius | Fahrenheit |

Temperatures convert with the affine formulas, not a multiplier:
`F = C·9/5 + 32`.

## Privacy

The module stores nothing. A parsed value lives only as long as the caller
holds it, and no function logs its input. The workspace canary scan covers this
crate.
