//! Multi-Region finder search. `region:eu-west-1,us-east-2` or `region:all`
//! runs the ordinary single-Region search in each Region, in parallel, and
//! merges the results. A failing Region becomes a Region-prefixed warning;
//! results from the other Regions are still shown. Account-wide (global)
//! inventories are read once, in the first Region, never repeated per Region.
use super::{search, SearchCache, SearchQuery, SearchUpdate, SharedSearchCache};
use crate::config::AwsConfig;
use anyhow::{bail, Result};
use futures::{stream, Future, StreamExt};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{mpsc, Mutex};

/// At most this many Regions are searched at once; each runs its own bounded
/// set of parallel inventory reads.
const REGION_CONCURRENCY: usize = 4;
/// `region:all` is resolved with EC2 DescribeRegions (enabled Regions only).
const DESCRIBE_REGIONS_TIMEOUT: Duration = Duration::from_secs(10);
/// A bound on an explicit list, so a typo cannot fan out without limit.
const MAX_REGIONS: usize = 40;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RegionScope {
    /// The active Region only (no `region:` qualifier).
    #[default]
    Current,
    /// An explicit list, in the order typed, without duplicates.
    List(Vec<String>),
    /// Every Region enabled for the account.
    All,
}

impl RegionScope {
    pub(super) fn parse(list: &str) -> Result<Self> {
        let list = list.to_ascii_lowercase();
        if list == "all" || list == "*" {
            return Ok(Self::All);
        }
        let mut regions: Vec<String> = Vec::new();
        for code in list.split(',').filter(|code| !code.is_empty()) {
            if !is_region_code(code) {
                bail!("Unknown Region {code:?}; try region:us-east-1,eu-west-1 or region:all");
            }
            if !regions.iter().any(|r| r == code) {
                regions.push(code.to_string());
            }
        }
        if regions.is_empty() {
            bail!("Name at least one Region, for example region:us-east-1 or region:all");
        }
        if regions.len() > MAX_REGIONS {
            bail!(
                "Too many Regions ({}); the limit is {MAX_REGIONS}",
                regions.len()
            );
        }
        Ok(Self::List(regions))
    }
}

/// `us-east-1`, `ap-southeast-4`, `us-gov-west-1`, `cn-northwest-1`.
fn is_region_code(code: &str) -> bool {
    let parts: Vec<_> = code.split('-').collect();
    parts.len() >= 3
        && parts.iter().all(|p| !p.is_empty())
        && parts[..parts.len() - 1]
            .iter()
            .all(|p| p.bytes().all(|b| b.is_ascii_lowercase()))
        && parts[parts.len() - 1].bytes().all(|b| b.is_ascii_digit())
}

/// One search cache per Region, so cached inventories never mix Regions.
pub type SharedRegionCaches = Arc<Mutex<HashMap<String, SharedSearchCache>>>;

async fn cache_for(caches: &SharedRegionCaches, region: &str) -> SharedSearchCache {
    caches
        .lock()
        .await
        .entry(region.to_string())
        .or_insert_with(|| Arc::new(Mutex::new(SearchCache::default())))
        .clone()
}

/// Enabled Regions for the account, sorted, with the active Region first.
async fn enabled_regions(config: &AwsConfig) -> Result<Vec<String>> {
    let out = tokio::time::timeout(
        DESCRIBE_REGIONS_TIMEOUT,
        config.ec2_client().describe_regions().send(),
    )
    .await
    .map_err(|_| anyhow::anyhow!("DescribeRegions timed out"))??;
    let mut regions: Vec<String> = out
        .regions()
        .iter()
        .filter_map(|r| r.region_name())
        .filter(|name| is_region_code(name))
        .map(str::to_string)
        .collect();
    regions.sort();
    regions.dedup();
    if let Some(at) = regions.iter().position(|r| *r == config.region) {
        let current = regions.remove(at);
        regions.insert(0, current);
    }
    if regions.is_empty() {
        bail!("DescribeRegions returned no enabled Regions");
    }
    Ok(regions)
}

/// The single entry point used by the terminal. Without a `region:` qualifier
/// this is exactly the single-Region search of the active Region.
pub async fn search_regions(
    config: AwsConfig,
    query: SearchQuery,
    caches: SharedRegionCaches,
    tx: mpsc::Sender<SearchUpdate>,
) {
    if !query.ready() {
        return;
    }
    let regions = match &query.regions {
        super::RegionScope::Current => vec![config.region.clone()],
        super::RegionScope::List(list) => list.clone(),
        super::RegionScope::All => match enabled_regions(&config).await {
            Ok(regions) => regions,
            Err(error) => {
                let _ = tx
                    .send(SearchUpdate::Warning(format!(
                        "Could not list enabled Regions ({error:#}); searching {} only",
                        config.region
                    )))
                    .await;
                vec![config.region.clone()]
            }
        },
    };
    if regions.len() == 1 {
        let cache = cache_for(&caches, &regions[0]).await;
        search(config.for_region(&regions[0]), query, cache, tx).await;
        return;
    }
    fan_out(regions, query, tx, |region, query, tx| {
        let config = config.for_region(&region);
        let caches = caches.clone();
        async move {
            let cache = cache_for(&caches, &region).await;
            search(config, query, cache, tx).await;
        }
    })
    .await;
}

/// Run `run` once per Region (at most `REGION_CONCURRENCY` at a time) and
/// merge its updates into `tx`: results pass through, warnings gain a Region
/// prefix, per-Region provider notes are folded into one, and a single
/// `Finished` closes the whole search.
pub(super) async fn fan_out<F, Fut>(
    regions: Vec<String>,
    query: SearchQuery,
    tx: mpsc::Sender<SearchUpdate>,
    run: F,
) where
    F: Fn(String, SearchQuery, mpsc::Sender<SearchUpdate>) -> Fut,
    Fut: Future<Output = ()>,
{
    if tx
        .send(SearchUpdate::Provider("Parallel multi-Region search"))
        .await
        .is_err()
    {
        return;
    }
    let work = regions
        .into_iter()
        .enumerate()
        .filter_map(|(index, region)| {
            let mut query = query.clone();
            if index > 0 {
                // Account-wide inventories were already read in the first Region.
                query.kinds.retain(|kind| !kind.is_global());
                query.include_global = false;
                if query.kinds.is_empty() {
                    return None;
                }
            }
            let (region_tx, mut region_rx) = mpsc::channel(32);
            let tx = tx.clone();
            let search = run(region.clone(), query, region_tx);
            Some(async move {
                // The forwarder owns the receiver: if the finder goes away it
                // returns, the receiver drops, and the Region search stops on its
                // next send instead of blocking on a full channel.
                let forward = async move {
                    let mut limited = false;
                    while let Some(update) = region_rx.recv().await {
                        let update = match update {
                            SearchUpdate::Results(rows) => SearchUpdate::Results(rows),
                            SearchUpdate::Warning(warning) => {
                                SearchUpdate::Warning(format!("{region}: {warning}"))
                            }
                            SearchUpdate::Provider(_) => continue,
                            SearchUpdate::Finished { limited: capped } => {
                                limited |= capped;
                                continue;
                            }
                        };
                        if tx.send(update).await.is_err() {
                            return None;
                        }
                    }
                    Some(limited)
                };
                let ((), limited) = tokio::join!(search, forward);
                limited
            })
        });
    let mut stream = stream::iter(work).buffer_unordered(REGION_CONCURRENCY);
    let mut limited = false;
    while let Some(result) = stream.next().await {
        match result {
            Some(capped) => limited |= capped,
            // The finder closed or restarted the search: stop all Regions.
            None => return,
        }
    }
    let _ = tx.send(SearchUpdate::Finished { limited }).await;
}

#[cfg(test)]
mod tests;
