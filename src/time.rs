use alloc::format;
use alloc::string::String;

fn decompose_unix_timestamp(unix_timestamp: i64) -> (u16, u8, u8, u8, u8, u8) {
    const SECONDS_IN_MINUTE: i64 = 60;
    const SECONDS_IN_HOUR: i64 = 60 * SECONDS_IN_MINUTE;
    const SECONDS_IN_DAY: i64 = 24 * SECONDS_IN_HOUR;
    const DAYS_IN_YEAR: i64 = 365;
    const DAYS_IN_LEAP_YEAR: i64 = 366;

    let mut year: i64 = 1970;
    let mut days_since_epoch = unix_timestamp.div_euclid(SECONDS_IN_DAY);
    let mut remaining_seconds = unix_timestamp.rem_euclid(SECONDS_IN_DAY);

    while days_since_epoch
        >= if is_leap_year(year) {
            DAYS_IN_LEAP_YEAR
        } else {
            DAYS_IN_YEAR
        }
    {
        days_since_epoch -= if is_leap_year(year) {
            DAYS_IN_LEAP_YEAR
        } else {
            DAYS_IN_YEAR
        };
        year += 1;
    }

    while days_since_epoch < 0 {
        year -= 1;
        days_since_epoch += if is_leap_year(year) {
            DAYS_IN_LEAP_YEAR
        } else {
            DAYS_IN_YEAR
        };
    }

    let mut month = 0;
    while days_since_epoch >= days_in_month(year, month) {
        days_since_epoch -= days_in_month(year, month);
        month += 1;
    }

    let day = days_since_epoch + 1;

    let hour = remaining_seconds / SECONDS_IN_HOUR;
    remaining_seconds %= SECONDS_IN_HOUR;
    let minute = remaining_seconds / SECONDS_IN_MINUTE;
    let second = remaining_seconds % SECONDS_IN_MINUTE;

    (
        year as u16,
        month as u8 + 1,
        day as u8,
        hour as u8,
        minute as u8,
        second as u8,
    )
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn days_in_month(year: i64, month: usize) -> i64 {
    const DAYS_IN_MONTH: [i64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

    if month == 1 && is_leap_year(year) {
        29
    } else {
        DAYS_IN_MONTH[month]
    }
}

pub fn format_unix_timestamp(unix_timestamp: i64, pattern: &str) -> String {
    let (year, month, day, hour, minute, second) = decompose_unix_timestamp(unix_timestamp);

    let mut output = String::with_capacity(pattern.len() + 16);
    let mut characters = pattern.chars();

    while let Some(character) = characters.next() {
        if character != '%' {
            output.push(character);
            continue;
        }

        match characters.next() {
            Some('Y') => output.push_str(&format!("{:04}", year)),
            Some('m') => output.push_str(&format!("{:02}", month)),
            Some('d') => output.push_str(&format!("{:02}", day)),
            Some('H') => output.push_str(&format!("{:02}", hour)),
            Some('I') => output.push_str(&format!("{:02}", hour_12(hour))),
            Some('M') => output.push_str(&format!("{:02}", minute)),
            Some('S') => output.push_str(&format!("{:02}", second)),
            Some('p') => output.push_str(if hour < 12 { "AM" } else { "PM" }),
            Some('%') => output.push('%'),
            Some(other) => {
                output.push('%');
                output.push(other);
            }
            None => output.push('%'),
        }
    }

    output
}

const fn hour_12(hour_24: u8) -> u8 {
    match hour_24 % 12 {
        0 => 12,
        value => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_24_hour_time() {
        let timestamp = 13 * 3600 + 5 * 60;
        assert_eq!(format_unix_timestamp(timestamp, "%H:%M"), "13:05");
    }

    #[test]
    fn format_12_hour_time_with_am_pm() {
        let midnight = 0;
        let afternoon = 13 * 3600 + 5 * 60;

        assert_eq!(format_unix_timestamp(midnight, "%I:%M %p"), "12:00 AM");
        assert_eq!(format_unix_timestamp(afternoon, "%I:%M %p"), "01:05 PM");
    }

    #[test]
    fn format_date_and_time() {
        assert_eq!(
            format_unix_timestamp(0, "%Y-%m-%d %H:%M:%S"),
            "1970-01-01 00:00:00"
        );
    }

    #[test]
    fn format_negative_unix_time() {
        assert_eq!(
            format_unix_timestamp(-1, "%Y-%m-%d %H:%M:%S"),
            "1969-12-31 23:59:59"
        );
    }

    #[test]
    fn unix_epoch_is_correct() {
        assert_eq!(decompose_unix_timestamp(0), (1970, 1, 1, 0, 0, 0));
    }

    #[test]
    fn one_second_before_epoch_is_correct() {
        assert_eq!(decompose_unix_timestamp(-1), (1969, 12, 31, 23, 59, 59));
    }

    #[test]
    fn one_day_before_epoch_is_correct() {
        assert_eq!(decompose_unix_timestamp(-86_400), (1969, 12, 31, 0, 0, 0));
    }

    #[test]
    fn leap_day_2024_is_correct() {
        assert_eq!(
            decompose_unix_timestamp(1_709_164_800),
            (2024, 2, 29, 0, 0, 0)
        );
    }

    #[test]
    fn leap_year_rules_are_correct() {
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2023));
    }

    #[test]
    fn february_days_are_correct() {
        assert_eq!(days_in_month(2024, 1), 29);
        assert_eq!(days_in_month(2023, 1), 28);
    }
}
