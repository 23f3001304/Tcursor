use crate::export::render::{FrameRenderer, RenderMeta};
use crate::export::types::Aspect;
use crate::platform::Platform;
use crate::ports::system::SystemPort;
use crate::session::paths::ProjectPaths;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::SystemTime;

pub(crate) struct Cached {
    pub folder: String,
    pub mtime: Option<SystemTime>,
    pub aspect: Aspect,
    pub renderer: FrameRenderer,
    pub meta: RenderMeta,
}

pub(crate) struct WarmSlot<T> {
    cell: Mutex<Option<T>>,
    gate: Mutex<()>,
}

impl<T> Default for WarmSlot<T> {
    fn default() -> Self {
        Self {
            cell: Mutex::new(None),
            gate: Mutex::new(()),
        }
    }
}

impl<T> WarmSlot<T> {
    pub(crate) fn with<R>(
        &self,
        reuse: impl FnOnce(T) -> Option<T>,
        make: impl FnOnce() -> Result<T, String>,
        work: impl FnOnce(&mut T) -> Result<R, String>,
    ) -> Result<R, String> {
        let _gate = self.gate.lock().unwrap_or_else(|e| e.into_inner());
        let taken = self.cell.lock().unwrap_or_else(|e| e.into_inner()).take();
        let mut entry = match taken.and_then(reuse) {
            Some(e) => e,
            None => make()?,
        };
        let out = work(&mut entry);
        *self.cell.lock().unwrap_or_else(|e| e.into_inner()) = Some(entry);
        out
    }

    #[cfg(test)]
    pub(crate) fn cell_free(&self) -> bool {
        self.cell.try_lock().is_ok()
    }

    #[cfg(test)]
    pub(crate) fn is_warm(&self) -> bool {
        self.cell
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
    }
}

#[derive(Default)]
pub struct PreviewSession {
    slot: WarmSlot<Cached>,
    webcam: AtomicBool,
}

impl PreviewSession {
    pub fn has_webcam(&self) -> bool {
        self.webcam.load(Ordering::Relaxed)
    }
}

fn reuse(
    mut c: Cached,
    folder: &str,
    mtime: Option<SystemTime>,
    paths: &ProjectPaths,
) -> Option<Cached> {
    if c.folder != folder {
        return None;
    }
    if c.mtime == mtime {
        return Some(c);
    }
    if c.aspect != crate::edit::seed::load_or_seed(paths).aspect {
        return None;
    }
    c.renderer.reload_edit(paths);
    c.mtime = mtime;
    Some(c)
}

fn build(
    folder: &str,
    mtime: Option<SystemTime>,
    paths: &ProjectPaths,
    system: &dyn SystemPort,
) -> Result<Cached, String> {
    let aspect = crate::edit::seed::load_or_seed(paths).aspect;
    let (renderer, meta) = super::build_renderer(paths, system).map_err(|e| e.to_string())?;
    Ok(Cached {
        folder: folder.to_string(),
        mtime,
        aspect,
        renderer,
        meta,
    })
}

pub(crate) fn with_warm<T>(
    session: &PreviewSession,
    system: &dyn SystemPort,
    folder: &str,
    f: impl FnOnce(&mut Cached, &ProjectPaths) -> Result<T, String>,
) -> Result<T, String> {
    let paths = ProjectPaths {
        folder: PathBuf::from(folder),
    };
    let mtime = std::fs::metadata(paths.edit())
        .and_then(|m| m.modified())
        .ok();
    session.slot.with(
        |c| reuse(c, folder, mtime, &paths),
        || build(folder, mtime, &paths, system),
        |c| {
            session
                .webcam
                .store(c.renderer.has_webcam(), Ordering::Relaxed);
            f(c, &paths)
        },
    )
}

pub(crate) fn with_warm_app<T>(
    app: &tauri::AppHandle,
    folder: &str,
    f: impl FnOnce(&mut Cached, &ProjectPaths) -> Result<T, String>,
) -> Result<T, String> {
    use tauri::Manager;
    let session = app.state::<PreviewSession>();
    let platform = app.state::<Arc<Platform>>();
    with_warm(&session, platform.system.as_ref(), folder, f)
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
