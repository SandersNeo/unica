use crate::domain::cache::{path_for_report, CacheAccess, CacheImpact, CacheReport};
use crate::domain::events::DomainEvent;
use crate::domain::workspace::WorkspaceContext;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkspaceState {
    workspace_root: String,
    workspace_epoch: u64,
    caches: BTreeMap<String, CacheEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    status: CacheStatus,
    epoch: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum CacheStatus {
    Fresh,
    Stale,
}

pub struct WorkspaceStateRepository {
    state_path: PathBuf,
}

impl WorkspaceStateRepository {
    pub fn new(context: &WorkspaceContext) -> Self {
        Self {
            state_path: context.cache_root.join("state.json"),
        }
    }

    pub fn report(
        &self,
        context: &WorkspaceContext,
        events: &[DomainEvent],
        dry_run: bool,
        cache_access: CacheAccess,
    ) -> Result<CacheReport, String> {
        let impact = CacheImpact::from_events(events);
        let mut state = self.load(context);
        let mut invalidated = sorted(impact.invalidated);
        let mut refreshed = sorted(impact.eager_refresh);
        let mut lazy_rebuilt = Vec::new();

        if !events.is_empty() && !dry_run {
            for name in &invalidated {
                state.caches.insert(
                    name.clone(),
                    CacheEntry {
                        status: CacheStatus::Stale,
                        epoch: context.workspace_epoch,
                    },
                );
            }
            for name in &refreshed {
                state.caches.insert(
                    name.clone(),
                    CacheEntry {
                        status: CacheStatus::Fresh,
                        epoch: context.workspace_epoch,
                    },
                );
                self.write_cache_metadata(context, name, "eager")?;
            }
            state.workspace_epoch = context.workspace_epoch;
            self.save(&state)?;
        }

        if dry_run {
            refreshed.clear();
        }
        if events.is_empty() {
            invalidated.clear();
            refreshed.clear();
        }

        if events.is_empty() && !dry_run {
            for name in cache_access.reads {
                let is_stale = state
                    .caches
                    .get(*name)
                    .map(|entry| entry.status == CacheStatus::Stale)
                    .unwrap_or_else(|| is_lazy_cache(name));
                if is_stale && is_lazy_cache(name) {
                    state.caches.insert(
                        (*name).to_string(),
                        CacheEntry {
                            status: CacheStatus::Fresh,
                            epoch: context.workspace_epoch,
                        },
                    );
                    self.write_cache_metadata(context, name, "lazy")?;
                    lazy_rebuilt.push((*name).to_string());
                }
            }
            if !lazy_rebuilt.is_empty() {
                state.workspace_epoch = context.workspace_epoch;
                self.save(&state)?;
            }
        }

        let mut stale = Vec::new();
        let mut fresh = Vec::new();
        for (name, entry) in state.caches {
            match entry.status {
                CacheStatus::Fresh => fresh.push(name),
                CacheStatus::Stale => stale.push(name),
            }
        }

        Ok(CacheReport {
            mode: if events.is_empty() {
                "read".to_string()
            } else if dry_run {
                "dry-run".to_string()
            } else {
                "applied".to_string()
            },
            root: path_for_report(&context.cache_root),
            workspace_epoch: context.workspace_epoch,
            events: events
                .iter()
                .map(|event| event.name().to_string())
                .collect(),
            invalidated,
            refreshed,
            lazy_rebuilt,
            stale,
            fresh,
        })
    }

    fn load(&self, context: &WorkspaceContext) -> WorkspaceState {
        let Ok(text) = fs::read_to_string(&self.state_path) else {
            return default_state(context);
        };
        serde_json::from_str(&text).unwrap_or_else(|_| default_state(context))
    }

    fn save(&self, state: &WorkspaceState) -> Result<(), String> {
        if let Some(parent) = self.state_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("failed to create Unica cache directory: {err}"))?;
        }
        let text = serde_json::to_string_pretty(state).map_err(|err| err.to_string())?;
        fs::write(&self.state_path, text + "\n")
            .map_err(|err| format!("failed to write Unica cache state: {err}"))
    }

    fn write_cache_metadata(
        &self,
        context: &WorkspaceContext,
        name: &str,
        mode: &str,
    ) -> Result<(), String> {
        let dir = context.cache_root.join("caches");
        fs::create_dir_all(&dir)
            .map_err(|err| format!("failed to create Unica cache metadata directory: {err}"))?;
        let text = serde_json::json!({
            "name": name,
            "mode": mode,
            "workspaceEpoch": context.workspace_epoch,
        });
        fs::write(
            dir.join(format!("{name}.json")),
            serde_json::to_string_pretty(&text).map_err(|err| err.to_string())? + "\n",
        )
        .map_err(|err| format!("failed to write Unica cache metadata for {name}: {err}"))
    }
}

fn default_state(context: &WorkspaceContext) -> WorkspaceState {
    WorkspaceState {
        workspace_root: context.workspace_root.display().to_string(),
        workspace_epoch: context.workspace_epoch,
        caches: BTreeMap::new(),
    }
}

fn sorted(values: std::collections::BTreeSet<String>) -> Vec<String> {
    values.into_iter().collect()
}

fn is_lazy_cache(name: &str) -> bool {
    matches!(name, "bsl_index" | "bsl_diagnostics")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::events::{DomainEvent, DomainEventKind};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn stale_heavy_index_is_rebuilt_lazily_when_read() {
        let root = temp_root("unica-cache-lazy");
        fs::create_dir_all(&root).unwrap();
        let context = WorkspaceContext {
            cwd: root.clone(),
            workspace_root: root.clone(),
            cache_root: root.join(".cache"),
            workspace_epoch: 1,
        };
        let repo = WorkspaceStateRepository::new(&context);

        let invalidation = repo
            .report(
                &context,
                &[DomainEvent::new(
                    DomainEventKind::ModuleChanged,
                    "Module.bsl",
                )],
                false,
                CacheAccess::default(),
            )
            .unwrap();
        assert!(invalidation.stale.contains(&"bsl_index".to_string()));

        let rebuilt = repo
            .report(
                &context,
                &[],
                false,
                CacheAccess {
                    reads: &["bsl_index"],
                    writes: &[],
                },
            )
            .unwrap();
        assert_eq!(rebuilt.lazy_rebuilt, vec!["bsl_index".to_string()]);
        assert!(context
            .cache_root
            .join("caches")
            .join("bsl_index.json")
            .is_file());

        let _ = fs::remove_dir_all(root);
    }

    fn temp_root(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}"))
    }
}
