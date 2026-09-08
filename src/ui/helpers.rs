use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Color;
use rust_decimal::{Decimal, RoundingStrategy};

pub fn format_amount(amount: &Decimal) -> String {
    // Rounding via f64 would go off the binary approximation, not the stored decimal digits.
    // round_dp alone rounds half to even, so ask for the usual currency convention instead.
    let rounded = amount.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
    let s = format!("{:.2}", rounded.abs());
    let (int_part, frac_part) = s.split_once('.').unwrap_or((s.as_str(), "00"));

    let mut formatted_int = String::new();
    for (count, c) in int_part.chars().rev().enumerate() {
        if count > 0 && count % 3 == 0 {
            formatted_int.push(',');
        }
        formatted_int.push(c);
    }

    let formatted_int: String = formatted_int.chars().rev().collect();
    let sign = if rounded.is_sign_negative() && !rounded.is_zero() {
        "-"
    } else {
        ""
    };

    format!("{}{}.{}", sign, formatted_int, frac_part)
}

/// Signed for display, so a gain or a variance reads as one at a glance.
pub fn format_signed_amount(amount: &Decimal) -> String {
    if amount >= &Decimal::ZERO {
        format!("+{}", format_amount(amount))
    } else {
        format_amount(amount)
    }
}

pub fn format_hours(amount: &Decimal, hourly_rate: Option<Decimal>) -> String {
    if let Some(rate) = hourly_rate
        && rate > Decimal::ZERO
    {
        let hours = (amount / rate).round_dp(1);
        return format!("{:.1}h", hours);
    }
    format_amount(amount)
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn month_to_short_str(month: u32) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "?",
    }
}

pub fn month_to_color(month: u32) -> Color {
    match month {
        1 => Color::LightRed,
        2 => Color::LightGreen,
        3 => Color::LightBlue,
        4 => Color::LightYellow,
        5 => Color::LightMagenta,
        6 => Color::LightCyan,
        7 => Color::Red,
        8 => Color::Green,
        9 => Color::Blue,
        10 => Color::Yellow,
        11 => Color::Magenta,
        12 => Color::Cyan,
        _ => Color::White,
    }
}
