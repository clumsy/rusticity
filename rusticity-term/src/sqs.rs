pub mod pipe;
pub mod queue;
pub mod sub;
pub mod tag;
pub mod trigger;

use std::collections::HashMap;

pub use pipe::EventBridgePipe;
pub use queue::Queue;
pub use sub::SnsSubscription;
pub use tag::QueueTag;
pub use trigger::LambdaTrigger;

pub fn init(i18n: &mut HashMap<String, String>) {
    queue::init(i18n);
    trigger::init(i18n);
    pipe::init(i18n);
    tag::init(i18n);
    sub::init(i18n);
}

pub fn console_url_queues(region: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/sqs/v3/home?region={}#/queues",
        region, region
    )
}

pub fn console_url_queue_detail(region: &str, queue_url: &str) -> String {
    let encoded_url = urlencoding::encode(queue_url);
    format!(
        "https://{}.console.aws.amazon.com/sqs/v3/home?region={}#/queues/{}",
        region, region, encoded_url
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_console_url_queues() {
        assert_eq!(
            console_url_queues("us-east-1"),
            "https://us-east-1.console.aws.amazon.com/sqs/v3/home?region=us-east-1#/queues"
        );
    }

    #[test]
    fn test_console_url_queue_detail() {
        let url = "https://sqs.us-east-1.amazonaws.com/654654343159/MyTest";
        assert_eq!(
            console_url_queue_detail("us-east-1", url),
            "https://us-east-1.console.aws.amazon.com/sqs/v3/home?region=us-east-1#/queues/https%3A%2F%2Fsqs.us-east-1.amazonaws.com%2F654654343159%2FMyTest"
        );
    }
}
