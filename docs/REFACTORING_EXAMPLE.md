# Example: Extracting TableView into a Separate Module

This document demonstrates a practical example of refactoring `model.rs` by extracting the `TableView` struct and its implementation into a dedicated module.

## Step-by-Step Process

### Step 1: Create the Module Structure

First, create the directory structure for the new module:

```bash
mkdir -p src/model/views
```

This creates:
```
src/
├── model/
│   ├── mod.rs              # Module entry point
│   └── views/
│       ├── mod.rs          # Views submodule
│       └── table_view.rs   # TableView implementation
└── model.rs                # Original (to be refactored)
```

### Step 2: Extract TableView Definition

**File: `src/model/views/table_view.rs`**

Extract the `TableView` struct and its implementation from `model.rs` (lines 102-158):

```rust
use std::collections::HashMap;
use std::sync::Arc;

use crate::model::ColumnView;

/// Represents the state and data for rendering a table view.
/// 
/// This struct maintains the current view state including cursor position,
/// visible columns, search results, and column histograms.
pub struct TableView {
    pub(crate) name: String,
    pub(crate) rows: Arc<Vec<usize>>,
    pub(crate) visible_columns: Vec<usize>,
    pub(crate) visible_width: usize,
    pub(crate) curser_row: usize,
    pub(crate) curser_column: usize,
    pub(crate) offset_row: usize,
    pub(crate) offset_column: usize,
    pub(crate) data: Vec<ColumnView>,
    pub(crate) search_results: Vec<(usize, usize)>,
    pub(crate) search_idx: usize,
    pub(crate) show_index: bool,
    pub(crate) index: ColumnView,
    pub(crate) column_histograms: HashMap<usize, (Vec<usize>, Vec<String>)>,
    pub(crate) heigh: usize,
    pub(crate) width: usize,
}

impl TableView {
    /// Creates an empty TableView with default values.
    pub(crate) fn empty() -> Self {
        TableView {
            name: String::new(),
            rows: Arc::new(Vec::new()),
            visible_columns: Vec::new(),
            visible_width: 0,
            curser_column: 0,
            curser_row: 0,
            offset_column: 0,
            offset_row: 0,
            data: Vec::new(),
            search_results: Vec::new(),
            search_idx: 0,
            show_index: false,
            index: ColumnView::empty(),
            column_histograms: HashMap::new(),
            heigh: 0,
            width: 0,
        }
    }

    /// Builds the index column based on current visible rows.
    pub(crate) fn build_index(&mut self) {
        let rbegin = self.offset_row;
        let rend = std::cmp::min(rbegin + self.heigh, self.rows.len());

        let data = self.rows[rbegin..rend]
            .iter()
            .map(|idx| (idx + 1).to_string())
            .collect::<Vec<String>>();
        let width = data.last().map(|s| s.len()).unwrap_or(3);
        self.index = ColumnView {
            name: "".to_string(),
            width,
            data,
        }
    }
}
```

**Key Changes:**
- Add documentation comments (`///`) for public items
- Use `pub(crate)` for fields to maintain encapsulation within the crate
- Import dependencies at the top (`HashMap`, `Arc`, `ColumnView`)

### Step 3: Create Views Module

**File: `src/model/views/mod.rs`**

```rust
pub mod table_view;

pub use table_view::TableView;
```

This file:
- Declares the `table_view` submodule
- Re-exports `TableView` for easier access

### Step 4: Create Model Module Entry Point

**File: `src/model/mod.rs`**

```rust
// Re-export views module
pub mod views;

// Re-export commonly used types
pub use views::TableView;

// ColumnView remains here as it's used by multiple components
#[derive(Clone)]
pub struct ColumnView {
    pub name: String,
    pub width: usize,
    pub data: Vec<String>,
}

impl ColumnView {
    pub(crate) fn empty() -> Self {
        ColumnView {
            name: "".to_string(),
            width: 0,
            data: Vec::new(),
        }
    }
}
```

This file:
- Re-exports the `views` module
- Re-exports `TableView` for convenient access
- Keeps `ColumnView` here since it's used by multiple modules

### Step 5: Modify model.rs

**Original `src/model.rs` (before changes):**
```rust
// Lines 70-158 contained:
pub struct ColumnView { ... }
impl ColumnView { ... }
pub struct TableView { ... }
impl TableView { ... }
```

**Modified `src/model.rs` (after changes):**

1. **Remove the extracted code** (lines 102-158):
   - Delete `pub struct TableView { ... }`
   - Delete `impl TableView { ... }`

2. **Add import at the top** (after line 11):
   ```rust
   use crate::model::TableView;
   ```

3. **Keep the rest of model.rs unchanged**

### Step 6: Update Main.rs (if needed)

If `main.rs` or other files imported types from `model.rs`, update them:

**Before:**
```rust
use crate::model::{Model, Status};
```

**After:**
```rust
use crate::model::{Model, Status, TableView};
// Or, if Model re-exports it:
use crate::model::Model;
```

Since we re-export `TableView` from `model/mod.rs`, existing code using `crate::model::TableView` continues to work without changes!

## Result

### Before Refactoring:
```
src/
└── model.rs (1,621 lines)
    ├── FileType enum
    ├── Status enum
    ├── FileInfo struct
    ├── Column struct
    ├── ColumnView struct
    ├── ColumnStatus enum
    ├── Modus enum
    ├── TableView struct ← 57 lines
    ├── RecordView struct
    ├── HistogramView struct
    ├── UIData struct
    ├── UILayout struct
    └── Model struct (huge impl block)
```

### After Refactoring:
```
src/
├── model.rs (1,564 lines - 57 lines removed)
│   ├── FileType enum
│   ├── Status enum
│   ├── FileInfo struct
│   ├── Column struct
│   ├── ColumnStatus enum
│   ├── Modus enum
│   ├── RecordView struct
│   ├── HistogramView struct
│   ├── UIData struct
│   ├── UILayout struct
│   └── Model struct
│
└── model/
    ├── mod.rs (with ColumnView)
    └── views/
        ├── mod.rs
        └── table_view.rs (TableView - 75 lines)
```

**Benefits:**
- ✅ `model.rs` reduced from 1,621 to 1,564 lines
- ✅ `TableView` has its own focused file
- ✅ Better organization and discoverability
- ✅ Easier to test `TableView` independently
- ✅ No breaking changes to existing code (re-exports maintain API)

## Testing the Refactoring

After making these changes, verify everything still works:

```bash
# Check that the code compiles
cargo build

# Run tests (if any exist)
cargo test

# Check for warnings
cargo clippy
```

## Next Steps

Following this same pattern, you can extract:

1. **RecordView** → `src/model/views/record_view.rs`
2. **HistogramView** → `src/model/views/histogram_view.rs`
3. **File loading logic** → `src/model/loader.rs`
4. **Search operations** → `src/model/search.rs`
5. **Type definitions** → `src/model/types.rs`

Each extraction follows the same process:
1. Create the new file
2. Copy the struct/impl
3. Add necessary imports
4. Update `mod.rs` with re-exports
5. Remove from original file
6. Test compilation

## Common Pitfalls to Avoid

❌ **Don't** make fields `pub` unless necessary - use `pub(crate)` instead  
❌ **Don't** forget to re-export types in `mod.rs`  
❌ **Don't** change the module's public API during refactoring  
✅ **Do** add documentation as you extract code  
✅ **Do** test after each extraction  
✅ **Do** keep related functionality together  

## Advanced: Circular Dependencies

If you encounter circular dependency errors:

```rust
// In model.rs
use crate::model::views::TableView;
// Error: circular dependency!
```

**Solution:** Move shared types to a separate module:
```
src/model/
├── mod.rs
├── types.rs       ← Shared types (Column, ColumnView, etc.)
├── views/
│   └── table_view.rs
└── ...
```

Then both can import from `types.rs` without circular dependency.
