//! Read-only Step Functions metadata inventory. Two list operations only:
//! `ListStateMachines` for state machines and `ListExecutions` for a single
//! state machine's executions. No DescribeExecution / GetExecutionHistory,
//! no state-machine definitions, no execution inputs/outputs, and no mutating
//! calls (Start/Stop/Redrive). Every value here is control-plane metadata.
//!
//! EXPRESS state machines keep no execution history reachable through
//! `ListExecutions` (the service answers `StateMachineTypeNotSupported`), so an
//! execution request for a non-STANDARD machine is rejected locally, before any
//! HTTP call, with an explicit unsupported message.
use crate::config::AwsConfig;
use crate::pagination::{Pagination, StopReason};
use anyhow::{bail, Context, Result};
use aws_smithy_types::date_time::Format;
use aws_smithy_types::DateTime;
use std::time::Duration;

const LIST_TIMEOUT: Duration = Duration::from_secs(30);
// Fail explicitly at the bound rather than presenting an incomplete inventory.
const MAX_PAGES: usize = 1000;
// ListStateMachines / ListExecutions accept maxResults 0..=1000; request the
// documented maximum so a full inventory needs the fewest round-trips.
const PAGE_SIZE: i32 = 1000;

/// Metadata for one state machine, sourced entirely from `ListStateMachines`.
/// No field is or derives from a state-machine definition. Absent SDK values
/// are preserved as empty strings rather than invented.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StateMachineMetadata {
    pub name: String,
    pub arn: String,
    /// `STANDARD` or `EXPRESS`, verbatim from the service's `type` field;
    /// empty only when the SDK omitted it.
    pub machine_type: String,
    /// RFC3339 UTC, or empty when the SDK omitted it.
    pub creation_date: String,
}

/// Metadata for one execution, sourced entirely from `ListExecutions`. No field
/// is or derives from an execution's input or output. Absent timestamps stay
/// empty; `duration_millis` is `Some` only for a completed execution with a
/// well-ordered start/stop pair.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExecutionMetadata {
    pub name: String,
    pub arn: String,
    pub state_machine_arn: String,
    pub status: String,
    /// RFC3339 UTC, or empty when the SDK omitted it.
    pub start_date: String,
    /// RFC3339 UTC, or empty when the SDK omitted it (e.g. a RUNNING execution).
    pub stop_date: String,
    /// Wall-clock duration in milliseconds. `Some` only when the execution has
    /// both a start and a stop date and `stop >= start`; `None` otherwise
    /// (running executions, or any out-of-order/unknown pair).
    pub duration_millis: Option<u64>,
    /// Qualified state-machine ARNs reported by ListExecutions, empty if absent.
    pub state_machine_version_arn: String,
    pub state_machine_alias_arn: String,
    /// Preserve an omitted count as unknown, distinct from a reported zero.
    pub redrive_count: Option<i32>,
    /// RFC3339 UTC, empty if the execution has not been redriven or AWS omitted it.
    pub redrive_date: String,
}

/// A state machine must be STANDARD to list executions. Reject EXPRESS and any
/// unrecognised type locally so the caller gets an explicit message instead of
/// a service-side `StateMachineTypeNotSupported` (EXPRESS) or a misleading
/// empty success.
impl StateMachineMetadata {
    fn ensure_executions_supported(&self) -> Result<()> {
        match self.machine_type.as_str() {
            "STANDARD" => Ok(()),
            "EXPRESS" => bail!(
                "listing executions is not supported for EXPRESS state machine {:?}; \
                 Step Functions retains no listable execution history for EXPRESS workflows",
                self.name
            ),
            other => bail!(
                "listing executions is not supported for state machine {:?} of type {other:?}; \
                 only STANDARD state machines expose a listable execution history",
                self.name
            ),
        }
    }
}

/// Format an optional timestamp as RFC3339 UTC, empty when absent.
fn rfc3339(ts: Option<&DateTime>) -> String {
    ts.and_then(|dt| dt.fmt(Format::DateTime).ok())
        .unwrap_or_default()
}

/// Whole milliseconds between two `DateTime`s when `stop >= start`, else `None`.
/// Guards against a service clock skew that would otherwise yield a negative or
/// absurd duration. Sub-millisecond fractions are truncated toward zero.
fn duration_millis(start: Option<&DateTime>, stop: Option<&DateTime>) -> Option<u64> {
    let (start, stop) = (start?, stop?);
    let nanos = stop.as_nanos().checked_sub(start.as_nanos())?;
    if nanos < 0 {
        return None;
    }
    let millis = nanos / 1_000_000;
    u64::try_from(millis).ok()
}

/// Validate a fully-qualified, unqualified state-machine ARN:
/// `arn:<partition>:states:<region>:<account>:stateMachine:<name>`. A qualified
/// ARN (an extra `:version`/`:alias` segment) is rejected — the list APIs key on
/// the bare state-machine ARN. Returns `(region, account, name)` on success.
pub(crate) fn parse_state_machine_arn(arn: &str) -> Result<(&str, &str, &str)> {
    let parts: Vec<_> = arn.splitn(6, ':').collect();
    if parts.first() != Some(&"arn") || parts.get(2) != Some(&"states") {
        bail!("not a Step Functions state machine ARN: {arn:?}");
    }
    if parts.len() != 6
        || parts[1].is_empty() // partition
        || parts[3].is_empty() // region
        || parts[4].len() != 12
        || !parts[4].bytes().all(|c| c.is_ascii_digit()) // account id
        || arn.chars().any(char::is_control)
    {
        bail!("malformed Step Functions state machine ARN: {arn:?}");
    }
    // Resource segment is exactly `stateMachine:<name>` — no further `:` (which
    // would mark a version/alias-qualified ARN the list APIs do not accept).
    let resource = parts[5];
    let name = resource
        .strip_prefix("stateMachine:")
        .filter(|name| !name.is_empty() && !name.contains(':'))
        .with_context(|| format!("not an unqualified state machine ARN: {arn:?}"))?;
    Ok((parts[3], parts[4], name))
}

/// Validate an execution ARN and confirm it belongs to the given state machine:
/// `arn:<partition>:states:<region>:<account>:execution:<stateMachineName>:<executionName>`.
/// Region, account and the embedded state-machine name must all match the parent,
/// so a response cannot smuggle in an execution from another machine/account/region.
fn validate_execution_arn(
    arn: &str,
    region: &str,
    account: &str,
    state_machine_name: &str,
) -> Result<()> {
    let parts: Vec<_> = arn.splitn(7, ':').collect();
    if parts.first() != Some(&"arn") || parts.get(2) != Some(&"states") {
        bail!("not a Step Functions execution ARN: {arn:?}");
    }
    if parts.len() != 7
        || parts[1].is_empty()
        || parts[3] != region
        || parts[4] != account
        || parts[5] != "execution"
        || arn.chars().any(char::is_control)
    {
        bail!("execution ARN does not match the requested state machine scope: {arn:?}");
    }
    let (machine, execution) = parts[6]
        .split_once(':')
        .filter(|(m, e)| !m.is_empty() && !e.is_empty() && !e.contains(':'))
        .with_context(|| format!("malformed Step Functions execution ARN: {arn:?}"))?;
    if machine != state_machine_name {
        bail!(
            "execution ARN {arn:?} belongs to state machine {machine:?}, not {state_machine_name:?}"
        );
    }
    let _ = execution;
    Ok(())
}

pub(crate) fn validate_scope(config: &AwsConfig, region: &str, account: &str) -> Result<()> {
    if !config.region.is_empty() && config.region != region {
        bail!("State machine ARN does not match the active Region");
    }
    if !config.account_id.is_empty() && config.account_id != account {
        bail!("State machine ARN does not match the active account");
    }
    Ok(())
}

pub struct StepFunctionsClient {
    config: AwsConfig,
}

impl StepFunctionsClient {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
    }

    /// List all state machines' metadata, sorted deterministically by name then
    /// ARN. The whole operation is bounded by a 30s timeout; a repeated
    /// pagination token or the page cap is a hard error, never a partial success.
    pub async fn list_state_machines(&self) -> Result<Vec<StateMachineMetadata>> {
        tokio::time::timeout(LIST_TIMEOUT, self.list_state_machines_inner(MAX_PAGES))
            .await
            .map_err(|_| {
                anyhow::anyhow!("Step Functions ListStateMachines timed out after {LIST_TIMEOUT:?}")
            })?
    }

    async fn list_state_machines_inner(
        &self,
        max_pages: usize,
    ) -> Result<Vec<StateMachineMetadata>> {
        let client = self.config.sfn_client();
        let mut machines = Vec::new();
        let mut token = None;
        let mut pagination = Pagination::new(max_pages);
        for _ in 0..max_pages {
            let page = client
                .list_state_machines()
                .max_results(PAGE_SIZE)
                .set_next_token(token)
                .send()
                .await
                .context("Step Functions ListStateMachines request failed")?;
            for item in page.state_machines() {
                let arn = item.state_machine_arn();
                if arn.is_empty() {
                    bail!("Step Functions ListStateMachines returned a state machine with no ARN");
                }
                let (region, account, arn_name) = parse_state_machine_arn(arn)?;
                validate_scope(&self.config, region, account)?;
                let name = item.name();
                if name.is_empty() {
                    bail!("Step Functions ListStateMachines returned a state machine with no name");
                }
                if name != arn_name {
                    bail!("Step Functions ListStateMachines returned a name inconsistent with its ARN");
                }
                machines.push(StateMachineMetadata {
                    name: name.to_string(),
                    arn: arn.to_string(),
                    machine_type: item.r#type().as_str().to_string(),
                    creation_date: rfc3339(Some(item.creation_date())),
                });
            }
            match pagination.advance(page.next_token) {
                Ok(Some(next)) => token = Some(next),
                Ok(None) => {
                    machines.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.arn.cmp(&b.arn)));
                    return Ok(machines);
                }
                Err(StopReason::RepeatedToken) => {
                    bail!("Step Functions ListStateMachines returned a repeated pagination token; aborting");
                }
                Err(StopReason::PageLimit | StopReason::ResultLimit) => break,
            }
        }
        bail!(
            "Step Functions ListStateMachines exceeded {max_pages} pages; inventory is incomplete"
        )
    }

    /// List one STANDARD state machine's executions, newest first (by start date,
    /// then ARN). EXPRESS and unknown machine types are rejected before any HTTP
    /// request. Bounded by a 30s timeout; a repeated token or the page cap is a
    /// hard error. Scopes every returned execution to the parent's name, account
    /// and region, and (when present) checks the returned `stateMachineArn`.
    pub async fn list_executions(
        &self,
        state_machine: &StateMachineMetadata,
    ) -> Result<Vec<ExecutionMetadata>> {
        state_machine.ensure_executions_supported()?;
        let (region, account, name) = parse_state_machine_arn(&state_machine.arn)?;
        validate_scope(&self.config, region, account)?;
        if name != state_machine.name {
            bail!("State machine name does not match its ARN");
        }
        tokio::time::timeout(
            LIST_TIMEOUT,
            self.list_executions_inner(&state_machine.arn, region, account, name, MAX_PAGES),
        )
        .await
        .map_err(|_| {
            anyhow::anyhow!("Step Functions ListExecutions timed out after {LIST_TIMEOUT:?}")
        })?
    }

    async fn list_executions_inner(
        &self,
        state_machine_arn: &str,
        region: &str,
        account: &str,
        state_machine_name: &str,
        max_pages: usize,
    ) -> Result<Vec<ExecutionMetadata>> {
        let client = self.config.sfn_client();
        let mut executions = Vec::new();
        let mut token = None;
        let mut pagination = Pagination::new(max_pages);
        for _ in 0..max_pages {
            let page = client
                .list_executions()
                .state_machine_arn(state_machine_arn)
                .max_results(PAGE_SIZE)
                .set_next_token(token)
                .send()
                .await
                .context("Step Functions ListExecutions request failed")?;
            for item in page.executions() {
                let arn = item.execution_arn();
                if arn.is_empty() {
                    bail!("Step Functions ListExecutions returned an execution with no ARN");
                }
                validate_execution_arn(arn, region, account, state_machine_name)?;
                if arn.split(':').nth(1) != state_machine_arn.split(':').nth(1) {
                    bail!("Execution ARN partition does not match the requested state machine");
                }
                let name = item.name();
                if name.is_empty() {
                    bail!("Step Functions ListExecutions returned an execution with no name");
                }
                if arn.rsplit(':').next() != Some(name) {
                    bail!("Execution name does not match its ARN");
                }
                // When the service echoes a stateMachineArn it must match the one
                // we requested; a mismatch is a scoping violation, not a row.
                let echoed = item.state_machine_arn();
                if !echoed.is_empty() && echoed != state_machine_arn {
                    bail!(
                        "Step Functions ListExecutions returned execution {arn:?} scoped to \
                         {echoed:?}, not the requested {state_machine_arn:?}"
                    );
                }
                executions.push(ExecutionMetadata {
                    name: name.to_string(),
                    arn: arn.to_string(),
                    state_machine_arn: state_machine_arn.to_string(),
                    status: item.status().as_str().to_string(),
                    start_date: rfc3339(Some(item.start_date())),
                    stop_date: rfc3339(item.stop_date()),
                    duration_millis: duration_millis(Some(item.start_date()), item.stop_date()),
                    state_machine_version_arn: item
                        .state_machine_version_arn()
                        .unwrap_or_default()
                        .into(),
                    state_machine_alias_arn: item
                        .state_machine_alias_arn()
                        .unwrap_or_default()
                        .into(),
                    redrive_count: item.redrive_count(),
                    redrive_date: rfc3339(item.redrive_date()),
                });
            }
            match pagination.advance(page.next_token) {
                Ok(Some(next)) => token = Some(next),
                Ok(None) => {
                    // Newest first: descending start date, then ARN for a stable
                    // total order when start dates tie or are absent.
                    executions.sort_by(|a, b| {
                        chrono::DateTime::parse_from_rfc3339(&b.start_date)
                            .ok()
                            .cmp(&chrono::DateTime::parse_from_rfc3339(&a.start_date).ok())
                            .then_with(|| a.arn.cmp(&b.arn))
                    });
                    return Ok(executions);
                }
                Err(StopReason::RepeatedToken) => {
                    bail!("Step Functions ListExecutions returned a repeated pagination token; aborting");
                }
                Err(StopReason::PageLimit | StopReason::ResultLimit) => break,
            }
        }
        bail!("Step Functions ListExecutions exceeded {max_pages} pages; inventory is incomplete")
    }
}

#[cfg(test)]
mod tests;
