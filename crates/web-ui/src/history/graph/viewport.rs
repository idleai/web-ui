//! Pixel windowing and item-relative scroll anchors, independent of the DOM.

use std::collections::BTreeMap;
use std::ops::Range;

pub(super) const ROW_HEIGHT: f64 = 40.0;
const OVERSCAN: f64 = 120.0;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Row {
    pub(super) key: String,
    pub(super) top: f64,
    pub(super) height: f64,
}

#[derive(Debug, Clone)]
pub(super) struct Viewport {
    pub(super) rows: Vec<Row>,
    indices: BTreeMap<String, usize>,
    measured: BTreeMap<String, f64>,
    pub(super) top: f64,
    pub(super) height: f64,
    pub(super) width: f64,
    pub(super) total: f64,
    pub(super) focused: Option<String>,
    pub(super) row_height: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            indices: BTreeMap::new(),
            measured: BTreeMap::new(),
            top: 0.0,
            height: 480.0,
            width: 800.0,
            total: 0.0,
            focused: None,
            row_height: ROW_HEIGHT,
        }
    }
}

impl Viewport {
    pub(super) fn set_compact(&mut self, compact: bool) {
        let height = if compact { 24.0 } else { ROW_HEIGHT };
        if (self.row_height - height).abs() < 0.5 {
            return;
        }
        self.row_height = height;
        self.measured.clear();
        let keys: Vec<_> = self.rows.iter().map(|row| row.key.clone()).collect();
        self.update(&keys);
    }

    pub(super) fn update(&mut self, keys: &[String]) {
        let anchor_index = self
            .rows
            .partition_point(|row| row.top + row.height <= self.top);
        let anchor = self
            .rows
            .get(anchor_index)
            .map(|row| (row.key.clone(), self.top - row.top));
        let old_rows = std::mem::take(&mut self.rows);
        let mut top = 0.0;
        self.rows = keys
            .iter()
            .map(|key| {
                let height = self.measured.get(key).copied().unwrap_or(self.row_height);
                let row = Row {
                    key: key.clone(),
                    top,
                    height,
                };
                top += height;
                row
            })
            .collect();
        self.total = top;
        self.indices = keys
            .iter()
            .enumerate()
            .map(|(index, key)| (key.clone(), index))
            .collect();
        self.measured
            .retain(|key, _| self.indices.contains_key(key));
        if let Some((key, offset)) = anchor {
            if let Some(row) = self.row(&key) {
                self.top = row.top + offset.min((row.height - 1.0).max(0.0));
            } else if let Some(row) = old_rows
                .iter()
                .skip(anchor_index)
                .chain(old_rows.iter().take(anchor_index).rev())
                .find_map(|old| self.row(&old.key))
            {
                // Prefer the next surviving item; preserve its previous screen position.
                let old = old_rows.iter().find(|old| old.key == row.key);
                self.top = old.map_or(row.top, |old| row.top - (old.top - self.top));
            }
        }
        if self
            .focused
            .as_ref()
            .is_none_or(|key| !self.indices.contains_key(key))
        {
            self.focused = self
                .rows
                .get(anchor_index.min(self.rows.len().saturating_sub(1)))
                .map(|row| row.key.clone());
        }
        self.clamp();
    }

    pub(super) fn row(&self, key: &str) -> Option<&Row> {
        self.indices
            .get(key)
            .and_then(|index| self.rows.get(*index))
    }

    pub(super) fn index(&self, key: &str) -> Option<usize> {
        self.indices.get(key).copied()
    }

    pub(super) fn measure(&mut self, key: &str, height: f64) -> bool {
        let height = finite(height, self.row_height).clamp(self.row_height, 1_000_000.0);
        let previous = self.measured.get(key).copied().unwrap_or(self.row_height);
        if !self.indices.contains_key(key) || (previous - height).abs() < 0.5 {
            return false;
        }
        let _: Option<f64> = self.measured.insert(key.into(), height);
        let keys: Vec<_> = self.rows.iter().map(|row| row.key.clone()).collect();
        self.update(&keys);
        true
    }

    pub(super) fn resize(&mut self, width: f64, height: f64) -> bool {
        let height = finite(height, self.height).clamp(1.0, 1_000_000.0);
        let width = finite(width, self.width).clamp(1.0, 1_000_000.0);
        if (height - self.height).abs() < 0.5 && (width - self.width).abs() < 0.5 {
            return false;
        }
        self.height = height;
        self.width = width;
        self.clamp();
        true
    }

    pub(super) fn scroll(&mut self, top: f64) {
        self.top = finite(top, self.top);
        self.clamp();
    }

    pub(super) fn window(&self) -> Range<usize> {
        let top = (self.top - OVERSCAN).max(0.0);
        let bottom = self.top + self.height + OVERSCAN;
        let start = self.rows.partition_point(|row| row.top + row.height < top);
        let end = self.rows.partition_point(|row| row.top <= bottom);
        start..end
    }

    pub(super) fn mounted_indices(&self) -> Vec<usize> {
        let mut indices: Vec<_> = self.window().collect();
        if let Some(index) = self.focused.as_ref().and_then(|key| self.index(key))
            && !indices.contains(&index)
        {
            indices.push(index);
            indices.sort_unstable();
        }
        indices
    }

    pub(super) fn focus(&mut self, index: usize) {
        let Some(row) = self.rows.get(index) else {
            return;
        };
        self.focused = Some(row.key.clone());
        if row.top < self.top {
            self.top = row.top;
        } else if row.top + row.height > self.top + self.height {
            self.top = row.top + row.height.min(self.height) - self.height;
        }
        self.clamp();
    }

    fn clamp(&mut self) {
        self.top = self.top.clamp(0.0, (self.total - self.height).max(0.0));
    }
}

pub(super) fn finite(value: f64, fallback: f64) -> f64 {
    if value.is_finite() { value } else { fallback }
}

pub(super) fn number(value: usize) -> f64 {
    f64::from(u32::try_from(value).unwrap_or(u32::MAX))
}
