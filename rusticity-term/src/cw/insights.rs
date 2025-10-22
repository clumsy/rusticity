// Re-export types from ui/cw/insights
pub use crate::ui::cw::insights::{
    DateRangeType, InsightsFocus, QueryLanguage, State as InsightsState, TimeUnit,
};

pub fn console_url(region: &str, account_id: &str, query: &str, log_groups: &[String]) -> String {
    let mut url = format!(
        "https://{}.console.aws.amazon.com/cloudwatch/home?region={}#logsV2:logs-insights$3FqueryDetail$3D~(end~0~start~-3600~timeType~'RELATIVE~tz~'UTC~unit~'seconds",
        region, region
    );

    if !query.is_empty() {
        let encoded_query = query
            .replace(" ", "*20")
            .replace("\n", "*0a")
            .replace("|", "*7c")
            .replace(",", "*2c")
            .replace("@", "*40");
        url.push_str(&format!("~editorString~'{}", encoded_query));
    } else {
        url.push_str("~editorString~'fields*20*40timestamp*2c*20*40message*2c*20*40logStream*2c*20*40log*0a*7c*20sort*20*40timestamp*20desc*0a*7c*20limit*2010000");
    }

    url.push_str("~source~(");
    if !log_groups.is_empty() {
        for (i, group) in log_groups.iter().enumerate() {
            if i > 0 {
                url.push('~');
            }
            let encoded_group = group.replace("/", "*2f").replace(":", "*3a");
            url.push_str(&format!(
                "~'arn*3aaws*3alogs*3a{}*3a{}*3alog-group*3a{}",
                region, account_id, encoded_group
            ));
        }
    }
    url.push(')');
    url.push_str("~lang~'CWLI)$26tab$3Dlogs");
    url
}
