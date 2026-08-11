use std::collections::{BTreeMap, BTreeSet};

use uuid::Uuid;

use crate::domain::{Action, Chord, Mapping};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ConflictSeverity {
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ConflictCode {
    DirectSelfMap,
    DuplicateEnabledTrigger,
    EmitShortcutCycle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Conflict {
    pub code: ConflictCode,
    pub severity: ConflictSeverity,
    pub mapping_ids: Vec<Uuid>,
}

pub fn analyze(mappings: &[Mapping]) -> Vec<Conflict> {
    let active: Vec<_> = mappings.iter().filter(|mapping| mapping.enabled).collect();
    let mut trigger_ids: BTreeMap<&Chord, Vec<Uuid>> = BTreeMap::new();
    for mapping in &active {
        trigger_ids
            .entry(mapping.trigger.chord())
            .or_default()
            .push(mapping.id);
    }

    let mut conflicts = Vec::new();
    for ids in trigger_ids.values_mut() {
        ids.sort_unstable();
        if ids.len() > 1 {
            conflicts.push(Conflict {
                code: ConflictCode::DuplicateEnabledTrigger,
                severity: ConflictSeverity::Error,
                mapping_ids: ids.clone(),
            });
        }
    }

    let vertices: BTreeSet<_> = active.iter().map(|mapping| mapping.id).collect();
    let mut edges: BTreeMap<Uuid, BTreeSet<Uuid>> = vertices
        .iter()
        .copied()
        .map(|id| (id, BTreeSet::new()))
        .collect();
    for mapping in &active {
        if let Action::EmitShortcut { chord } = &mapping.action
            && let Some(targets) = trigger_ids.get(chord)
        {
            edges
                .entry(mapping.id)
                .or_default()
                .extend(targets.iter().copied());
        }
    }

    for component in strongly_connected_components(&vertices, &edges) {
        let mapping_ids: Vec<_> = component.iter().copied().collect();
        let self_edge = mapping_ids.len() == 1
            && edges
                .get(&mapping_ids[0])
                .is_some_and(|targets| targets.contains(&mapping_ids[0]));
        if mapping_ids.len() > 1 || self_edge {
            conflicts.push(Conflict {
                code: if self_edge {
                    ConflictCode::DirectSelfMap
                } else {
                    ConflictCode::EmitShortcutCycle
                },
                severity: ConflictSeverity::Error,
                mapping_ids,
            });
        }
    }

    conflicts.sort_by(|left, right| {
        (left.code, &left.mapping_ids).cmp(&(right.code, &right.mapping_ids))
    });
    conflicts
}

/// Deterministic Kosaraju SCC analysis over sorted vertices and edges.
fn strongly_connected_components(
    vertices: &BTreeSet<Uuid>,
    edges: &BTreeMap<Uuid, BTreeSet<Uuid>>,
) -> Vec<BTreeSet<Uuid>> {
    let mut visited = BTreeSet::new();
    let mut finish_order = Vec::new();
    for vertex in vertices {
        finish_dfs(*vertex, edges, &mut visited, &mut finish_order);
    }

    let mut reverse: BTreeMap<Uuid, BTreeSet<Uuid>> = vertices
        .iter()
        .copied()
        .map(|id| (id, BTreeSet::new()))
        .collect();
    for (source, targets) in edges {
        for target in targets {
            reverse.entry(*target).or_default().insert(*source);
        }
    }

    let mut components = Vec::new();
    visited.clear();
    for vertex in finish_order.into_iter().rev() {
        if visited.insert(vertex) {
            let mut component = BTreeSet::new();
            collect_dfs(vertex, &reverse, &mut visited, &mut component);
            components.push(component);
        }
    }
    components
}

fn finish_dfs(
    vertex: Uuid,
    edges: &BTreeMap<Uuid, BTreeSet<Uuid>>,
    visited: &mut BTreeSet<Uuid>,
    finish_order: &mut Vec<Uuid>,
) {
    if !visited.insert(vertex) {
        return;
    }
    let mut stack = vec![(vertex, false)];
    while let Some((current, finished)) = stack.pop() {
        if finished {
            finish_order.push(current);
            continue;
        }

        stack.push((current, true));
        if let Some(targets) = edges.get(&current) {
            for target in targets.iter().rev() {
                if visited.insert(*target) {
                    stack.push((*target, false));
                }
            }
        }
    }
}

fn collect_dfs(
    vertex: Uuid,
    edges: &BTreeMap<Uuid, BTreeSet<Uuid>>,
    visited: &mut BTreeSet<Uuid>,
    component: &mut BTreeSet<Uuid>,
) {
    component.insert(vertex);
    let mut stack = vec![vertex];
    while let Some(current) = stack.pop() {
        if let Some(targets) = edges.get(&current) {
            for target in targets.iter().rev() {
                if visited.insert(*target) {
                    component.insert(*target);
                    stack.push(*target);
                }
            }
        }
    }
}

pub fn analyze_candidate(existing: &[Mapping], candidate: &Mapping) -> Vec<Conflict> {
    let mut mappings: Vec<_> = existing
        .iter()
        .filter(|mapping| mapping.id != candidate.id)
        .cloned()
        .collect();
    let mut candidate = candidate.clone();
    candidate.enabled = true;
    mappings.push(candidate);
    analyze(&mappings)
}
