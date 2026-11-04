use super::models::{DateOrder, ParsedChat, ParsedMessage, diagnostic};
use super::{Result, error};
use chrono::{NaiveDate, NaiveTime};
use regex::Regex;
use std::{collections::BTreeSet, sync::LazyLock};

static HEADER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:\[(?P<id>\d{1,4}[/.\-]\d{1,2}[/.\-]\d{1,4}),?\s+(?P<it>\d{1,2}:\d{2}(?::\d{2})?(?:\s*[AaPp][Mm])?)\]\s?|(?P<ad>\d{1,4}[/.\-]\d{1,2}[/.\-]\d{1,4}),?\s+(?P<at>\d{1,2}:\d{2}(?::\d{2})?(?:\s*[AaPp][Mm])?)\s+-\s)(?P<body>.*)$").unwrap()
});
static SUSPICIOUS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\[?\d{1,4}[/.\-]\d{1,2}[/.\-]\d{1,4}.*(?: - |\])").unwrap());
static ATTACHED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<attached:\s*([^>]+)>|(?m)^(.+?) \(file attached\)").unwrap());

pub fn decode(bytes: &[u8]) -> Result<(String, String)> {
    let (text, encoding) = if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        if bytes.len() % 2 != 0 {
            return Err(error(
                "invalid_encoding",
                "The UTF-16 export is truncated or invalid.",
            ));
        }
        let little = bytes[0] == 0xff;
        let words: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|pair| {
                if little {
                    u16::from_le_bytes([pair[0], pair[1]])
                } else {
                    u16::from_be_bytes([pair[0], pair[1]])
                }
            })
            .collect();
        (
            String::from_utf16(&words).map_err(|_| {
                error(
                    "invalid_encoding",
                    "The export contains invalid UTF-16. Use a UTF-8 or BOM-marked UTF-16 export.",
                )
            })?,
            if little { "UTF-16 LE" } else { "UTF-16 BE" }.to_string(),
        )
    } else {
        let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
        (
            std::str::from_utf8(bytes)
                .map_err(|_| {
                    error(
                        "invalid_encoding",
                        "The export is not valid UTF-8 or BOM-marked UTF-16.",
                    )
                })?
                .to_owned(),
            "UTF-8".into(),
        )
    };
    if text
        .chars()
        .any(|c| c == '\0' || (c.is_control() && !matches!(c, '\n' | '\r' | '\t')))
    {
        return Err(error(
            "invalid_encoding",
            "The selected text contains binary/control data and is not a supported chat export.",
        ));
    }
    Ok((text.replace("\r\n", "\n").replace('\r', "\n"), encoding))
}

fn date_time(date: &str, time: &str, order: DateOrder) -> (Option<String>, bool, bool) {
    let parts: Vec<_> = date.split(['/', '.', '-']).collect();
    if parts.len() != 3 {
        return (None, false, false);
    }
    let numbers: Option<Vec<u32>> = parts.iter().map(|part| part.parse().ok()).collect();
    let Some(values) = numbers else {
        return (None, false, false);
    };
    let (a, b, c) = (values[0], values[1], values[2]);
    let short_year = parts[0].len() != 4 && parts[2].len() == 2;
    let year = if parts[0].len() == 4 {
        a
    } else if short_year {
        2000 + c
    } else {
        c
    };
    let dmy = NaiveDate::from_ymd_opt(year as i32, b, a);
    let mdy = NaiveDate::from_ymd_opt(year as i32, a, b);
    let ambiguous = parts[0].len() != 4 && a != b && dmy.is_some() && mdy.is_some();
    let date = if parts[0].len() == 4 {
        NaiveDate::from_ymd_opt(a as i32, b, c)
    } else {
        match order {
            DateOrder::DayFirst => dmy,
            DateOrder::MonthFirst => mdy,
            DateOrder::Auto if ambiguous => None,
            DateOrder::Auto => dmy.or(mdy),
        }
    };
    let normalized_time = time.trim().to_ascii_uppercase();
    let time = [
        "%H:%M:%S",
        "%H:%M",
        "%I:%M:%S %p",
        "%I:%M %p",
        "%I:%M:%S%p",
        "%I:%M%p",
    ]
    .iter()
    .find_map(|format| NaiveTime::parse_from_str(&normalized_time, format).ok());
    let local = date
        .zip(time)
        .map(|(date, time)| date.and_time(time).format("%Y-%m-%dT%H:%M:%S").to_string());
    (local, ambiguous && order == DateOrder::Auto, short_year)
}

pub fn parse(bytes: &[u8], order: DateOrder, max_messages: usize) -> Result<ParsedChat> {
    let (text, encoding) = decode(bytes)?;
    let mut result = ParsedChat {
        messages: Vec::new(),
        participants: Vec::new(),
        diagnostics: Vec::new(),
        date_order_required: false,
        encoding,
    };
    let mut recognized = 0;
    for (index, line) in text.split('\n').enumerate() {
        let line_number = index as u32 + 1;
        // Direction marks are ignored only at the start of a header; content is preserved.
        let candidate = line.trim_start_matches(['\u{200e}', '\u{200f}', '\u{feff}']);
        if let Some(header) = HEADER.captures(candidate) {
            recognized += 1;
            let date = header.name("id").or_else(|| header.name("ad")).unwrap();
            let time = header.name("it").or_else(|| header.name("at")).unwrap();
            let original_timestamp = candidate[date.start()..time.end()].to_string();
            let (local, ambiguous, short_year) = date_time(date.as_str(), time.as_str(), order);
            if ambiguous {
                result.date_order_required = true;
                diagnostic(
                    &mut result.diagnostics,
                    "ambiguous_dates",
                    "Choose day-first or month-first dates before importing. No date order has been guessed.",
                    Some(line_number),
                );
            } else if local.is_none() {
                diagnostic(
                    &mut result.diagnostics,
                    "invalid_timestamp",
                    "A timestamp could not be parsed. Its original representation is preserved.",
                    Some(line_number),
                );
            }
            if short_year {
                diagnostic(
                    &mut result.diagnostics,
                    "two_digit_year",
                    "Two-digit years use 2000–2099. Review the preview; original timestamps are retained.",
                    Some(line_number),
                );
            }
            let body = header.name("body").unwrap().as_str();
            let (sender, message, classification) = if let Some((sender, message)) = body
                .split_once(": ")
                .filter(|(sender, _)| !sender.trim().is_empty() && sender.chars().count() <= 256)
            {
                (
                    Some(sender.to_string()),
                    message.to_string(),
                    if message.contains("This message was deleted")
                        || message.contains("You deleted this message")
                    {
                        "deleted"
                    } else {
                        "message"
                    },
                )
            } else {
                let system = [
                    "end-to-end encrypted",
                    "created group",
                    "changed the subject",
                    "added ",
                    "left",
                    "changed this group's",
                ]
                .iter()
                .any(|needle| body.contains(needle));
                if !system {
                    diagnostic(
                        &mut result.diagnostics,
                        "sender_unknown",
                        "Some records have no identifiable sender. Their text is preserved as unknown/system-like records.",
                        Some(line_number),
                    );
                }
                (
                    None,
                    body.to_string(),
                    if system { "system" } else { "unknown" },
                )
            };
            result.messages.push(ParsedMessage {
                sender,
                text: message,
                original_timestamp: Some(original_timestamp),
                local_timestamp: local,
                classification: classification.into(),
                original_record: line.into(),
                media: Vec::new(),
                line_number,
            });
        } else if SUSPICIOUS.is_match(candidate) || result.messages.is_empty() {
            if line.is_empty() && result.messages.is_empty() {
                continue;
            }
            diagnostic(
                &mut result.diagnostics,
                "unparsed_records",
                "Unrecognized records are retained, not discarded. Review parser limitations before confirming.",
                Some(line_number),
            );
            result.messages.push(ParsedMessage {
                sender: None,
                text: line.into(),
                original_timestamp: None,
                local_timestamp: None,
                classification: "unknown".into(),
                original_record: line.into(),
                media: Vec::new(),
                line_number,
            });
        } else if let Some(previous) = result.messages.last_mut() {
            previous.text.push('\n');
            previous.text.push_str(line);
            previous.original_record.push('\n');
            previous.original_record.push_str(line);
        }
        if result.messages.len() > max_messages {
            return Err(error(
                "import_limit",
                "This export exceeds the configured message limit.",
            ));
        }
    }
    if recognized == 0 {
        return Err(error(
            "unsupported_format",
            "No compatible WhatsApp message headers were found. Choose a readable chat export, not an encrypted native backup.",
        ));
    }
    let mut participants = BTreeSet::new();
    let mut references = 0usize;
    for message in &mut result.messages {
        if let Some(sender) = &message.sender {
            participants.insert(sender.clone());
        }
        message.media = ATTACHED
            .captures_iter(&message.text)
            .filter_map(|capture| capture.get(1).or_else(|| capture.get(2)))
            .map(|value| Some(value.as_str().to_string()))
            .collect();
        if message.media.is_empty()
            && [
                "<Media omitted>",
                "image omitted",
                "video omitted",
                "audio omitted",
                "sticker omitted",
                "document omitted",
            ]
            .iter()
            .any(|value| message.text.contains(value))
        {
            message.media.push(None);
        }
        references += message.media.len();
        if references > max_messages {
            return Err(error(
                "import_limit",
                "The export exceeds the configured attachment-reference limit.",
            ));
        }
    }
    result.participants = participants.into_iter().collect();
    diagnostic(
        &mut result.diagnostics,
        "timezone_unknown",
        "Export timestamps have no timezone. Parsed local times are saved without a UTC conversion.",
        None,
    );
    Ok(result)
}
