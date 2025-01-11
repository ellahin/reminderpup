use chrono::TimeDelta;
use sqlx::postgres::types::PgInterval;

pub fn pginterval_to_string(interval: &PgInterval) -> String {
    let mut delta = TimeDelta::microseconds(interval.microseconds);

    let days = delta.num_days();

    delta = delta - TimeDelta::days(days);

    let hours = delta.num_hours();

    delta = delta - TimeDelta::hours(hours);

    let mins = delta.num_minutes();

    let mut res = String::new();

    if days > 0 {
        match days == 1 {
            true => res = format!("{} day ", days),
            false => res = format!("{} days ", days),
        }
    }
    if hours > 0 {
        match hours == 1 {
            true => res = format!("{} hour ", hours),
            false => res = format!("{} hours ", hours),
        }
    }
    if mins > 0 {
        match mins == 1 {
            true => res = format!("{} minute", mins),
            false => res = format!("{} minutes", mins),
        }
    }

    return res;
}

pub fn format_timezone(interval: &PgInterval) -> String {
    let user_delta = TimeDelta::microseconds(interval.microseconds);

    let hour = user_delta.num_hours();

    let min = match (user_delta - TimeDelta::hours(user_delta.num_hours()))
        .num_minutes()
        .is_positive()
    {
        true => (user_delta - TimeDelta::hours(user_delta.num_hours())).num_minutes(),
        false => (user_delta - TimeDelta::hours(user_delta.num_hours())).num_minutes() * -1_i64,
    };

    format!("{}:{:0>#2}", hour, min)
}
