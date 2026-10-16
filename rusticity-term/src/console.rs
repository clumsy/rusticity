//! Shared AWS console URL assembly. Services own routes and fragment syntax;
//! this module owns hosts and ordinary query encoding. No account/session alias
//! belongs in a console hostname.

pub(crate) use urlencoding::encode as component;

fn domain(region: &str) -> &'static str {
    if region.starts_with("cn-") {
        "amazonaws.cn"
    } else if region.starts_with("us-gov-") {
        "amazonaws-us-gov.com"
    } else {
        "aws.amazon.com"
    }
}

/// Paths and fragments are service-authored route templates. Encode dynamic
/// path/fragment components with `component` exactly once, not the whole route.
/// Query values are always RAW; `query` handles their encoding.
pub(crate) struct ConsoleUrl {
    base: String,
    query: Vec<String>,
    fragment: Option<String>,
}

impl ConsoleUrl {
    pub(crate) fn regional(region: &str, path: &str) -> Self {
        // GovCloud documents a single console entry host; the query selects
        // the region. Do not invent regional or S3 host prefixes there.
        if region.starts_with("us-gov-") {
            return Self::global(region, path);
        }
        Self::new(
            &format!("{}.console.{}", component(region), domain(region)),
            path,
        )
    }

    /// IAM list routes use the global console, with no regional host prefix.
    pub(crate) fn global(region: &str, path: &str) -> Self {
        Self::new(&format!("console.{}", domain(region)), path)
    }

    /// S3 bucket routes use S3's console host rather than the regional host.
    pub(crate) fn s3(region: &str, path: &str) -> Self {
        if region.starts_with("us-gov-") {
            return Self::global(region, path);
        }
        Self::new(&format!("s3.console.{}", domain(region)), path)
    }

    fn new(host: &str, path: &str) -> Self {
        Self {
            base: format!("https://{host}{path}"),
            query: Vec::new(),
            fragment: None,
        }
    }

    pub(crate) fn query(mut self, key: &str, value: &str) -> Self {
        self.query
            .push(format!("{}={}", component(key), component(value)));
        self
    }

    /// Fragment syntax varies by AWS service (including CloudWatch's bespoke
    /// encoding). Preserve it verbatim; ordinary URL encoding here would break it.
    pub(crate) fn fragment(mut self, route: impl Into<String>) -> Self {
        self.fragment = Some(route.into());
        self
    }

    pub(crate) fn finish(self) -> String {
        let mut url = self.base;
        if !self.query.is_empty() {
            url.push('?');
            url.push_str(&self.query.join("&"));
        }
        if let Some(fragment) = self.fragment {
            url.push('#');
            url.push_str(&fragment);
        }
        url
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod detail_tests;
#[cfg(test)]
mod inventory_tests;

/// ARN partition for the regions whose console hosts we support.
pub(crate) fn partition(region: &str) -> &'static str {
    if region.starts_with("cn-") {
        "aws-cn"
    } else if region.starts_with("us-gov-") {
        "aws-us-gov"
    } else {
        "aws"
    }
}

#[cfg(test)]
mod audit_tests;
