use anyhow::Result;
use time::{
    format_description::well_known::Iso8601, macros::format_description, Date, OffsetDateTime,
};

/// Format a date like `2012-01-20T18:18:21Z` to a `String` in the format `2020-January-19`
pub(crate) fn format_ymd(date: OffsetDateTime) -> String {
    let format = format_description!("[year]-[month]-[day]");
    date.format(&format).unwrap()
}

/// Format a date like `2012-01-20T18:18:21Z` to a `Date`
pub(crate) fn parse_ymd(date: &str) -> Result<Date> {
    let format = format_description!("[year repr:full]-[month]-[day]");
    Ok(Date::parse(date, &format)?)
}

/// Format a date like `2012-01-20T18:18:21Z` to a `OffsetDateTime`
pub(crate) fn parse_iso8601(date: &str) -> Result<OffsetDateTime> {
    Ok(OffsetDateTime::parse(date, &Iso8601::DEFAULT)?)
}

/// Format a date like `2012-01-20T18:18:21Z` to a `String` in the format `2020-January-19`
pub(crate) fn iso8601_to_ymd(date: &str) -> Result<String> {
    let parsed_date = parse_iso8601(date)?;
    Ok(format_ymd(parsed_date))
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    /// Test parsing a date string in the format "2020-January-19"
    fn test_parse_ymd() {
        let date = "2020-10-19";
        let parsed_date = parse_ymd(date).unwrap();
        assert_eq!(parsed_date.year(), 2020);
        assert_eq!(parsed_date.month(), time::Month::October);
        assert_eq!(parsed_date.day(), 19);
    }

    #[test]
    /// Test formatting a date string in the format "2020-January-19"
    fn test_format_ymd() {
        let date = datetime!(2020-01-02 03:04:05 +06:07:08);
        let formatted = format_ymd(date);
        assert_eq!(formatted, "2020-01-02".to_owned());
    }
}
