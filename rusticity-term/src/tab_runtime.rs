//! Per-tab ownership of mutable service view state.
//!
//! The app keeps exactly one live copy of each service's view state (the
//! `*_state` fields on [`App`]). Tabs make that a lie: two SNS tabs, or an SNS
//! tab opened while another SNS read is still in flight, must each see their own
//! rows, their own pending read and their own detail drill-down — independent
//! even when they point at the *same* service.
//!
//! [`TabRuntime`] resolves that by treating the live `*_state` fields as a
//! *window* onto whichever tab currently owns the screen. The other tabs' state
//! is parked in [`TabRuntime::snapshots`], keyed by the owning tab's stable
//! [`TabId`]. [`sync_active`] swaps the window: it parks the outgoing owner's
//! live state back into its snapshot and hydrates the incoming owner's state
//! into the live fields. [`poll`] temporarily hydrates every *background* owned
//! tab so its reads make progress, then restores the visible window untouched.
//!
//! What this module deliberately does NOT move:
//! * AWS clients, [`App::config`], profile/region — identity, shared by all tabs.
//! * `tabs`, `current_tab`, `current_service`, pickers, sessions — global chrome.
//! * `cli_output::State` — it already keys its views by its own id, so it is
//!   independent without snapshotting; [`poll`] drives it once in the visible
//!   context instead.
//!
//! Only the managed services (SNS, Step Functions, KMS) participate in the
//! background read/hydrate cycle today; the third worker supplies their
//! `inventory::{poll,request,handle_action}` and `RootState::load_status`.
//! Every *other* service still gets a snapshot so its view state is
//! tab-independent, but its reads continue to run only while it is the visible
//! owner (main-owner context), matching pre-tab behaviour.

use crate::app::{App, Service, ViewMode};
use crate::async_read::{ReadScope, ReadStatus};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Stable, monotonic identity for a tab. Cloned freely; equality is identity.
///
/// Two tabs on the same service have distinct ids, a reordered tab keeps its id,
/// and a closed tab's id is never reissued — so a background read can always be
/// matched to the exact tab that started it, regardless of the tab's current
/// index or title.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TabId(u64);

impl TabId {
    /// Allocate a fresh id. Never returns a value handed out before, even across
    /// tab close/reopen, because the counter only moves forward.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        TabId(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

/// The parked view state of one non-visible tab.
///
/// A snapshot owns the `*_state` value(s) that belong to the tab's service plus
/// the legacy [`ViewMode`] (CloudWatch and a few others still steer rendering
/// off `app.view_mode`). The three IAM services share one `IamState` field on
/// [`App`]; each gets its own variant so three IAM tabs stay independent, and
/// each carries a full `IamState`.
///
/// `CliOutput` has no variant: its state is self-keyed in `cli_output::State`.
enum Snapshot {
    ApiGatewayApis(Box<crate::app::ApigState>, ViewMode),
    CloudWatchLogGroups(Box<crate::app::CloudWatchLogGroupsState>, ViewMode),
    CloudWatchInsights(Box<crate::app::CloudWatchInsightsState>, ViewMode),
    CloudWatchAlarms(Box<crate::app::CloudWatchAlarmsState>, ViewMode),
    CloudTrailEvents(Box<crate::app::CloudTrailState>, ViewMode),
    RdsInstances(Box<crate::app::RdsState>, ViewMode),
    S3Buckets(Box<crate::app::S3State>, ViewMode),
    SnsTopics(Box<crate::app::SnsState>, ViewMode),
    SecretsManagerSecrets(Box<crate::app::SecretsManagerState>, ViewMode),
    AcmCertificates(Box<crate::app::AcmState>, ViewMode),
    StepFunctionsStateMachines(Box<crate::ui::stepfunctions::State>, ViewMode),
    SqsQueues(Box<crate::app::SqsState>, ViewMode),
    VpcVpcs(Box<crate::app::VpcState>, ViewMode),
    Ec2Instances(Box<crate::app::Ec2State>, ViewMode),
    EcrRepositories(Box<crate::app::EcrState>, ViewMode),
    KmsKeys(Box<crate::app::KmsState>, ViewMode),
    EfsFileSystems(Box<crate::app::EfsState>, ViewMode),
    FsxFileSystems(Box<crate::app::FsxState>, ViewMode),
    DynamoDbTables(Box<crate::app::DynamoDbState>, ViewMode),
    LambdaFunctions(Box<crate::app::LambdaState>, ViewMode),
    LambdaApplications(Box<crate::app::LambdaApplicationState>, ViewMode),
    CloudFormationStacks(Box<crate::app::CfnState>, ViewMode),
    /// Aliased: all three IAM services snapshot their own `IamState`.
    IamUsers(Box<crate::app::IamState>, ViewMode),
    IamRoles(Box<crate::app::IamState>, ViewMode),
    IamUserGroups(Box<crate::app::IamState>, ViewMode),
}

impl Snapshot {
    /// Which service this parked state belongs to. Used to assert a snapshot is
    /// hydrated back into a tab of the matching service.
    fn service(&self) -> Service {
        match self {
            Snapshot::ApiGatewayApis(..) => Service::ApiGatewayApis,
            Snapshot::CloudWatchLogGroups(..) => Service::CloudWatchLogGroups,
            Snapshot::CloudWatchInsights(..) => Service::CloudWatchInsights,
            Snapshot::CloudWatchAlarms(..) => Service::CloudWatchAlarms,
            Snapshot::CloudTrailEvents(..) => Service::CloudTrailEvents,
            Snapshot::RdsInstances(..) => Service::RdsInstances,
            Snapshot::S3Buckets(..) => Service::S3Buckets,
            Snapshot::SnsTopics(..) => Service::SnsTopics,
            Snapshot::SecretsManagerSecrets(..) => Service::SecretsManagerSecrets,
            Snapshot::AcmCertificates(..) => Service::AcmCertificates,
            Snapshot::StepFunctionsStateMachines(..) => Service::StepFunctionsStateMachines,
            Snapshot::SqsQueues(..) => Service::SqsQueues,
            Snapshot::VpcVpcs(..) => Service::VpcVpcs,
            Snapshot::Ec2Instances(..) => Service::Ec2Instances,
            Snapshot::EcrRepositories(..) => Service::EcrRepositories,
            Snapshot::KmsKeys(..) => Service::KmsKeys,
            Snapshot::EfsFileSystems(..) => Service::EfsFileSystems,
            Snapshot::FsxFileSystems(..) => Service::FsxFileSystems,
            Snapshot::DynamoDbTables(..) => Service::DynamoDbTables,
            Snapshot::LambdaFunctions(..) => Service::LambdaFunctions,
            Snapshot::LambdaApplications(..) => Service::LambdaApplications,
            Snapshot::CloudFormationStacks(..) => Service::CloudFormationStacks,
            Snapshot::IamUsers(..) => Service::IamUsers,
            Snapshot::IamRoles(..) => Service::IamRoles,
            Snapshot::IamUserGroups(..) => Service::IamUserGroups,
        }
    }
}

/// Produce a snapshot by MOVING the live state for `service` out of `app`,
/// leaving a fresh/default value in its place. The live fields are never left
/// dangling: every arm swaps in the same empty state the constructors install.
fn take_live(app: &mut App, service: Service) -> Option<Snapshot> {
    let view_mode = app.view_mode;
    let snap = match service {
        Service::CliOutput(_) => return None,
        Service::ApiGatewayApis => {
            Snapshot::ApiGatewayApis(Box::new(std::mem::take(&mut app.apig_state)), view_mode)
        }
        Service::CloudWatchLogGroups => Snapshot::CloudWatchLogGroups(
            Box::new(std::mem::replace(
                &mut app.log_groups_state,
                crate::app::CloudWatchLogGroupsState::new(),
            )),
            view_mode,
        ),
        Service::CloudWatchInsights => Snapshot::CloudWatchInsights(
            Box::new(std::mem::replace(
                &mut app.insights_state,
                crate::app::CloudWatchInsightsState::new(),
            )),
            view_mode,
        ),
        Service::CloudWatchAlarms => Snapshot::CloudWatchAlarms(
            Box::new(std::mem::replace(
                &mut app.alarms_state,
                crate::app::CloudWatchAlarmsState::new(),
            )),
            view_mode,
        ),
        Service::CloudTrailEvents => Snapshot::CloudTrailEvents(
            Box::new(std::mem::take(&mut app.cloudtrail_state)),
            view_mode,
        ),
        Service::RdsInstances => {
            Snapshot::RdsInstances(Box::new(std::mem::take(&mut app.rds_state)), view_mode)
        }
        Service::S3Buckets => {
            Snapshot::S3Buckets(Box::new(std::mem::take(&mut app.s3_state)), view_mode)
        }
        Service::SnsTopics => {
            Snapshot::SnsTopics(Box::new(std::mem::take(&mut app.sns_state)), view_mode)
        }
        Service::AcmCertificates => {
            Snapshot::AcmCertificates(Box::new(std::mem::take(&mut app.acm_state)), view_mode)
        }
        Service::SecretsManagerSecrets => Snapshot::SecretsManagerSecrets(
            Box::new(std::mem::take(&mut app.secretsmanager_state)),
            view_mode,
        ),
        Service::StepFunctionsStateMachines => Snapshot::StepFunctionsStateMachines(
            Box::new(std::mem::take(&mut app.stepfunctions_state)),
            view_mode,
        ),
        Service::SqsQueues => {
            Snapshot::SqsQueues(Box::new(std::mem::take(&mut app.sqs_state)), view_mode)
        }
        Service::VpcVpcs => {
            Snapshot::VpcVpcs(Box::new(std::mem::take(&mut app.vpc_state)), view_mode)
        }
        Service::Ec2Instances => {
            Snapshot::Ec2Instances(Box::new(std::mem::take(&mut app.ec2_state)), view_mode)
        }
        Service::EcrRepositories => {
            Snapshot::EcrRepositories(Box::new(std::mem::take(&mut app.ecr_state)), view_mode)
        }
        Service::KmsKeys => {
            Snapshot::KmsKeys(Box::new(std::mem::take(&mut app.kms_state)), view_mode)
        }
        Service::EfsFileSystems => {
            Snapshot::EfsFileSystems(Box::new(std::mem::take(&mut app.efs_state)), view_mode)
        }
        Service::FsxFileSystems => {
            Snapshot::FsxFileSystems(Box::new(std::mem::take(&mut app.fsx_state)), view_mode)
        }
        Service::DynamoDbTables => {
            Snapshot::DynamoDbTables(Box::new(std::mem::take(&mut app.dynamodb_state)), view_mode)
        }
        Service::LambdaFunctions => {
            Snapshot::LambdaFunctions(Box::new(std::mem::take(&mut app.lambda_state)), view_mode)
        }
        Service::LambdaApplications => Snapshot::LambdaApplications(
            Box::new(std::mem::take(&mut app.lambda_application_state)),
            view_mode,
        ),
        Service::CloudFormationStacks => {
            Snapshot::CloudFormationStacks(Box::new(std::mem::take(&mut app.cfn_state)), view_mode)
        }
        Service::IamUsers => {
            Snapshot::IamUsers(Box::new(std::mem::take(&mut app.iam_state)), view_mode)
        }
        Service::IamRoles => {
            Snapshot::IamRoles(Box::new(std::mem::take(&mut app.iam_state)), view_mode)
        }
        Service::IamUserGroups => {
            Snapshot::IamUserGroups(Box::new(std::mem::take(&mut app.iam_state)), view_mode)
        }
    };
    Some(snap)
}

/// Move a parked snapshot back into the live `*_state` fields, restoring the
/// legacy [`ViewMode`]. The inverse of [`take_live`].
fn hydrate(app: &mut App, snap: Snapshot) {
    match snap {
        Snapshot::ApiGatewayApis(s, vm) => {
            app.apig_state = *s;
            app.view_mode = vm;
        }
        Snapshot::CloudWatchLogGroups(s, vm) => {
            app.log_groups_state = *s;
            app.view_mode = vm;
        }
        Snapshot::CloudWatchInsights(s, vm) => {
            app.insights_state = *s;
            app.view_mode = vm;
        }
        Snapshot::CloudWatchAlarms(s, vm) => {
            app.alarms_state = *s;
            app.view_mode = vm;
        }
        Snapshot::CloudTrailEvents(s, vm) => {
            app.cloudtrail_state = *s;
            app.view_mode = vm;
        }
        Snapshot::RdsInstances(s, vm) => {
            app.rds_state = *s;
            app.view_mode = vm;
        }
        Snapshot::S3Buckets(s, vm) => {
            app.s3_state = *s;
            app.view_mode = vm;
        }
        Snapshot::SnsTopics(s, vm) => {
            app.sns_state = *s;
            app.view_mode = vm;
        }
        Snapshot::AcmCertificates(s, vm) => {
            app.acm_state = *s;
            app.view_mode = vm;
        }
        Snapshot::SecretsManagerSecrets(s, vm) => {
            app.secretsmanager_state = *s;
            app.view_mode = vm;
        }
        Snapshot::StepFunctionsStateMachines(s, vm) => {
            app.stepfunctions_state = *s;
            app.view_mode = vm;
        }
        Snapshot::SqsQueues(s, vm) => {
            app.sqs_state = *s;
            app.view_mode = vm;
        }
        Snapshot::VpcVpcs(s, vm) => {
            app.vpc_state = *s;
            app.view_mode = vm;
        }
        Snapshot::Ec2Instances(s, vm) => {
            app.ec2_state = *s;
            app.view_mode = vm;
        }
        Snapshot::EcrRepositories(s, vm) => {
            app.ecr_state = *s;
            app.view_mode = vm;
        }
        Snapshot::KmsKeys(s, vm) => {
            app.kms_state = *s;
            app.view_mode = vm;
        }
        Snapshot::EfsFileSystems(s, vm) => {
            app.efs_state = *s;
            app.view_mode = vm;
        }
        Snapshot::FsxFileSystems(s, vm) => {
            app.fsx_state = *s;
            app.view_mode = vm;
        }
        Snapshot::DynamoDbTables(s, vm) => {
            app.dynamodb_state = *s;
            app.view_mode = vm;
        }
        Snapshot::LambdaFunctions(s, vm) => {
            app.lambda_state = *s;
            app.view_mode = vm;
        }
        Snapshot::LambdaApplications(s, vm) => {
            app.lambda_application_state = *s;
            app.view_mode = vm;
        }
        Snapshot::CloudFormationStacks(s, vm) => {
            app.cfn_state = *s;
            app.view_mode = vm;
        }
        Snapshot::IamUsers(s, vm) | Snapshot::IamRoles(s, vm) | Snapshot::IamUserGroups(s, vm) => {
            app.iam_state = *s;
            app.view_mode = vm;
        }
    }
}

/// Owns the parked state of every non-visible tab and tracks which tab's state
/// is currently in the live `*_state` window.
#[derive(Default)]
pub struct TabRuntime {
    /// Parked view state, keyed by the owning tab's stable id.
    snapshots: BTreeMap<TabId, Snapshot>,
    /// The tab whose state currently occupies the live `*_state` fields, or
    /// `None` before the runtime has ever bound (a freshly constructed app).
    bound: Option<TabId>,
    bound_service: Option<Service>,
    identity: Option<ReadScope>,
}

impl TabRuntime {
    /// True before the runtime has adopted any tab's state. In this state the
    /// live `*_state` fields are whatever a constructor or a test fixture
    /// installed, and must be ADOPTED on first bind rather than overwritten.
    fn unbound(&self) -> bool {
        self.bound.is_none()
    }
}

/// The tab that owns the screen: the current tab, but only when its service
/// still matches `current_service` (they can diverge mid-transition). Returns
/// `None` for a CLI output tab (self-managed) or a detached/empty fixture.
fn active_owner(app: &App) -> Option<(TabId, Service)> {
    let tab = app.tabs.get(app.current_tab)?;
    if tab.service != app.current_service {
        return None;
    }
    Some((tab.id, tab.service))
}

/// Swap the live `*_state` window to the currently-active tab.
///
/// Call this both BEFORE and AFTER [`App::handle_action`], and immediately after
/// any code that changes `current_tab`/`current_service` (tab select, next/prev,
/// close, session replace) — before any per-service reset runs — so a reset
/// never mutates the previously-visible tab's parked state.
///
/// Behaviour:
/// * First call on an unbound runtime with a resolvable owner ADOPTS the live
///   state as that owner's (no snapshot exists yet; the fixture/ctor state is
///   kept, not replaced with an empty one).
/// * When the active owner is unchanged, this is a no-op.
/// * Otherwise it parks the outgoing owner's live state (if that tab is still
///   open) and hydrates the incoming owner's snapshot, creating a fresh empty
///   one the first time a tab is seen (so a brand-new managed tab starts blank
///   and will `inventory::request` on its first poll).
/// * A detached fixture (no tab, or `tab.service != current_service`) leaves the
///   live fields alone and does not fabricate an id — tests stay usable.
pub fn sync_active(app: &mut App) {
    let identity = ReadScope::from_app(app, "").with_owner(None);
    if app
        .tab_runtime
        .identity
        .as_ref()
        .is_some_and(|saved| saved != &identity)
    {
        reset_for_session_replace(app);
        crate::cli_output::invalidate_context(app);
    }
    app.tab_runtime.identity = Some(identity);
    purge_closed(app);
    let Some((owner, service)) = active_owner(app) else {
        // Nothing owns the screen (empty fixture, CLI tab, or mid-transition
        // divergence). Leave the live window and binding untouched.
        return;
    };

    if app.tab_runtime.unbound() {
        // Adopt the pre-existing live state as this owner's, without disturbing
        // prepared fixture fields.
        app.tab_runtime.bound = Some(owner);
        app.tab_runtime.bound_service = Some(service);
        if is_managed(service) && live_status(app, service) == ReadStatus::Idle {
            request_fresh_managed(app, service);
        }
        return;
    }

    let previous = app.tab_runtime.bound;
    if previous == Some(owner) {
        return; // Already the visible window.
    }

    // Park the outgoing owner's live state, but only if that tab is still open —
    // a closed tab's state was already dropped (and its reads aborted) by purge.
    if let Some(prev_id) = previous {
        let prev_still_open = app
            .tabs
            .iter()
            .any(|tab| tab.id == prev_id && !matches!(tab.service, Service::CliOutput(_)));
        if prev_still_open {
            // The service to park under is the one the live fields hold, which is
            // the previous owner's service. Find it from the tab list.
            if let Some(prev_service) = app
                .tabs
                .iter()
                .find(|tab| tab.id == prev_id)
                .map(|tab| tab.service)
            {
                if let Some(snap) = take_live(app, prev_service) {
                    app.tab_runtime.snapshots.insert(prev_id, snap);
                }
            }
        }
    }

    // Hydrate the incoming owner.
    match app.tab_runtime.snapshots.remove(&owner) {
        Some(snap) => {
            // A tab seen before: restore exactly what it had. Switching back to a
            // loaded managed tab must NOT re-request — its rows (and any finished
            // read) come straight from the snapshot.
            debug_assert_eq!(
                snap.service(),
                service,
                "snapshot/service mismatch on hydrate"
            );
            hydrate(app, snap);
        }
        None => {
            // A tab seen for the first time. Install a fresh default so it begins
            // blank: `take_live` leaves that default in the live fields (we drop
            // the snapshot it hands back). This also overwrites any stale state a
            // just-closed previous owner may have left behind when it was not
            // parked.
            if let Some(fresh) = take_live(app, service) {
                drop(fresh);
            }
            // A brand-new managed tab kicks off its first inventory read exactly
            // once. Non-managed services keep their legacy open-time request path.
            request_fresh_managed(app, service);
        }
    }

    app.tab_runtime.bound = Some(owner);
    app.tab_runtime.bound_service = Some(service);
}

/// Queue the first inventory read for a freshly-opened managed tab. Delegates to
/// the service's own `inventory::request` (third worker), which operates on the
/// now-live `*_state` fields. A no-op for non-managed services.
fn request_fresh_managed(app: &mut App, service: Service) {
    match service {
        Service::SnsTopics => crate::sns::inventory::request(app),
        Service::StepFunctionsStateMachines => crate::stepfunctions::inventory::request(app),
        Service::KmsKeys => crate::kms::inventory::request(app),
        Service::AcmCertificates => crate::acm::inventory::request(app),
        Service::SqsQueues => crate::sqs::inventory::request(app),
        Service::LambdaFunctions => crate::lambda::inventory::request(app),
        Service::DynamoDbTables => crate::dynamodb::inventory::request(app),
        _ => {}
    }
}

/// Drop snapshots for tabs that no longer exist, aborting their owned reads
/// (dropping an `AsyncRead` aborts its task). Call after any tab removal.
///
/// Deliberately does NOT clear `bound`: if the closed tab was the bound owner,
/// leaving `bound` pointing at the (now-absent) id makes the next [`sync_active`]
/// treat the move as an ordinary switch — it finds the previous owner is no
/// longer open, so it does NOT park the live (orphaned) state, and instead
/// hydrates the incoming owner from its snapshot. Clearing `bound` here would
/// instead make `sync_active` ADOPT the orphaned live state as the new owner's,
/// which is wrong.
pub fn purge_closed(app: &mut App) {
    app.cli_output.retain_tabs(&app.tabs);
    let live: std::collections::BTreeSet<TabId> = app.tabs.iter().map(|tab| tab.id).collect();
    app.tab_runtime.snapshots.retain(|id, _| live.contains(id));
    if app.tab_runtime.bound.is_some_and(|id| !live.contains(&id)) {
        if let Some(service) = app.tab_runtime.bound_service.take() {
            drop(take_live(app, service));
        }
    }
}

/// Reset the runtime for a whole-tab-list replacement (loading a session). Every
/// new tab carries a brand-new id, so every parked snapshot is stale: drop them
/// all. `bound` is set to a throwaway id that matches no tab, which forces the
/// next [`sync_active`] down the ordinary switch path (fresh-hydrate + managed
/// request) rather than ADOPTING the stale live state as the new owner's.
pub fn reset_for_session_replace(app: &mut App) {
    if let Some(service) = app.tab_runtime.bound_service.take() {
        drop(take_live(app, service));
    }
    app.tab_runtime.snapshots.clear();
    app.tab_runtime.bound = Some(TabId::new());
}

/// Any owned managed tab (visible or background) with a busy read, or any busy
/// CLI output tab. Drives the main loop's "keep polling" decision.
/// True only for a newly-created identity, never a changed vector position.
pub fn opened_new_tab(app: &App, previous: &[crate::app::Tab]) -> bool {
    app.tabs
        .get(app.current_tab)
        .is_some_and(|current| previous.iter().all(|old| old.id != current.id))
}

/// Main must not re-run a legacy loader just because an existing tab regained
/// focus. Its owned snapshot already contains its current view/results.
pub fn restored_existing_tab(
    app: &App,
    previous: &[crate::app::Tab],
    previous_index: usize,
) -> bool {
    let Some(current) = app.tabs.get(app.current_tab) else {
        return false;
    };
    previous
        .get(previous_index)
        .is_some_and(|old| old.id != current.id)
        && previous.iter().any(|old| old.id == current.id)
}

pub fn has_background_work(app: &App) -> bool {
    if app.cli_output.busy() {
        return true;
    }
    // Visible managed tab.
    if let Some((_, service)) = active_owner(app) {
        if is_managed(service) && live_busy(app, service) {
            return true;
        }
    }
    // Parked managed tabs.
    app.tab_runtime.snapshots.values().any(snapshot_busy)
}

/// Whether a service participates in the background read/hydrate cycle.
fn is_managed(service: Service) -> bool {
    matches!(
        service,
        Service::SnsTopics
            | Service::StepFunctionsStateMachines
            | Service::KmsKeys
            | Service::SecretsManagerSecrets
            | Service::AcmCertificates
            | Service::RdsInstances
            | Service::SqsQueues
            | Service::LambdaFunctions
            | Service::DynamoDbTables
    )
}

/// The read status the UI header should show for `tab`. For the visible owner it
/// reads the live state; for a parked managed tab it reads the snapshot. Legacy
/// services with no loading tracking report [`ReadStatus::Idle`] rather than a
/// fabricated `Ready`.
pub(crate) fn status_for(app: &App, tab: TabId) -> ReadStatus {
    if let Some(Service::CliOutput(id)) = app
        .tabs
        .iter()
        .find(|owner| owner.id == tab)
        .map(|owner| owner.service)
    {
        return app.cli_output.status_for(id);
    }
    if let Some((owner, service)) = active_owner(app) {
        if owner == tab {
            return live_status(app, service);
        }
    }
    match app.tab_runtime.snapshots.get(&tab) {
        Some(snap) => snapshot_status(snap),
        None => ReadStatus::Idle,
    }
}

/// Read status of the LIVE state for a managed service (the visible owner).
/// `RootState::load_status()` (third worker) chooses between the root-list and
/// the entered-detail status. Non-managed services have no loading tracking and
/// report [`ReadStatus::Idle`] — never a fabricated `Ready`.
fn live_status(app: &App, service: Service) -> ReadStatus {
    match service {
        Service::SnsTopics => app.sns_state.load_status(),
        Service::StepFunctionsStateMachines => app.stepfunctions_state.load_status(),
        Service::KmsKeys => app.kms_state.load_status(),
        Service::SecretsManagerSecrets => app.secretsmanager_state.load_status(),
        Service::AcmCertificates => app.acm_state.load_status(),
        Service::RdsInstances => app.rds_state.load_status(),
        Service::SqsQueues => app.sqs_state.load_status(),
        Service::LambdaFunctions => app.lambda_state.load_status(),
        Service::DynamoDbTables => app.dynamodb_state.load_status(),
        _ => ReadStatus::Idle,
    }
}

/// Read status of a PARKED managed snapshot. Mirrors [`live_status`] but reads
/// the boxed snapshot state.
fn snapshot_status(snap: &Snapshot) -> ReadStatus {
    match snap {
        Snapshot::SnsTopics(s, _) => s.load_status(),
        Snapshot::StepFunctionsStateMachines(s, _) => s.load_status(),
        Snapshot::KmsKeys(s, _) => s.load_status(),
        Snapshot::SecretsManagerSecrets(s, _) => s.load_status(),
        Snapshot::AcmCertificates(s, _) => s.load_status(),
        Snapshot::RdsInstances(s, _) => s.load_status(),
        Snapshot::SqsQueues(s, _) => s.load_status(),
        Snapshot::LambdaFunctions(s, _) => s.load_status(),
        Snapshot::DynamoDbTables(s, _) => s.load_status(),
        _ => ReadStatus::Idle,
    }
}

fn live_busy(app: &App, service: Service) -> bool {
    matches!(live_status(app, service), ReadStatus::Loading { .. })
}

fn snapshot_busy(snap: &Snapshot) -> bool {
    matches!(snapshot_status(snap), ReadStatus::Loading { .. })
}

/// Intercept a root-level Refresh/Retry for a managed service so the owning view
/// clears its rows BEFORE the next render, by delegating to the service's own
/// `inventory::handle_action` (third worker). Returns true when handled.
///
/// The outer [`App::handle_action`] wrapper may call this before `inner` so the
/// rows are cleared for the frame; the per-service action handlers still run for
/// everything this does not claim.
pub fn handle_action(app: &mut App, action: &crate::keymap::Action) -> bool {
    match app.current_service {
        Service::SnsTopics => crate::sns::inventory::handle_action(app, action),
        Service::StepFunctionsStateMachines => {
            crate::stepfunctions::inventory::handle_action(app, action)
        }
        Service::KmsKeys => crate::kms::inventory::handle_action(app, action),
        Service::AcmCertificates => crate::acm::inventory::handle_action(app, action),
        Service::SqsQueues => crate::sqs::inventory::handle_action(app, action),
        Service::LambdaFunctions => crate::lambda::inventory::handle_action(app, action),
        Service::DynamoDbTables => crate::dynamodb::inventory::handle_action(app, action),
        _ => false,
    }
}

/// Advance every owned tab's background reads, without disturbing the visible
/// window.
///
/// Order and invariants:
/// 1. Snapshot the visible UI context (indices/mode) so it is restored verbatim.
/// 2. Park the visible owner's live state into `snapshots` so the live `*_state`
///    fields are free and EVERY owned managed tab can be processed uniformly —
///    this is what makes polling a background tab safe: nothing overwrites the
///    visible owner's unsaved live data.
/// 3. For each managed owned tab (now all parked): hydrate it, point the cursor
///    at it so its read scope resolves it as owner, run its `inventory::poll`
///    (+ detail poll), and park it back with any progress. No draw happens.
/// 4. Restore the visible owner's state into the live fields and the saved
///    context, so the screen is byte-for-byte what it was.
/// 5. Poll `cli_output` ONCE in the restored visible context — CLI views key by
///    id, so they need no per-tab hydration.
///
/// On an AWS-context mismatch each service's own poll clears its data and drops
/// its jobs (it compares against the read's owner scope); a mere service/tab
/// focus change is NOT a context change and must never cancel a read.
///
/// Returns true if any poll reported a change (the caller should redraw).
pub async fn poll(app: &mut App) -> bool {
    sync_active(app);
    // (1) Preserve the visible context.
    let saved_tab = app.current_tab;
    let saved_service = app.current_service;
    let saved_view_mode = app.view_mode;
    let saved_service_selected = app.service_selected;
    let saved_mode = app.mode;

    let mut changed = false;

    // (2) Park the visible owner so the live fields are free. Remember it so we
    // can hydrate it back verbatim at the end.
    let visible_owner = active_owner(app);
    if let Some((id, service)) = visible_owner {
        if let Some(snap) = take_live(app, service) {
            app.tab_runtime.snapshots.insert(id, snap);
        }
    }

    // (3) Every managed owned tab, now uniformly parked. Collect ids first so we
    // are not iterating the map while mutating it.
    let managed: Vec<(TabId, Service)> = app
        .tab_runtime
        .snapshots
        .iter()
        .filter(|(_, snap)| is_managed(snap.service()))
        .map(|(id, snap)| (*id, snap.service()))
        .collect();

    for (id, service) in managed {
        let Some(snap) = app.tab_runtime.snapshots.remove(&id) else {
            continue;
        };
        hydrate(app, snap);
        point_context_at(app, id, service);

        changed |= poll_managed(app, service).await;

        if let Some(reparked) = take_live(app, service) {
            app.tab_runtime.snapshots.insert(id, reparked);
        }
    }

    // (4) Restore the visible owner's live state and the saved context.
    if let Some((id, service)) = visible_owner {
        if let Some(snap) = app.tab_runtime.snapshots.remove(&id) {
            debug_assert_eq!(snap.service(), service);
            hydrate(app, snap);
        }
    }
    app.current_tab = saved_tab;
    app.current_service = saved_service;
    app.view_mode = saved_view_mode;
    app.service_selected = saved_service_selected;
    app.mode = saved_mode;

    // (5) CLI output, once, in the original visible context.
    changed |= crate::cli_output::poll(app).await;

    // Defensive: CLI poll must not have moved focus.
    app.current_tab = saved_tab;
    app.current_service = saved_service;
    app.view_mode = saved_view_mode;
    app.service_selected = saved_service_selected;
    app.mode = saved_mode;

    changed
}

/// Point the global cursor fields at `id`/`service` so a read's scope resolves
/// this tab as its owner during a background poll. Finds the tab's current index
/// (identity is stable across reorders; the index is not).
fn point_context_at(app: &mut App, id: TabId, service: Service) {
    if let Some(index) = app.tabs.iter().position(|tab| tab.id == id) {
        app.current_tab = index;
    }
    app.current_service = service;
    app.service_selected = true;
}

/// Run a managed service's inventory poll plus its existing detail poll. The
/// third worker supplies `inventory::poll`; the detail poll is the service's
/// pre-existing one.
async fn poll_managed(app: &mut App, service: Service) -> bool {
    match service {
        Service::SnsTopics => {
            let inventory = crate::sns::inventory::poll(app).await;
            crate::sns::subscriptions::poll(app).await || inventory
        }
        Service::StepFunctionsStateMachines => {
            let inventory = crate::stepfunctions::inventory::poll(app).await;
            crate::stepfunctions::executions::poll(app).await || inventory
        }
        Service::KmsKeys => {
            let inventory = crate::kms::inventory::poll(app).await;
            crate::kms::detail::poll(app).await || inventory
        }
        // The inventory keeps its legacy loader; only the entered detail is owned.
        Service::SecretsManagerSecrets => crate::secretsmanager::detail::poll(app).await,
        Service::AcmCertificates => crate::acm::inventory::poll(app).await,
        // The list keeps its legacy loader; only the entered detail is owned.
        Service::RdsInstances => crate::rds::detail::poll(app).await,
        Service::SqsQueues => crate::sqs::inventory::poll(app).await,
        Service::LambdaFunctions => crate::lambda::inventory::poll(app).await,
        Service::DynamoDbTables => crate::dynamodb::inventory::poll(app).await,
        _ => false,
    }
}

#[cfg(test)]
mod tests;
