#[derive(Debug, Default, Clone)]
pub struct Region {
    pub name: &'static str,
    pub code: &'static str,
    pub group: &'static str,
    pub opt_in: bool,
    pub latency_ms: Option<u64>,
}

impl Region {
    pub fn all() -> Vec<Self> {
        vec![
            Region {
                name: "N. Virginia",
                code: "us-east-1",
                group: "United States",
                ..Default::default()
            },
            Region {
                name: "Ohio",
                code: "us-east-2",
                group: "United States",
                ..Default::default()
            },
            Region {
                name: "N. California",
                code: "us-west-1",
                group: "United States",
                ..Default::default()
            },
            Region {
                name: "Oregon",
                code: "us-west-2",
                group: "United States",
                ..Default::default()
            },
            Region {
                name: "Cape Town",
                code: "af-south-1",
                group: "Africa",
                opt_in: true,
                ..Default::default()
            },
            Region {
                name: "Hong Kong",
                code: "ap-east-1",
                group: "Asia Pacific",
                opt_in: true,
                ..Default::default()
            },
            Region {
                name: "Hyderabad",
                code: "ap-south-2",
                group: "Asia Pacific",
                opt_in: true,
                ..Default::default()
            },
            Region {
                name: "Jakarta",
                code: "ap-southeast-3",
                group: "Asia Pacific",
                opt_in: true,
                ..Default::default()
            },
            Region {
                name: "Malaysia",
                code: "ap-southeast-5",
                group: "Asia Pacific",
                opt_in: true,
                ..Default::default()
            },
            Region {
                name: "Melbourne",
                code: "ap-southeast-4",
                group: "Asia Pacific",
                opt_in: true,
                ..Default::default()
            },
            Region {
                name: "Mumbai",
                code: "ap-south-1",
                group: "Asia Pacific",
                ..Default::default()
            },
            Region {
                name: "New Zealand",
                code: "ap-southeast-6",
                group: "Asia Pacific",
                opt_in: true,
                ..Default::default()
            },
            Region {
                name: "Osaka",
                code: "ap-northeast-3",
                group: "Asia Pacific",
                ..Default::default()
            },
            Region {
                name: "Seoul",
                code: "ap-northeast-2",
                group: "Asia Pacific",
                ..Default::default()
            },
            Region {
                name: "Singapore",
                code: "ap-southeast-1",
                group: "Asia Pacific",
                ..Default::default()
            },
            Region {
                name: "Sydney",
                code: "ap-southeast-2",
                group: "Asia Pacific",
                ..Default::default()
            },
            Region {
                name: "Taipei",
                code: "ap-east-2",
                group: "Asia Pacific",
                opt_in: true,
                ..Default::default()
            },
            Region {
                name: "Thailand",
                code: "ap-southeast-7",
                group: "Asia Pacific",
                opt_in: true,
                ..Default::default()
            },
            Region {
                name: "Tokyo",
                code: "ap-northeast-1",
                group: "Asia Pacific",
                ..Default::default()
            },
            Region {
                name: "Central",
                code: "ca-central-1",
                group: "Canada",
                ..Default::default()
            },
            Region {
                name: "Calgary",
                code: "ca-west-1",
                group: "Canada West",
                opt_in: true,
                ..Default::default()
            },
            Region {
                name: "Frankfurt",
                code: "eu-central-1",
                group: "Europe",
                ..Default::default()
            },
            Region {
                name: "Ireland",
                code: "eu-west-1",
                group: "Europe",
                ..Default::default()
            },
            Region {
                name: "London",
                code: "eu-west-2",
                group: "Europe",
                ..Default::default()
            },
            Region {
                name: "Milan",
                code: "eu-south-1",
                group: "Europe",
                opt_in: true,
                ..Default::default()
            },
            Region {
                name: "Paris",
                code: "eu-west-3",
                group: "Europe",
                ..Default::default()
            },
            Region {
                name: "Spain",
                code: "eu-south-2",
                group: "Europe",
                opt_in: true,
                ..Default::default()
            },
            Region {
                name: "Stockholm",
                code: "eu-north-1",
                group: "Europe",
                ..Default::default()
            },
            Region {
                name: "Zurich",
                code: "eu-central-2",
                group: "Europe",
                opt_in: true,
                ..Default::default()
            },
            Region {
                name: "Tel Aviv",
                code: "il-central-1",
                group: "Israel",
                opt_in: true,
                ..Default::default()
            },
            Region {
                name: "Central",
                code: "mx-central-1",
                group: "Mexico",
                opt_in: true,
                ..Default::default()
            },
            Region {
                name: "Bahrain",
                code: "me-south-1",
                group: "Middle East",
                opt_in: true,
                ..Default::default()
            },
            Region {
                name: "UAE",
                code: "me-central-1",
                group: "Middle East",
                opt_in: true,
                ..Default::default()
            },
            Region {
                name: "São Paulo",
                code: "sa-east-1",
                group: "South America",
                ..Default::default()
            },
        ]
    }
}
