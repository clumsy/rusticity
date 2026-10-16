// Re-export types from ui/cw/insights
pub use crate::ui::cw::insights::{
    DateRangeType, InsightsFocus, QueryLanguage, State as InsightsState, TimeUnit,
};

/// String body in CloudWatch's JSURL fragment grammar (not percent encoding).
/// JSURL escapes UTF-16 code units: *hh below 256, **hhhh otherwise, and uses
/// ! for $. Delimiters and literal escape markers must never pass through.
fn encode_fragment_string(value: &str) -> String {
    let mut encoded = String::new();
    for unit in value.encode_utf16() {
        match unit {
            0x30..=0x39 | 0x41..=0x5a | 0x61..=0x7a | 0x5f | 0x2d | 0x2e => {
                encoded.push(char::from_u32(u32::from(unit)).unwrap());
            }
            0x24 => encoded.push('!'),
            0..=0xff => encoded.push_str(&format!("*{unit:02x}")),
            _ => encoded.push_str(&format!("**{unit:04x}")),
        }
    }
    encoded
}

pub fn console_url(region: &str, account_id: &str, query: &str, log_groups: &[String]) -> String {
    let mut route = "logsV2:logs-insights$3FqueryDetail$3D~(end~0~start~-3600~timeType~'RELATIVE~tz~'UTC~unit~'seconds".to_string();
    let query = if query.is_empty() {
        "fields @timestamp, @message, @logStream, @log\n| sort @timestamp desc\n| limit 10000"
    } else {
        query
    };
    route.push_str("~editorString~'");
    route.push_str(&encode_fragment_string(query));
    route.push_str("~source~(");
    if log_groups.is_empty() {
        route.push('~'); // JSURL empty array; () would represent an object.
    } else {
        for group in log_groups {
            let arn = format!(
                "arn:{}:logs:{region}:{account_id}:log-group:{group}",
                crate::console::partition(region)
            );
            // Each string supplies its own leading ~; never insert a second
            // separator between array elements.
            route.push_str("~'");
            route.push_str(&encode_fragment_string(&arn));
        }
    }
    route.push_str(")~lang~'CWLI)$26tab$3Dlogs");
    crate::console::ConsoleUrl::regional(region, "/cloudwatch/home")
        .query("region", region)
        .fragment(route)
        .finish()
}

#[cfg(test)]
mod fragment_tests {
    use super::*;

    #[test]
    fn query_literals_cannot_become_fragment_syntax() {
        assert_eq!(
            encode_fragment_string("a~')*20!$\n雪😀"),
            "a*7e*27*29*2a20*21!*0a**96ea**d83d**de00"
        );
        let url = console_url(
            "us-east-1",
            "123456789012",
            "filter @message like /a~b)/ | limit 1",
            &[],
        );
        assert!(url.contains(
            "~editorString~'filter*20*40message*20like*20*2fa*7eb*29*2f*20*7c*20limit*201"
        ));
        assert!(url.contains("~source~(~)~lang~'CWLI"));
    }

    #[test]
    fn multiple_sources_have_one_separator_and_literal_escapes_are_preserved() {
        let url = console_url(
            "us-east-1",
            "123456789012",
            "",
            &["/one".into(), "/two*2f".into()],
        );
        assert!(url.contains("log-group*3a*2fone~'arn*3aaws*3alogs*3aus-east-1*3a123456789012*3alog-group*3a*2ftwo*2a2f)"));
        assert!(!url.contains("~~'"));
    }
}
