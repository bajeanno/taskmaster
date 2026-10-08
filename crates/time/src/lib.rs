use std::{
    fmt::Display,
    time::{Duration, SystemTime},
};

const DAYS_IN_JAN: u64 = 31;
const DAYS_IN_FEB: u64 = 28;
const DAYS_IN_FEB_BIS: u64 = 29;
const DAYS_IN_MAR: u64 = 31;
const DAYS_IN_APR: u64 = 30;
const DAYS_IN_MAY: u64 = 31;
const DAYS_IN_JUN: u64 = 30;
const DAYS_IN_JUL: u64 = 31;
const DAYS_IN_AUG: u64 = 31;
const DAYS_IN_SEP: u64 = 30;
const DAYS_IN_OCT: u64 = 31;
const DAYS_IN_NOV: u64 = 30;
const DAYS_IN_DEC: u64 = 31;

const SECS_IN_HOUR: u64 = 3600;
const SECS_IN_MINUTE: u64 = 60;
const HOURS_IN_DAY: u64 = 24;
const DAYS_IN_YEAR: u64 = 365;
const DAYS_IN_LEAP_YEAR: u64 = 366;
const YEAR_0: u64 = 1970;

enum Month {
    Jan,
    Feb,
    Mar,
    Apr,
    May,
    Jun,
    Jul,
    Aug,
    Sep,
    Oct,
    Nov,
    Dec,
}

impl Month {
    fn num(&self) -> u32 {
        match self {
            Month::Jan => 1,
            Month::Feb => 2,
            Month::Mar => 3,
            Month::Apr => 4,
            Month::May => 5,
            Month::Jun => 6,
            Month::Jul => 7,
            Month::Aug => 8,
            Month::Sep => 9,
            Month::Oct => 10,
            Month::Nov => 11,
            Month::Dec => 12,
        }
    }

    fn days(&self, year: u64) -> u64 {
        match self {
            Month::Jan => DAYS_IN_JAN,
            Month::Feb => {
                if year.is_multiple_of(4) {
                    DAYS_IN_FEB
                } else {
                    DAYS_IN_FEB_BIS
                }
            }
            Month::Mar => DAYS_IN_MAR,
            Month::Apr => DAYS_IN_APR,
            Month::May => DAYS_IN_MAY,
            Month::Jun => DAYS_IN_JUN,
            Month::Jul => DAYS_IN_JUL,
            Month::Aug => DAYS_IN_AUG,
            Month::Sep => DAYS_IN_SEP,
            Month::Oct => DAYS_IN_OCT,
            Month::Nov => DAYS_IN_NOV,
            Month::Dec => DAYS_IN_DEC,
        }
    }

    fn from_days(days: &mut u64, year: u64) -> Self {
        for month_num in 1..=12 {
            let month = Self::from_num(month_num);
            if *days < month.days(year) {
                *days += 1;
                return month;
            }
            *days -= month.days(year);
        }
        unreachable!()
    }

    fn from_num(num: u64) -> Self {
        match num {
            1 => Self::Jan,
            2 => Self::Feb,
            3 => Self::Mar,
            4 => Self::Apr,
            5 => Self::May,
            6 => Self::Jun,
            7 => Self::Jul,
            8 => Self::Aug,
            9 => Self::Sep,
            10 => Self::Oct,
            11 => Self::Nov,
            12 => Self::Dec,
            _ => unreachable!(),
        }
    }
}

impl Display for Month {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}", self.num())
    }
}

fn year_len(year: u64) -> u64 {
    if year.is_multiple_of(4) {
        DAYS_IN_LEAP_YEAR
    } else {
        DAYS_IN_YEAR
    }
}

pub fn get_current_time() -> String {
    let mut duration = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();

    let total_days = duration.as_secs() / SECS_IN_HOUR / HOURS_IN_DAY;
    duration -= Duration::from_hours(HOURS_IN_DAY * total_days);

    let mut year = YEAR_0;
    let mut days = total_days;
    while days >= year_len(year) {
        days -= year_len(year);
        year += 1;
    }
    // converting the numbers of days to calendar day representation
    days += 1;

    let month = Month::from_days(&mut days, year);

    let hours = duration.as_secs() / SECS_IN_HOUR;
    duration -= Duration::from_hours(hours);

    let minutes = duration.as_secs() / SECS_IN_MINUTE;
    duration -= Duration::from_mins(minutes);

    let secs = duration.as_secs();

    format!("[{year}-{month}-{days} {hours}h{minutes}m{secs}s]")
}