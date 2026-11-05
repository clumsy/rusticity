//! Local, persistent resource bookmarks. This store touches only the local
//! filesystem: it never reads credentials, never calls AWS, and never persists
//! resource payloads -- only whitelisted identity metadata (profile, connection
//! region, account guard, and the resource's kind/id/name/arn/region).
//!
//! Durability contract:
//! - Every mutation is read-merge-write under a sibling `create_new` lock, so
//!   two cooperative instances cannot lose each other's entries.
//! - The write is atomic (private 0600 sibling temp + rename on Unix), so an
//!   interrupted write leaves the original file intact.
//! - A malformed, oversized, or unknown-version file is never clobbered; the
//!   caller gets an error and the bytes on disk are preserved.
//! - A symlink at the bookmarks path is refused outright (no follow, no write).
use anyhow::{bail, Context, Result};
use rusticity_core::finder::{Resource, ResourceKind};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Serde schema version. Bump only on an incompatible on-disk change; an
/// unknown version is refused rather than silently reinterpreted.
const VERSION: u32 = 1;

/// Hard ceiling on the file we are willing to parse. A larger file is treated
/// as corrupt/hostile and is never read into memory or clobbered.
const MAX_FILE_BYTES: u64 = 1024 * 1024; // 1 MiB
/// Hard ceiling on the number of records we keep or accept.
const MAX_RECORDS: usize = 1000;

/// Reasonable field bounds. These exist to reject corrupt/hostile input, not to
/// mirror AWS's exact limits.
const MAX_FIELD_BYTES: usize = 2048;

/// A single bookmarked resource target. Equality is structural (all fields); use
/// [`Bookmark::same_target`] for identity that ignores the mutable display name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bookmark {
    pub profile: String,
    pub region: String,
    pub account_id: String,
    pub resource: Resource,
}

impl Bookmark {
    /// Construct a validated bookmark. Rejects empty profile/region/account,
    /// a non-12-digit account id, control characters, over-long fields, and an
    /// ARN whose service/account/region are inconsistent with the resource.
    pub fn new(
        profile: String,
        region: String,
        account_id: String,
        resource: Resource,
    ) -> Result<Self> {
        let bookmark = Self {
            profile,
            region,
            account_id,
            resource,
        };
        bookmark.validate()?;
        Ok(bookmark)
    }

    /// Two bookmarks point at the same target when profile, connection region,
    /// account and resource kind match, and the ARN matches when both carry one
    /// (otherwise the id). A rename (name-only change) does not create a new
    /// target, so toggling a renamed bookmark still finds the original.
    pub fn same_target(&self, other: &Self) -> bool {
        self.profile == other.profile
            && self.region == other.region
            && self.account_id == other.account_id
            && self.resource.kind == other.resource.kind
            && {
                let a = &self.resource;
                let b = &other.resource;
                if !a.arn.is_empty() && !b.arn.is_empty() {
                    a.arn == b.arn
                } else {
                    a.id == b.id
                }
            }
    }

    fn validate(&self) -> Result<()> {
        check_field("profile", &self.profile, true)?;
        check_field("connection region", &self.region, true)?;
        if !self
            .region
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            bail!("connection region must be an AWS Region name");
        }
        check_field("account id", &self.account_id, true)?;
        if self.account_id.len() != 12 || !self.account_id.bytes().all(|b| b.is_ascii_digit()) {
            bail!("account id must be exactly 12 digits");
        }
        // Reference and any unknown/non-inventory kind are not bookmarkable.
        encode_kind(self.resource.kind).with_context(|| {
            format!("resource kind {:?} is not bookmarkable", self.resource.kind)
        })?;
        check_field("resource id", &self.resource.id, true)?;
        // A resource name may be empty; it is a display label only.
        check_field("resource name", &self.resource.name, false)?;
        check_field("resource region", &self.resource.region, false)?;
        // ARN is optional (native selections may carry none); when present it
        // must be well-formed and consistent with this target.
        check_field("resource arn", &self.resource.arn, false)?;
        if !self.resource.arn.is_empty() {
            self.validate_arn()?;
        }
        Ok(())
    }

    /// Validate ARN syntax and cross-check service / account / region against the
    /// bookmark's own fields. Empty ARNs are allowed and skipped by the caller.
    fn validate_arn(&self) -> Result<()> {
        let arn = &self.resource.arn;
        // arn:partition:service:region:account:resource (resource may contain ':').
        let mut parts = arn.splitn(6, ':');
        let (p0, partition, service, region, account) = (
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
        );
        let resource_tail = parts.next();
        let (Some("arn"), Some(partition), Some(service), Some(region), Some(account), Some(tail)) =
            (p0, partition, service, region, account, resource_tail)
        else {
            bail!("malformed ARN: {arn}");
        };
        if !matches!(
            partition,
            "aws" | "aws-cn" | "aws-us-gov" | "aws-iso" | "aws-iso-b" | "aws-iso-e" | "aws-iso-f"
        ) {
            bail!("unknown ARN partition `{partition}` in {arn}");
        }
        if service.is_empty() || tail.is_empty() {
            bail!("ARN missing service or resource: {arn}");
        }
        // Service consistency: the ARN's service must match the kind's ARN service
        // (e.g. a Vpc uses `ec2`, not the SDK service name `vpc`).
        let expected = arn_service(self.resource.kind);
        if service != expected {
            bail!(
                "ARN service `{service}` does not match {:?} (expected `{expected}`)",
                self.resource.kind
            );
        }
        // Account consistency: global-service ARNs (iam, s3) omit the account in
        // the ARN region slot but still carry the account id; when the ARN does
        // carry an account it must match the guard.
        if !account.is_empty() && account != self.account_id {
            bail!(
                "ARN account `{account}` does not match bookmark account `{}`",
                self.account_id
            );
        }
        // Region consistency: global resources (iam, s3, public ecr) use an empty
        // ARN region; a regional ARN must match the resource's own region.
        if !region.is_empty() && !self.resource.region.is_empty() && region != self.resource.region
        {
            bail!(
                "ARN region `{region}` does not match resource region `{}`",
                self.resource.region
            );
        }
        Ok(())
    }
}

/// Reject control characters, over-long values, and (when required) emptiness.
fn check_field(label: &str, value: &str, required: bool) -> Result<()> {
    if required && value.trim().is_empty() {
        bail!("{label} must not be empty");
    }
    if value.len() > MAX_FIELD_BYTES {
        bail!("{label} exceeds {MAX_FIELD_BYTES} bytes");
    }
    if value.chars().any(|c| c.is_control()) {
        bail!("{label} contains control characters");
    }
    Ok(())
}

/// The service token as it appears in a resource's ARN. This differs from
/// [`ResourceKind::service`] for kinds whose ARN service is not the SDK service
/// name (Vpc -> ec2, public ECR -> ecr-public).
fn arn_service(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Vpc | ResourceKind::Instance => "ec2",
        ResourceKind::Api => "apigateway",
        ResourceKind::Stack => "cloudformation",
        ResourceKind::Alarm => "cloudwatch",
        ResourceKind::LogGroup => "logs",
        ResourceKind::Table => "dynamodb",
        ResourceKind::Repository => "ecr",
        ResourceKind::PublicRepository => "ecr-public",
        ResourceKind::Efs => "elasticfilesystem",
        ResourceKind::Fsx => "fsx",
        ResourceKind::User | ResourceKind::Role | ResourceKind::Group => "iam",
        ResourceKind::Key => "kms",
        ResourceKind::Function => "lambda",
        ResourceKind::Database => "rds",
        ResourceKind::Bucket => "s3",
        ResourceKind::Queue => "sqs",
        ResourceKind::Topic => "sns",
        ResourceKind::Secret => "secretsmanager",
        ResourceKind::StateMachine => "states",
        // Not bookmarkable; encode_kind() rejects it before this is reached.
        ResourceKind::Reference => "",
    }
}

/// Stable, explicit on-disk string for a bookmarkable kind. `Reference` and any
/// future non-inventory kind return `None` and are rejected. These strings are
/// part of the v1 schema and must never change for an existing kind.
fn encode_kind(kind: ResourceKind) -> Option<&'static str> {
    Some(match kind {
        ResourceKind::Api => "api",
        ResourceKind::Stack => "stack",
        ResourceKind::Alarm => "alarm",
        ResourceKind::LogGroup => "log_group",
        ResourceKind::Instance => "instance",
        ResourceKind::Vpc => "vpc",
        ResourceKind::Table => "table",
        ResourceKind::Repository => "repository",
        ResourceKind::PublicRepository => "public_repository",
        ResourceKind::Efs => "efs",
        ResourceKind::Fsx => "fsx",
        ResourceKind::User => "user",
        ResourceKind::Role => "role",
        ResourceKind::Group => "group",
        ResourceKind::Key => "key",
        ResourceKind::Function => "function",
        ResourceKind::Database => "database",
        ResourceKind::Bucket => "bucket",
        ResourceKind::Queue => "queue",
        ResourceKind::Topic => "topic",
        ResourceKind::Secret => "secret",
        ResourceKind::StateMachine => "state_machine",
        ResourceKind::Reference => return None,
    })
}

/// Inverse of [`encode_kind`]. An unknown string is rejected, never defaulted.
fn decode_kind(raw: &str) -> Result<ResourceKind> {
    let kind = match raw {
        "api" => ResourceKind::Api,
        "stack" => ResourceKind::Stack,
        "alarm" => ResourceKind::Alarm,
        "log_group" => ResourceKind::LogGroup,
        "instance" => ResourceKind::Instance,
        "vpc" => ResourceKind::Vpc,
        "table" => ResourceKind::Table,
        "repository" => ResourceKind::Repository,
        "public_repository" => ResourceKind::PublicRepository,
        "efs" => ResourceKind::Efs,
        "fsx" => ResourceKind::Fsx,
        "user" => ResourceKind::User,
        "role" => ResourceKind::Role,
        "group" => ResourceKind::Group,
        "key" => ResourceKind::Key,
        "function" => ResourceKind::Function,
        "database" => ResourceKind::Database,
        "bucket" => ResourceKind::Bucket,
        "queue" => ResourceKind::Queue,
        "topic" => ResourceKind::Topic,
        "secret" => ResourceKind::Secret,
        "state_machine" => ResourceKind::StateMachine,
        other => bail!("unknown resource kind `{other}`"),
    };
    Ok(kind)
}

// --- On-disk schema (whitelist only; no credentials, no payloads) -----------

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    version: u32,
    bookmarks: Vec<Record>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    profile: String,
    region: String,
    account_id: String,
    kind: String,
    resource_id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    arn: String,
    #[serde(default)]
    resource_region: String,
}

impl Record {
    fn from_bookmark(bookmark: &Bookmark) -> Result<Self> {
        let kind = encode_kind(bookmark.resource.kind)
            .with_context(|| format!("kind {:?} is not bookmarkable", bookmark.resource.kind))?
            .to_string();
        Ok(Self {
            profile: bookmark.profile.clone(),
            region: bookmark.region.clone(),
            account_id: bookmark.account_id.clone(),
            kind,
            resource_id: bookmark.resource.id.clone(),
            name: bookmark.resource.name.clone(),
            arn: bookmark.resource.arn.clone(),
            resource_region: bookmark.resource.region.clone(),
        })
    }

    fn into_bookmark(self) -> Result<Bookmark> {
        let kind = decode_kind(&self.kind)?;
        let resource = Resource {
            kind,
            id: self.resource_id,
            name: self.name,
            arn: self.arn,
            region: self.resource_region,
        };
        // Re-validate on the way in: a hand-edited or corrupt file must not
        // yield a bookmark that `new()` would have rejected.
        Bookmark::new(self.profile, self.region, self.account_id, resource)
    }
}

// --- Public module API ------------------------------------------------------

/// The default bookmarks file: `~/.rusticity/bookmarks.json`.
pub fn default_path() -> Result<PathBuf> {
    let home = std::env::home_dir().context("Could not find home directory")?;
    Ok(home.join(".rusticity").join("bookmarks.json"))
}

/// Load all bookmarks. A missing file yields an empty list. A symlink, an
/// oversized file, a malformed/unknown-version document, or any invalid record
/// is an error -- the on-disk bytes are never modified by a read.
pub fn load(path: &Path) -> Result<Vec<Bookmark>> {
    match read_document(path)? {
        Some(doc) => decode_document(doc),
        None => Ok(Vec::new()),
    }
}

/// Toggle a bookmark: add it if no same-target entry exists (returns `true`), or
/// remove the existing same-target entry (returns `false`). A re-add refreshes
/// the stored record (e.g. an updated display name) without duplicating.
pub fn toggle(path: &Path, bookmark: &Bookmark) -> Result<bool> {
    bookmark.validate()?;
    with_locked_write(path, |mut entries| {
        let before = entries.len();
        entries.retain(|existing| !existing.same_target(bookmark));
        let removed = entries.len() != before;
        if removed {
            Ok((entries, false))
        } else {
            if !crate::finder::navigation::has_detail(bookmark.resource.kind) {
                bail!("This resource has no dedicated detail view and cannot be bookmarked");
            }
            entries.push(bookmark.clone());
            if entries.len() > MAX_RECORDS {
                bail!("bookmark limit of {MAX_RECORDS} reached");
            }
            Ok((entries, true))
        }
    })
}

/// Remove the bookmark with this exact identity ([`Bookmark::same_target`]).
/// Removing an absent bookmark is a no-op success.
pub fn remove(path: &Path, bookmark: &Bookmark) -> Result<()> {
    with_locked_write(path, |mut entries| {
        entries.retain(|existing| !existing.same_target(bookmark));
        Ok((entries, ()))
    })
    .map(|_| ())
}

// --- File / lock plumbing ---------------------------------------------------

fn lock_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".lock");
    path.with_file_name(name)
}

/// Read and parse the document, enforcing the symlink and size guards. Returns
/// `None` when the file is absent; never mutates the file.
fn read_document(path: &Path) -> Result<Option<Document>> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err).with_context(|| format!("stat {}", path.display())),
    };
    if meta.file_type().is_symlink() {
        bail!(
            "refusing to use bookmarks path (it is a symlink): {}",
            path.display()
        );
    }
    if !meta.is_file() {
        bail!("bookmarks path is not a regular file: {}", path.display());
    }
    if meta.len() > MAX_FILE_BYTES {
        bail!(
            "bookmarks file is too large ({} bytes > {MAX_FILE_BYTES}); refusing to parse {}",
            meta.len(),
            path.display()
        );
    }
    use std::io::Read;
    let file = std::fs::File::open(path).with_context(|| format!("read {}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        bail!("bookmarks file grew beyond the size limit");
    }
    let doc: Document =
        serde_json::from_slice(&bytes).with_context(|| format!("parse {}", path.display()))?;
    if doc.version != VERSION {
        bail!(
            "unsupported bookmarks version {} (expected {VERSION}); refusing to overwrite {}",
            doc.version,
            path.display()
        );
    }
    if doc.bookmarks.len() > MAX_RECORDS {
        bail!(
            "bookmarks file holds {} records (> {MAX_RECORDS}); refusing to parse {}",
            doc.bookmarks.len(),
            path.display()
        );
    }
    Ok(Some(doc))
}

fn decode_document(doc: Document) -> Result<Vec<Bookmark>> {
    doc.bookmarks
        .into_iter()
        .map(Record::into_bookmark)
        .collect()
}

/// Acquire the sibling lock, run a read-merge-write transaction, and release the
/// lock. The closure receives the current entries and returns the next entries
/// plus a value to pass back to the caller. The write is atomic; on any failure
/// the original file is left intact. Only this call's own lock and temp files
/// are ever removed.
fn with_locked_write<T>(
    path: &Path,
    transform: impl FnOnce(Vec<Bookmark>) -> Result<(Vec<Bookmark>, T)>,
) -> Result<T> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
    }
    let lock = lock_path(path);
    let _guard = LockGuard::acquire(&lock)?;

    let current = match read_document(path)? {
        Some(doc) => decode_document(doc)?,
        None => Vec::new(),
    };
    let (next, value) = transform(current)?;

    // Deterministic de-dup: last writer for a given target wins, order preserved
    // by first appearance. This keeps a hand-merged file from ever persisting two
    // entries for one target.
    let mut deduped: Vec<Bookmark> = Vec::with_capacity(next.len());
    for bookmark in next {
        bookmark.validate()?;
        if let Some(slot) = deduped
            .iter_mut()
            .find(|existing| existing.same_target(&bookmark))
        {
            *slot = bookmark;
        } else {
            deduped.push(bookmark);
        }
    }
    if deduped.len() > MAX_RECORDS {
        bail!("bookmark limit of {MAX_RECORDS} exceeded");
    }

    let records: Vec<Record> = deduped
        .iter()
        .map(Record::from_bookmark)
        .collect::<Result<_>>()?;
    let doc = Document {
        version: VERSION,
        bookmarks: records,
    };
    let mut bytes = serde_json::to_vec_pretty(&doc).context("serialize bookmarks")?;
    bytes.push(b'\n');
    if bytes.len() as u64 > MAX_FILE_BYTES {
        bail!("serialized bookmarks exceed {MAX_FILE_BYTES} bytes");
    }
    atomic_write(path, &bytes)?;
    Ok(value)
}

/// Write `bytes` to `path` atomically via a private sibling temp file + rename.
/// On Unix the temp file is created 0600. A failure leaves the original intact
/// and removes only our own temp file.
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut tmp = path.file_name().unwrap_or_default().to_os_string();
    tmp.push(format!(".tmp.{}", std::process::id()));
    let tmp_path = path.with_file_name(tmp);

    use std::io::Write;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    // A failed create never removes an existing file owned by another writer.
    let mut file = options
        .open(&tmp_path)
        .with_context(|| format!("create temp {}", tmp_path.display()))?;
    let result = (|| -> Result<()> {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&tmp_path, path)
            .with_context(|| format!("rename into {}", path.display()))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp_path);
    }
    result
}

/// A cooperative advisory lock held by the existence of a sibling file created
/// with `create_new`. If the lock already exists we refuse rather than block, so
/// a caller never deletes another instance's lock. The guard removes only the
/// lock file it created.
struct LockGuard {
    path: PathBuf,
}

impl LockGuard {
    fn acquire(lock: &Path) -> Result<Self> {
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(lock)
        {
            Ok(_) => Ok(Self {
                path: lock.to_path_buf(),
            }),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => bail!(
                "bookmarks are locked by another instance ({}); try again",
                lock.display()
            ),
            Err(err) => Err(err).with_context(|| format!("create lock {}", lock.display())),
        }
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        // Remove only our own lock; never another instance's.
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// A unique directory under an EXPLICIT `/tmp`. The ambient `TMPDIR` may
    /// point at a scratch dir that has been deleted, so we never rely on it.
    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let n = COUNTER.fetch_add(1, Ordering::SeqCst);
            let path = Path::new("/tmp").join(format!(
                "rusticity-bookmarks-test-{}-{}-{n}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self { path }
        }
        fn file(&self) -> PathBuf {
            self.path.join("bookmarks.json")
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn resource(kind: ResourceKind, id: &str, name: &str, arn: &str, region: &str) -> Resource {
        Resource {
            kind,
            id: id.into(),
            name: name.into(),
            arn: arn.into(),
            region: region.into(),
        }
    }

    fn sample(kind: ResourceKind) -> Bookmark {
        // A representative, ARN-consistent resource for every bookmarkable kind.
        let region = "us-east-2";
        let account = "123456789012";
        let (id, name, arn, res_region) = match kind {
            ResourceKind::Api => (
                "abc123",
                "orders-api",
                "arn:aws:apigateway:us-east-2::/restapis/abc123",
                // apigateway restapi ARNs carry no account; region present.
                "us-east-2",
            ),
            ResourceKind::Stack => (
                "orders",
                "orders",
                "arn:aws:cloudformation:us-east-2:123456789012:stack/orders/uuid",
                "us-east-2",
            ),
            ResourceKind::Alarm => (
                "cpu-high",
                "cpu-high",
                "arn:aws:cloudwatch:us-east-2:123456789012:alarm:cpu-high",
                "us-east-2",
            ),
            ResourceKind::LogGroup => (
                "/aws/lambda/fn",
                "/aws/lambda/fn",
                "arn:aws:logs:us-east-2:123456789012:log-group:/aws/lambda/fn",
                "us-east-2",
            ),
            ResourceKind::Instance => (
                "i-0abc",
                "web",
                "arn:aws:ec2:us-east-2:123456789012:instance/i-0abc",
                "us-east-2",
            ),
            ResourceKind::Vpc => (
                "vpc-0abc",
                "main",
                "arn:aws:ec2:us-east-2:123456789012:vpc/vpc-0abc",
                "us-east-2",
            ),
            ResourceKind::Table => (
                "orders",
                "orders",
                "arn:aws:dynamodb:us-east-2:123456789012:table/orders",
                "us-east-2",
            ),
            ResourceKind::Repository => (
                "team/app",
                "team/app",
                "arn:aws:ecr:us-east-2:123456789012:repository/team/app",
                "us-east-2",
            ),
            ResourceKind::PublicRepository => (
                "team/app",
                "team/app",
                // Public ECR is global (empty region) but carries an account.
                "arn:aws:ecr-public::123456789012:repository/team/app",
                "",
            ),
            ResourceKind::Efs => (
                "fs-0abc",
                "shared",
                "arn:aws:elasticfilesystem:us-east-2:123456789012:file-system/fs-0abc",
                "us-east-2",
            ),
            ResourceKind::Fsx => (
                "fs-0abc",
                "lustre",
                "arn:aws:fsx:us-east-2:123456789012:file-system/fs-0abc",
                "us-east-2",
            ),
            ResourceKind::User => (
                "alice",
                "alice",
                // IAM is global: empty region slot, account present.
                "arn:aws:iam::123456789012:user/alice",
                "",
            ),
            ResourceKind::Role => (
                "deployer",
                "deployer",
                "arn:aws:iam::123456789012:role/deployer",
                "",
            ),
            ResourceKind::Group => (
                "admins",
                "admins",
                "arn:aws:iam::123456789012:group/admins",
                "",
            ),
            ResourceKind::Key => (
                "key-0abc",
                "key-0abc",
                "arn:aws:kms:us-east-2:123456789012:key/key-0abc",
                "us-east-2",
            ),
            ResourceKind::Function => (
                "handler",
                "handler",
                "arn:aws:lambda:us-east-2:123456789012:function:handler",
                "us-east-2",
            ),
            ResourceKind::Database => (
                "orders",
                "orders",
                "arn:aws:rds:us-east-2:123456789012:db:orders",
                "us-east-2",
            ),
            ResourceKind::Bucket => (
                "my-bucket",
                "my-bucket",
                // S3 bucket ARNs are global: no region, no account.
                "arn:aws:s3:::my-bucket",
                "",
            ),
            ResourceKind::Queue => (
                "jobs",
                "jobs",
                "arn:aws:sqs:us-east-2:123456789012:jobs",
                "us-east-2",
            ),
            ResourceKind::Topic => (
                "alerts",
                "alerts",
                "arn:aws:sns:us-east-2:123456789012:alerts",
                "us-east-2",
            ),
            ResourceKind::Secret => (
                "arn:aws:secretsmanager:us-east-2:123456789012:secret:prod/db-AbCdEf",
                "prod/db-AbCdEf",
                "arn:aws:secretsmanager:us-east-2:123456789012:secret:prod/db-AbCdEf",
                "us-east-2",
            ),
            ResourceKind::StateMachine => (
                "arn:aws:states:us-east-2:123456789012:stateMachine:orders",
                "orders",
                "arn:aws:states:us-east-2:123456789012:stateMachine:orders",
                "us-east-2",
            ),
            ResourceKind::Reference => unreachable!("Reference is not bookmarkable"),
        };
        Bookmark::new(
            "default".into(),
            region.into(),
            account.into(),
            resource(kind, id, name, arn, res_region),
        )
        .unwrap_or_else(|e| panic!("sample {kind:?} should validate: {e}"))
    }

    #[test]
    fn default_path_is_under_rusticity_home() {
        // Only check the shape; HOME may vary in CI.
        if std::env::var_os("HOME").is_some() {
            let path = default_path().unwrap();
            assert!(path.ends_with(".rusticity/bookmarks.json"), "{path:?}");
        }
    }

    #[test]
    fn roundtrip_every_bookmarkable_kind() {
        let dir = TempDir::new();
        let file = dir.file();
        let all: Vec<Bookmark> = ResourceKind::ALL
            .iter()
            .copied()
            .filter(|kind| crate::finder::navigation::has_detail(*kind))
            .map(sample)
            .collect();
        assert_eq!(all.len(), 20);
        for bookmark in &all {
            assert!(toggle(&file, bookmark).unwrap(), "first toggle should add");
        }
        let loaded = load(&file).unwrap();
        assert_eq!(loaded.len(), 20);
        // Order-independent set equality.
        for bookmark in &all {
            assert!(
                loaded.iter().any(|b| b == bookmark),
                "missing {:?}",
                bookmark.resource.kind
            );
        }
    }

    #[test]
    fn reference_kind_is_not_bookmarkable() {
        assert!(encode_kind(ResourceKind::Reference).is_none());
        let bad = Bookmark::new(
            "default".into(),
            "us-east-2".into(),
            "123456789012".into(),
            resource(ResourceKind::Reference, "x", "x", "", "us-east-2"),
        );
        assert!(bad.is_err());
    }

    #[test]
    fn same_target_uses_scope_and_arn_or_id_not_name() {
        let a = sample(ResourceKind::Table);
        let mut renamed = a.clone();
        renamed.resource.name = "Renamed".into();
        assert!(a.same_target(&renamed), "rename must not change target");

        // Different account => different target.
        let mut other_account = a.clone();
        other_account.account_id = "210987654321".into();
        assert!(!a.same_target(&other_account));

        // Different connection region => different target.
        let mut other_region = a.clone();
        other_region.region = "eu-west-1".into();
        assert!(!a.same_target(&other_region));

        // Different profile => different target.
        let mut other_profile = a.clone();
        other_profile.profile = "prod".into();
        assert!(!a.same_target(&other_profile));
    }

    #[test]
    fn same_target_falls_back_to_id_when_arn_absent() {
        let region = "us-east-2";
        let make = |id: &str, name: &str| {
            Bookmark::new(
                "default".into(),
                region.into(),
                "123456789012".into(),
                resource(ResourceKind::Bucket, id, name, "", ""),
            )
            .unwrap()
        };
        let a = make("my-bucket", "my-bucket");
        let a_renamed = make("my-bucket", "display");
        let b = make("other-bucket", "other-bucket");
        assert!(a.same_target(&a_renamed));
        assert!(!a.same_target(&b));
    }

    #[test]
    fn toggle_twice_removes_and_rename_does_not_duplicate() {
        let dir = TempDir::new();
        let file = dir.file();
        let bookmark = sample(ResourceKind::Function);
        assert!(toggle(&file, &bookmark).unwrap());
        assert_eq!(load(&file).unwrap().len(), 1);

        // Re-add with a different display name: still the same target, so toggle
        // removes it rather than duplicating.
        let mut renamed = bookmark.clone();
        renamed.resource.name = "Renamed Handler".into();
        assert!(!toggle(&file, &renamed).unwrap(), "same target toggles off");
        assert!(load(&file).unwrap().is_empty());

        // Toggling on again stores the (renamed) record exactly once.
        assert!(toggle(&file, &renamed).unwrap());
        let loaded = load(&file).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].resource.name, "Renamed Handler");
    }

    #[test]
    fn remove_absent_is_ok_and_remove_targets_exact_identity() {
        let dir = TempDir::new();
        let file = dir.file();
        let a = sample(ResourceKind::Queue);
        let b = sample(ResourceKind::Topic);
        // Removing from a missing file is a no-op success.
        remove(&file, &a).unwrap();
        toggle(&file, &a).unwrap();
        toggle(&file, &b).unwrap();
        assert_eq!(load(&file).unwrap().len(), 2);
        remove(&file, &a).unwrap();
        let loaded = load(&file).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].resource.kind, ResourceKind::Topic);
        // Removing the already-absent one again is still fine.
        remove(&file, &a).unwrap();
        assert_eq!(load(&file).unwrap().len(), 1);
    }

    #[test]
    fn corrupt_and_future_version_files_are_not_clobbered() {
        let dir = TempDir::new();
        let file = dir.file();

        // Corrupt JSON.
        std::fs::write(&file, b"{ not json").unwrap();
        assert!(load(&file).is_err());
        assert!(toggle(&file, &sample(ResourceKind::Key)).is_err());
        assert_eq!(std::fs::read(&file).unwrap(), b"{ not json");

        // Unknown future version.
        let future = br#"{"version":999,"bookmarks":[]}"#;
        std::fs::write(&file, future).unwrap();
        assert!(load(&file).is_err());
        assert!(remove(&file, &sample(ResourceKind::Key)).is_err());
        assert_eq!(std::fs::read(&file).unwrap(), future);

        // Unknown kind string in a well-formed v1 doc is rejected on load.
        let bad_kind =
            br#"{"version":1,"bookmarks":[{"profile":"d","region":"us-east-2","account_id":"123456789012","kind":"nope","resource_id":"x"}]}"#;
        std::fs::write(&file, bad_kind).unwrap();
        assert!(load(&file).is_err());
        assert_eq!(std::fs::read(&file).unwrap(), bad_kind);
    }

    #[test]
    fn oversized_file_is_refused_without_reading_into_a_bookmark() {
        let dir = TempDir::new();
        let file = dir.file();
        let big = vec![b' '; (MAX_FILE_BYTES as usize) + 1];
        std::fs::write(&file, &big).unwrap();
        assert!(load(&file).is_err());
        assert!(toggle(&file, &sample(ResourceKind::Bucket)).is_err());
        // Untouched.
        assert_eq!(std::fs::metadata(&file).unwrap().len(), big.len() as u64);
    }

    #[test]
    fn field_bounds_are_enforced() {
        let region = "us-east-2";
        let ok = resource(ResourceKind::Table, "orders", "orders", "", region);
        // Empty profile rejected.
        assert!(
            Bookmark::new("".into(), region.into(), "123456789012".into(), ok.clone()).is_err()
        );
        // Empty region rejected.
        assert!(Bookmark::new("d".into(), "".into(), "123456789012".into(), ok.clone()).is_err());
        // Bad account id (not 12 digits) rejected.
        assert!(Bookmark::new("d".into(), region.into(), "12345".into(), ok.clone()).is_err());
        assert!(
            Bookmark::new("d".into(), region.into(), "12345678901x".into(), ok.clone()).is_err()
        );
        // Control character rejected.
        let ctrl = resource(ResourceKind::Table, "ord\ners", "n", "", region);
        assert!(Bookmark::new("d".into(), region.into(), "123456789012".into(), ctrl).is_err());
        // Empty resource id rejected.
        let empty_id = resource(ResourceKind::Table, "", "n", "", region);
        assert!(Bookmark::new("d".into(), region.into(), "123456789012".into(), empty_id).is_err());
        // Empty name is allowed; empty ARN is allowed.
        let empty_name = resource(ResourceKind::Table, "orders", "", "", region);
        assert!(
            Bookmark::new("d".into(), region.into(), "123456789012".into(), empty_name).is_ok()
        );
    }

    #[test]
    fn inconsistent_arn_is_rejected_and_empty_arn_allowed() {
        let region = "us-east-2";
        // Wrong service for a Vpc (must be ec2).
        let wrong_service = resource(
            ResourceKind::Vpc,
            "vpc-0abc",
            "main",
            "arn:aws:vpc:us-east-2:123456789012:vpc/vpc-0abc",
            region,
        );
        assert!(Bookmark::new(
            "d".into(),
            region.into(),
            "123456789012".into(),
            wrong_service
        )
        .is_err());
        // Wrong account in the ARN.
        let wrong_account = resource(
            ResourceKind::Table,
            "orders",
            "orders",
            "arn:aws:dynamodb:us-east-2:999999999999:table/orders",
            region,
        );
        assert!(Bookmark::new(
            "d".into(),
            region.into(),
            "123456789012".into(),
            wrong_account
        )
        .is_err());
        // Wrong region in a regional ARN.
        let wrong_region = resource(
            ResourceKind::Table,
            "orders",
            "orders",
            "arn:aws:dynamodb:eu-west-1:123456789012:table/orders",
            region,
        );
        assert!(Bookmark::new(
            "d".into(),
            region.into(),
            "123456789012".into(),
            wrong_region
        )
        .is_err());
        // Correct Vpc ARN (ec2) accepted.
        assert!(matches!(
            sample(ResourceKind::Vpc).resource.kind,
            ResourceKind::Vpc
        ));
        // Empty ARN for a native selection is accepted.
        let native = resource(ResourceKind::Table, "orders", "orders", "", region);
        assert!(Bookmark::new("d".into(), region.into(), "123456789012".into(), native).is_ok());
    }

    #[test]
    fn no_payload_fields_are_persisted() {
        let dir = TempDir::new();
        let file = dir.file();
        toggle(&file, &sample(ResourceKind::Key)).unwrap();
        let raw = std::fs::read_to_string(&file).unwrap();
        // Only whitelisted keys appear; nothing resembling a secret value/payload.
        let doc: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let record = &doc["bookmarks"][0];
        let allowed: std::collections::BTreeSet<&str> = [
            "profile",
            "region",
            "account_id",
            "kind",
            "resource_id",
            "name",
            "arn",
            "resource_region",
        ]
        .into_iter()
        .collect();
        for key in record.as_object().unwrap().keys() {
            assert!(
                allowed.contains(key.as_str()),
                "unexpected persisted key {key}"
            );
        }
        // Sanity: no obvious payload words leak in.
        for word in ["value", "payload", "secretstring", "credential", "password"] {
            assert!(!raw.to_lowercase().contains(word), "leaked `{word}`");
        }
    }

    #[test]
    fn symlink_bookmarks_path_is_refused() {
        let dir = TempDir::new();
        let real = dir.path.join("real.json");
        std::fs::write(&real, br#"{"version":1,"bookmarks":[]}"#).unwrap();
        let link = dir.path.join("link.json");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&real, &link).unwrap();
        #[cfg(unix)]
        {
            assert!(load(&link).is_err());
            assert!(toggle(&link, &sample(ResourceKind::Table)).is_err());
            // The symlink target is untouched.
            assert_eq!(
                std::fs::read(&real).unwrap(),
                br#"{"version":1,"bookmarks":[]}"#
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn persisted_file_is_private_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new();
        let file = dir.file();
        toggle(&file, &sample(ResourceKind::Role)).unwrap();
        let mode = std::fs::metadata(&file).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "bookmarks file must be private");
    }

    #[test]
    fn lock_protects_against_a_held_lock() {
        let dir = TempDir::new();
        let file = dir.file();
        toggle(&file, &sample(ResourceKind::Table)).unwrap();
        // Simulate another instance holding the lock.
        let lock = lock_path(&file);
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock)
            .unwrap();
        // A mutation must refuse rather than race or delete the other lock.
        let err = toggle(&file, &sample(ResourceKind::Topic)).unwrap_err();
        assert!(err.to_string().contains("locked"), "{err}");
        // The foreign lock is still present (we did not delete it).
        assert!(lock.exists());
        // Load is lock-free and still works.
        assert_eq!(load(&file).unwrap().len(), 1);
        // Clean up the foreign lock; the next mutation then succeeds.
        std::fs::remove_file(&lock).unwrap();
        assert!(toggle(&file, &sample(ResourceKind::Topic)).unwrap());
        assert_eq!(load(&file).unwrap().len(), 2);
    }

    #[test]
    fn concurrent_cooperative_writers_do_not_lose_entries() {
        let dir = TempDir::new();
        let file = std::sync::Arc::new(dir.file());
        let threads = 8;
        let per_thread = 10;
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let file = file.clone();
                std::thread::spawn(move || {
                    for i in 0..per_thread {
                        let bookmark = Bookmark::new(
                            "default".into(),
                            "us-east-2".into(),
                            "123456789012".into(),
                            resource(
                                ResourceKind::Table,
                                &format!("table-{t}-{i}"),
                                &format!("table-{t}-{i}"),
                                &format!(
                                    "arn:aws:dynamodb:us-east-2:123456789012:table/table-{t}-{i}"
                                ),
                                "us-east-2",
                            ),
                        )
                        .unwrap();
                        // Retry on lock contention; cooperative instances must
                        // converge without losing anyone's entries.
                        loop {
                            match toggle(&file, &bookmark) {
                                Ok(_) => break,
                                Err(e) if e.to_string().contains("locked") => {
                                    std::thread::sleep(std::time::Duration::from_millis(2));
                                }
                                Err(e) => panic!("unexpected error: {e}"),
                            }
                        }
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
        let loaded = load(&file).unwrap();
        assert_eq!(loaded.len(), threads * per_thread);
    }

    #[test]
    fn preexisting_temp_is_not_removed_when_atomic_create_fails() {
        let dir = TempDir::new();
        let file = dir.file();
        toggle(&file, &sample(ResourceKind::Table)).unwrap();
        let before = std::fs::read(&file).unwrap();
        let tmp = file.with_file_name(format!("bookmarks.json.tmp.{}", std::process::id()));
        std::fs::write(&tmp, b"owned by someone else").unwrap();
        assert!(toggle(&file, &sample(ResourceKind::Topic)).is_err());
        assert_eq!(std::fs::read(&tmp).unwrap(), b"owned by someone else");
        assert_eq!(std::fs::read(&file).unwrap(), before);
        assert!(!lock_path(&file).exists());
    }

    #[test]
    fn unknown_fields_are_not_discarded_during_a_mutation() {
        let dir = TempDir::new();
        let file = dir.file();
        let bytes = br#"{"version":1,"bookmarks":[],"future_payload":"keep me"}"#;
        std::fs::write(&file, bytes).unwrap();
        assert!(load(&file).is_err());
        assert!(toggle(&file, &sample(ResourceKind::Table)).is_err());
        assert_eq!(std::fs::read(&file).unwrap(), bytes);
    }
}
