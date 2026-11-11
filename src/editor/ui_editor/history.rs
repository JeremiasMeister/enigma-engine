use crate::ui_runtime::UiScreen;

#[derive(Default)]
pub struct History {
    undo: Vec<Vec<UiScreen>>,
    redo: Vec<Vec<UiScreen>>,
    pending: Option<Vec<UiScreen>>,
}

impl History {
    pub fn changed(&mut self, before: Vec<UiScreen>, after: &[UiScreen]) {
        if before != after {
            self.pending.get_or_insert(before);
            self.redo.clear();
        }
    }
    pub fn finish(&mut self, current: &[UiScreen]) {
        if let Some(before) = self.pending.take() {
            if before != current {
                self.undo.push(before);
                if self.undo.len() > 64 {
                    self.undo.remove(0);
                }
            }
        }
    }
    pub fn undo(&mut self, current: &mut Vec<UiScreen>) -> bool {
        self.finish(current);
        if let Some(previous) = self.undo.pop() {
            self.redo.push(std::mem::replace(current, previous));
            true
        } else {
            false
        }
    }
    pub fn redo(&mut self, current: &mut Vec<UiScreen>) -> bool {
        self.finish(current);
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(current, next));
            true
        } else {
            false
        }
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty() || self.pending.is_some()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continuous_edits_are_one_undo_and_redo_preserves_result() {
        let mut current = vec![UiScreen::pause_menu("one")];
        let original = current.clone();
        let mut history = History::default();
        for value in [10.0, 20.0, 30.0] {
            let before = current.clone();
            current[0].nodes[0].position[0] = value;
            history.changed(before, &current);
        }
        history.finish(&current);
        let edited = current.clone();
        assert!(history.undo(&mut current));
        assert_eq!(current, original);
        assert!(!history.undo(&mut current));
        assert!(history.redo(&mut current));
        assert_eq!(current, edited);
    }

    #[test]
    fn undo_restores_deleted_screen_and_new_edit_invalidates_redo() {
        let mut current = vec![UiScreen::dialogue("one")];
        let mut history = History::default();
        let before = current.clone();
        current.clear();
        history.changed(before, &current);
        assert!(history.undo(&mut current));
        assert_eq!(current[0].id, "one");
        let before = current.clone();
        current[0].name = "New name".into();
        history.changed(before, &current);
        history.finish(&current);
        assert!(!history.redo(&mut current));
    }
}
