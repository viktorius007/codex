use std::collections::HashMap;
use std::path::Component;
use std::path::Path;
use std::sync::Arc;

use crate::SkillLoadOutcome;
use codex_exec_server::LOCAL_FS;
use codex_exec_server::ReadFileOptions;
use codex_skills::SkillMetadata;
use codex_utils_path_uri::PathUri;

use crate::catalog::SkillAuthority;
use crate::catalog::SkillCatalog;
use crate::catalog::SkillCatalogEntry;
use crate::catalog::SkillPackageId;
use crate::catalog::SkillProviderError;
use crate::catalog::SkillReadResult;
use crate::catalog::SkillResourceId;
use crate::catalog::SkillSearchResult;
use crate::catalog::SkillSourceKind;
use crate::provider::SkillListQuery;
use crate::provider::SkillProvider;
use crate::provider::SkillProviderFuture;
use crate::provider::SkillReadRequest;
use crate::provider::SkillSearchRequest;

const HOST_AUTHORITY_ID: &str = "host";

/// Host-owned skill provider backed by an immutable service snapshot.
///
/// Discovery and caching belong to `HostSkillsService`; this provider only maps a
/// snapshot into the authority-aware catalog/read contract.
#[derive(Clone, Default)]
pub struct HostSkillProvider;

impl HostSkillProvider {
    pub fn new() -> Self {
        Self
    }
}

impl SkillProvider for HostSkillProvider {
    fn list(&self, query: SkillListQuery) -> SkillProviderFuture<'_, SkillCatalog> {
        Box::pin(async move {
            let Some(host_snapshot) = query.host_snapshot else {
                return Err(SkillProviderError::new(
                    "host skill provider requires a host skills snapshot",
                ));
            };

            Ok(catalog_from_outcome(host_snapshot.outcome()))
        })
    }

    fn read<'a>(
        &'a self,
        request: SkillReadRequest<'a>,
    ) -> SkillProviderFuture<'a, SkillReadResult> {
        Box::pin(async move {
            let Some(host_snapshot) = request.host_snapshot else {
                return Err(SkillProviderError::new(
                    "host skill provider requires a host skills snapshot",
                ));
            };
            let outcome = host_snapshot.outcome();
            if let Some(skill) = outcome.skills.iter().find(|skill| {
                plugin_skill_locator(outcome, skill)
                    .is_some_and(|(package, _)| package == request.package)
            }) {
                let outside_package = || {
                    SkillProviderError::new(format!(
                        "host skill resource is outside its package: {}",
                        request.resource.as_str()
                    ))
                };
                let read_error = || {
                    SkillProviderError::new(format!(
                        "failed to read host skill resource {}",
                        request.resource.as_str()
                    ))
                };
                let relative_resource = request
                    .package
                    .relative_resource_path(request.resource.as_str())
                    .ok_or_else(&outside_package)?;
                let relative_path = Path::new(relative_resource);
                if !relative_path
                    .components()
                    .all(|component| matches!(component, Component::Normal(_)))
                {
                    return Err(outside_package());
                }
                let package_root = skill.path_to_skills_md.parent().ok_or_else(|| {
                    SkillProviderError::new("host skill package has no parent directory")
                })?;
                let requested_path = package_root.join(relative_path);
                let file_system = outcome
                    .file_system_for_skill(skill)
                    .unwrap_or_else(|| Arc::clone(&LOCAL_FS));
                let resource = file_system
                    .canonicalize(
                        &PathUri::from_abs_path(&requested_path),
                        /*sandbox*/ None,
                    )
                    .await
                    .map_err(|_| read_error())?;
                let resolved_path = resource.to_abs_path().map_err(|_| read_error())?;
                if !resolved_path.as_path().starts_with(package_root.as_path()) {
                    return Err(outside_package());
                }
                let contents = file_system
                    .read_file_text(&resource, ReadFileOptions::default(), /*sandbox*/ None)
                    .await
                    .map_err(|_| read_error())?;

                return Ok(SkillReadResult {
                    resource: request.resource,
                    contents,
                });
            }

            let Some(skill) = outcome.skills.iter().find(|skill| {
                let skill_path = skill.path_to_skills_md.to_string_lossy();
                skill_path == request.resource.as_str()
                    || skill_path.replace('\\', "/") == request.resource.as_str()
            }) else {
                return Err(SkillProviderError::new(format!(
                    "host skill resource is not loaded: {}",
                    request.resource.as_str()
                )));
            };

            let contents = host_snapshot.read_skill_text(skill).await.map_err(|err| {
                SkillProviderError::new(format!(
                    "failed to read host skill resource {}: {err}",
                    request.resource.as_str()
                ))
            })?;

            Ok(SkillReadResult {
                resource: request.resource,
                contents,
            })
        })
    }

    fn search(&self, _request: SkillSearchRequest) -> SkillProviderFuture<'_, SkillSearchResult> {
        Box::pin(async { Ok(SkillSearchResult::default()) })
    }
}

fn catalog_from_outcome(outcome: &SkillLoadOutcome) -> SkillCatalog {
    let root_order_by_path = outcome
        .skill_roots_in_discovery_order()
        .enumerate()
        .map(|(index, root)| (root.as_path(), index))
        .collect::<HashMap<_, _>>();
    let mut catalog = SkillCatalog {
        entries: Vec::new(),
        warnings: outcome
            .errors
            .iter()
            .map(|err| {
                format!(
                    "Failed to load skill at {}: {}",
                    err.path.display(),
                    err.message
                )
            })
            .collect(),
    };

    for (skill, enabled) in outcome.skills_with_enabled() {
        let mut entry = catalog_entry_from_skill(outcome, skill, enabled);
        if !entry.is_plugin_package() {
            if let Some(discovery_path) =
                outcome.skill_discovery_path_for_path(&skill.path_to_skills_md)
            {
                entry =
                    entry.with_display_path(discovery_path.to_string_lossy().replace('\\', "/"));
            }
            if let Some(root) = outcome.skill_root_for_path(&skill.path_to_skills_md) {
                entry = entry.with_alias_root(root.to_string_lossy().replace('\\', "/"));
                if let Some(root_order) = root_order_by_path.get(root.as_path()) {
                    entry = entry.with_alias_root_order(*root_order);
                }
            }
        }
        catalog.push_entry(entry);
    }

    catalog
}

fn catalog_entry_from_skill(
    outcome: &SkillLoadOutcome,
    skill: &SkillMetadata,
    enabled: bool,
) -> SkillCatalogEntry {
    let skill_path = skill.path_to_skills_md.to_string_lossy().into_owned();
    let display_path = skill_path.replace('\\', "/");
    let plugin_locator = plugin_skill_locator(outcome, skill);
    let is_plugin_package = plugin_locator.is_some();
    let (package, main_prompt) = plugin_locator.unwrap_or_else(|| {
        (
            SkillPackageId(skill_path.clone()),
            SkillResourceId::new(skill_path),
        )
    });
    let mut entry = SkillCatalogEntry::new(
        package,
        SkillAuthority::new(SkillSourceKind::Host, HOST_AUTHORITY_ID),
        skill.name.clone(),
        skill.description.clone(),
        main_prompt,
    )
    .with_short_description(skill.short_description.clone())
    .with_prompt_scope(skill.scope)
    .with_dependencies(skill.dependencies.clone());
    entry.plugin_id = skill.plugin_id.clone();
    if !is_plugin_package {
        entry = entry.with_display_path(display_path);
    }

    if !enabled {
        entry = entry.disabled();
    }
    if !skill.allows_implicit_invocation() {
        entry = entry.hidden_from_prompt();
    }

    entry
}

fn plugin_skill_locator(
    outcome: &SkillLoadOutcome,
    skill: &SkillMetadata,
) -> Option<(SkillPackageId, SkillResourceId)> {
    let plugin_id = skill.plugin_id.as_deref()?;
    let root = outcome.skill_root_for_path(&skill.path_to_skills_md)?;
    let relative_directory = skill.path_to_skills_md.strip_prefix(root).ok()?.parent()?;
    let relative_directory = relative_directory.to_string_lossy().replace('\\', "/");
    let package = if relative_directory.is_empty() {
        format!("skill://{plugin_id}")
    } else {
        format!("skill://{plugin_id}/{relative_directory}")
    };
    let resource = format!("{package}/SKILL.md");
    Some((SkillPackageId(package), SkillResourceId::new(resource)))
}

#[cfg(test)]
#[path = "host_tests.rs"]
mod tests;
