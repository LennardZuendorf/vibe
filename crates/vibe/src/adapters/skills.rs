//! Skill links into shared skills directories.
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::Ctx;
use crate::plan::{Note, OwnedEdit, PlanItem, Projection};

fn link_edit(path: &Path, target: &Path, present: bool) -> OwnedEdit {
    OwnedEdit::Link {
        path: path.display().to_string(),
        target: target.display().to_string(),
        present,
    }
}

fn owned_target<'a>(ctx: &'a Ctx, link: &Path) -> Option<&'a str> {
    let key = link.display().to_string();
    ctx.state
        .owned
        .links
        .iter()
        .find(|(p, _)| *p == key)
        .map(|(_, t)| t.as_str())
}

/// Parents of `link` that do not exist yet and that no earlier item already creates,
/// outermost first.
fn missing_parents(link: &Path, planned: &mut BTreeSet<PathBuf>) -> Vec<PathBuf> {
    let mut missing = Vec::new();
    let mut at = link.parent();
    while let Some(dir) = at {
        if dir.as_os_str().is_empty() || dir.exists() || planned.contains(dir) {
            break;
        }
        missing.push(dir.to_path_buf());
        at = dir.parent();
    }
    missing.reverse();
    planned.extend(missing.iter().cloned());
    missing
}

fn plan_create(p: &mut Projection, link: PathBuf, target: &Path, planned: &mut BTreeSet<PathBuf>) {
    let idx = p.items.len();
    for dir in missing_parents(&link, planned) {
        p.edits.push((
            idx,
            OwnedEdit::Created {
                path: dir.display().to_string(),
            },
        ));
    }
    p.edits.push((idx, link_edit(&link, target, true)));
    p.items.push(PlanItem::LinkCreate {
        path: link,
        target: target.to_path_buf(),
    });
}

/// Plans a link for each store skill into every directory of `dirs`, removes owned links
/// of skills the store no longer holds and reports shadowed skills. A name that is a real
/// directory, a file or a link vibe did not create is left alone and reported.
pub fn project(ctx: &Ctx, dirs: &[PathBuf]) -> Projection {
    let mut p = Projection::default();
    for s in &ctx.store.shadowed {
        p.notes.push(Note::Info(format!(
            "skill {}: kept {}, shadowed {}",
            s.name,
            s.kept.display(),
            s.dropped.display()
        )));
    }
    if !cfg!(unix) && !ctx.store.skills.is_empty() && !dirs.is_empty() {
        p.notes.push(Note::Info(
            "skills: links unsupported on this OS \u{2014} skipped".to_string(),
        ));
        return p;
    }
    let mut wanted: BTreeSet<PathBuf> = BTreeSet::new();
    let mut planned = BTreeSet::new();
    for dir in dirs {
        for skill in &ctx.store.skills {
            let link = dir.join(&skill.name);
            wanted.insert(link.clone());
            match fs::symlink_metadata(&link) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    plan_create(&mut p, link, &skill.dir, &mut planned);
                }
                Ok(m) if m.file_type().is_symlink() => {
                    let current = fs::read_link(&link).ok();
                    if current.as_deref() == Some(skill.dir.as_path()) {
                        continue;
                    }
                    let ours = current
                        .as_deref()
                        .is_some_and(|c| owned_target(ctx, &link) == c.to_str());
                    if ours && let Some(old) = current {
                        p.items.push(PlanItem::LinkRemove {
                            path: link.clone(),
                            target: old,
                        });
                        plan_create(&mut p, link, &skill.dir, &mut planned);
                    } else {
                        refuse_link(ctx, &mut p, &link);
                    }
                }
                Ok(_) | Err(_) => refuse_link(ctx, &mut p, &link),
            }
        }
    }
    for (path, target) in &ctx.state.owned.links {
        let link = PathBuf::from(path);
        let in_dirs = link.parent().is_some_and(|d| dirs.iter().any(|x| x == d));
        if in_dirs && !wanted.contains(&link) {
            let idx = p.items.len();
            p.items.push(PlanItem::LinkRemove {
                path: link.clone(),
                target: PathBuf::from(target),
            });
            p.edits
                .push((idx, link_edit(&link, Path::new(target), false)));
        }
    }
    p
}

/// A name that is not vibe's link: a recorded link that a user replaced is released once,
/// anything else is reported as skipped.
fn refuse_link(ctx: &Ctx, p: &mut Projection, link: &Path) {
    if let Some(target) = owned_target(ctx, link) {
        p.notes.push(Note::Kept(format!(
            "skills: {} replaced by user \u{2014} no longer managed",
            link.display()
        )));
        p.edits
            .push((usize::MAX, link_edit(link, Path::new(target), false)));
    } else {
        p.notes.push(not_ours(link));
    }
}

fn not_ours(link: &Path) -> Note {
    Note::Skipped(format!(
        "skills: {} exists, not a vibe link",
        link.display()
    ))
}
