use crossbeam_channel::Receiver;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Result, Watcher};
use std::path::Path;

#[allow(missing_docs)]
pub struct FilesystemWatcher {
    pub watcher: RecommendedWatcher,
    pub receiver: Receiver<Result<Event>>,
}

impl Default for FilesystemWatcher {
    fn default() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let watcher: RecommendedWatcher = RecommendedWatcher::new(
            move |res| {
                sender.send(res).expect("Watch event send failure.");
            },
            Config::default(),
        )
        .expect("Failed to create filesystem watcher.");
        FilesystemWatcher { watcher, receiver }
    }
}

impl FilesystemWatcher {
    /// Watch for changes recursively at the provided path.
    pub fn watch<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.watcher.watch(path.as_ref(), RecursiveMode::Recursive)
    }
}

use bevy::prelude::*;
use bevy::utils::HashSet;
use crossbeam_channel::TryRecvError;

use super::WebAssetIo;

pub fn filesystem_watcher_system(asset_server: Res<AssetServer>) {
    let mut changed = HashSet::default();

    let asset_io = if let Some(asset_io) = asset_server.asset_io().downcast_ref::<WebAssetIo>() {
        asset_io
    } else {
        return;
    };

    let result = asset_io.filesystem_watcher.read();

    if let Ok(option) = result {
        if let Some(ref watcher) = *option {
            loop {
                let event = match watcher.receiver.try_recv() {
                    Ok(result) => result.unwrap(),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => panic!("FilesystemWatcher disconnected."),
                };
                if let notify::event::Event {
                    kind: notify::event::EventKind::Modify(_),
                    paths,
                    ..
                } = event
                {
                    for path in &paths {
                        if !changed.contains(path) {
                            let relative_path = path.strip_prefix(&asset_io.root_path).unwrap();
                            asset_server.reload_asset(relative_path);
                        }
                    }
                    changed.extend(paths);
                }
            }
        }
    }
}
