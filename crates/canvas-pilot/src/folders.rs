//! Local layer folders, with atomic history for operations on their members.
use crate::Layout;

impl Layout {
    /// Folder names in front-to-back display order. Empty means the document root.
    pub fn folders(&self) -> Vec<String> {
        let mut folders = Vec::new();
        for info in self.info.iter().rev() {
            if !info.folder.is_empty() && !folders.contains(&info.folder) {
                folders.push(info.folder.clone());
            }
        }
        for name in &self.folders {
            if !folders.contains(name) {
                folders.push(name.clone());
            }
        }
        folders
    }

    pub fn folder_members(&self, folder: &str) -> Vec<usize> {
        self.info
            .iter()
            .enumerate()
            .filter_map(|(i, info)| (info.folder == folder).then_some(i))
            .collect()
    }

    /// Assign stable IDs to a folder (or root), preserving their geometry and links.
    pub fn assign_folder(&mut self, ids: &[usize], folder: &str) -> Result<(), String> {
        if folder.chars().count() > 128
            || ids.is_empty()
            || ids
                .iter()
                .any(|id| !self.info.iter().any(|info| info.id == *id))
        {
            return Err(
                "Choose existing layers and a folder name of at most 128 characters.".into(),
            );
        }
        self.finish_drag();
        let before = self.snapshot();
        for info in &mut self.info {
            if ids.contains(&info.id) {
                info.folder = folder.trim().into();
            }
        }
        self.remember(before);
        Ok(())
    }

    /// Explicit folders remain available even when their last layer is moved out.
    pub fn create_folder(&mut self, name: &str) -> Result<(), String> {
        self.create_folder_with_layers(name, &[])
    }

    pub fn create_folder_with_layers(&mut self, name: &str, ids: &[usize]) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty()
            || name.chars().count() > 128
            || self.folders.len() >= 256
            || self.folders().iter().any(|f| f == name)
            || ids.iter().any(|id| !self.info.iter().any(|l| l.id == *id))
        {
            return Err("Use a unique folder name of 1–128 characters and existing layers (maximum 256 created folders).".into());
        }
        self.finish_drag();
        let before = self.snapshot();
        self.folders.push(name.into());
        for info in &mut self.info {
            if ids.contains(&info.id) {
                info.folder = name.into();
            }
        }
        self.remember(before);
        Ok(())
    }

    pub fn rename_folder(&mut self, folder: &str, name: &str) -> Result<(), String> {
        let name = name.trim();
        if folder.is_empty()
            || name.is_empty()
            || name.chars().count() > 128
            || !self.folders().iter().any(|f| f == folder)
            || (folder != name && self.folders().iter().any(|f| f == name))
        {
            return Err("Use an existing folder and a unique name of 1–128 characters.".into());
        }
        self.finish_drag();
        let before = self.snapshot();
        for f in &mut self.folders {
            if f == folder {
                *f = name.into();
            }
        }
        for info in &mut self.info {
            if info.folder == folder {
                info.folder = name.into();
            }
        }
        self.remember(before);
        Ok(())
    }

    /// Remove a folder without deleting any of its layers.
    pub fn ungroup_folder(&mut self, folder: &str) -> Result<(), String> {
        if folder.is_empty() || !self.folders().iter().any(|f| f == folder) {
            return Err("Folder no longer exists.".into());
        }
        self.finish_drag();
        let before = self.snapshot();
        self.folders.retain(|f| f != folder);
        for info in &mut self.info {
            if info.folder == folder {
                info.folder.clear();
            }
        }
        self.remember(before);
        Ok(())
    }

    pub fn select_folder(&mut self, folder: &str, toggle: bool) {
        let members = self.folder_members(folder);
        let all = members
            .iter()
            .filter(|&&i| self.selectable(i))
            .all(|&i| self.selected[i]);
        self.set_selection(
            (0..self.layers.len())
                .map(|i| {
                    if members.contains(&i) {
                        !toggle || !all
                    } else {
                        toggle && self.selected[i]
                    }
                })
                .collect::<Vec<_>>(),
        );
    }

    /// Mixed groups become fully visible/unlocked when the caller requests it.
    pub fn set_folder_state(&mut self, folder: &str, visible: Option<bool>, locked: Option<bool>) {
        self.cancel_drag();
        let before = self.snapshot();
        for i in self.folder_members(folder) {
            if let Some(value) = visible {
                self.visible[i] = value;
            }
            if let Some(value) = locked {
                self.locked[i] = value;
            }
            self.selected[i] &= self.visible[i] && !self.locked[i];
        }
        self.remember(before);
    }

    fn order_layers(&mut self, order: &[usize]) {
        let before = self.snapshot();
        self.info = order.iter().map(|&i| before.info[i].clone()).collect();
        self.layers = order.iter().map(|&i| before.layers[i]).collect();
        self.selected = order.iter().map(|&i| before.selected[i]).collect();
        self.visible = order.iter().map(|&i| before.visible[i]).collect();
        self.locked = order.iter().map(|&i| before.locked[i]).collect();
        self.formats.source_for = order
            .iter()
            .map(|&i| before.formats.source_for[i])
            .collect();
        self.formats.advanced = order.iter().map(|&i| before.formats.advanced[i]).collect();
        self.formats.layer_fit = order.iter().map(|&i| before.formats.layer_fit[i]).collect();
        self.formats.masks = order.iter().map(|&i| before.formats.masks[i]).collect();
    }

    /// Move a folder as one stacking block, keeping member order and identities.
    pub fn move_folder(
        &mut self,
        folder: &str,
        target_id: usize,
        above: bool,
    ) -> Result<(), String> {
        let target = self
            .info
            .iter()
            .position(|info| info.id == target_id)
            .ok_or("Target layer no longer exists.")?;
        let members = self.folder_members(folder);
        if folder.is_empty() || members.is_empty() {
            return Err("Folder no longer exists.".into());
        }
        if members.contains(&target) {
            return Ok(());
        }
        self.finish_drag();
        let before = self.snapshot();
        let mut order = (0..self.layers.len())
            .filter(|i| !members.contains(i))
            .collect::<Vec<_>>();
        let slot = order.iter().position(|&i| i == target).unwrap() + usize::from(above);
        order.splice(slot..slot, members);
        self.order_layers(&order);
        self.remember(before);
        Ok(())
    }

    /// A row drop changes membership and stacking in one undoable operation.
    pub fn move_layer_to_folder(
        &mut self,
        id: usize,
        folder: &str,
        to: usize,
    ) -> Result<(), String> {
        let from = self
            .info
            .iter()
            .position(|info| info.id == id)
            .ok_or("Layer no longer exists.")?;
        if folder.chars().count() > 128 || to >= self.layers.len() {
            return Err("Invalid folder or layer position.".into());
        }
        self.finish_drag();
        let before = self.snapshot();
        self.info[from].folder = folder.trim().into();
        let mut order = (0..self.layers.len()).collect::<Vec<_>>();
        order.remove(from);
        order.insert(to, from);
        self.order_layers(&order);
        self.remember(before);
        Ok(())
    }
}
