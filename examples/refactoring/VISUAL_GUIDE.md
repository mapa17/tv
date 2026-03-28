# Visual Guide: Before and After Refactoring

## File Structure Comparison

### BEFORE (Current State)
```
src/
└── model.rs (1,621 lines)
    Contains everything:
    ├── FileType enum
    ├── Status enum  
    ├── FileInfo struct
    ├── Column struct
    ├── ColumnView struct ← Used by TableView
    ├── ColumnStatus enum
    ├── Modus enum
    ├── TableView struct (57 lines) ← TO BE EXTRACTED
    │   ├── 17 fields
    │   └── impl with 2 methods:
    │       ├── empty()
    │       └── build_index()
    ├── RecordView struct
    ├── HistogramView struct
    ├── UIData struct
    ├── UILayout struct
    └── Model struct (huge with 50+ methods)
```

### AFTER (Refactored State)
```
src/
├── model.rs (1,564 lines) ← 57 lines removed
│   Contains:
│   ├── mod views; ← NEW: Module declaration
│   ├── pub use views::TableView; ← NEW: Re-export
│   ├── FileType enum
│   ├── Status enum
│   ├── FileInfo struct
│   ├── Column struct
│   ├── ColumnView struct ← Stays here (used by multiple modules)
│   ├── ColumnStatus enum
│   ├── Modus enum
│   ├── RecordView struct
│   ├── HistogramView struct
│   ├── UIData struct
│   ├── UILayout struct
│   └── Model struct
│
└── model/ ← NEW: Submodule directory
    └── views/ ← NEW: Views submodule
        ├── mod.rs (4 lines)
        │   ├── pub mod table_view;
        │   └── pub use table_view::TableView;
        │
        └── table_view.rs (75 lines) ← NEW: Extracted code
            ├── use std::collections::HashMap;
            ├── use std::sync::Arc;
            ├── use super::ColumnView; ← Import from parent
            ├── TableView struct
            └── impl TableView
                ├── empty()
                └── build_index()
```

## Code Changes Detailed

### Change 1: model.rs - Add module declaration (TOP of file)

```diff
+ // Module declarations
+ mod views;
+ pub use views::TableView;
+
  use arboard::Clipboard;
  use polars::prelude::*;
  // ... rest of imports
```

### Change 2: model.rs - Change ColumnView visibility

```diff
  impl ColumnView {
-     fn empty() -> Self {
+     pub(crate) fn empty() -> Self {
          ColumnView {
              name: "".to_string(),
              width: 0,
              data: Vec::new(),
          }
      }
  }
```

### Change 3: model.rs - Remove TableView (DELETE 57 lines)

```diff
- pub struct TableView {
-     name: String,
-     rows: Arc<Vec<usize>>,
-     visible_columns: Vec<usize>,
-     // ... 14 more fields
- }
- 
- impl TableView {
-     fn empty() -> Self {
-         TableView {
-             // ... initialization
-         }
-     }
- 
-     fn build_index(&mut self) {
-         // ... implementation
-     }
- }
+ 
+ // TableView has been moved to model/views/table_view.rs
```

### Change 4: Create NEW file model/views/table_view.rs

```rust
use std::collections::HashMap;
use std::sync::Arc;

use super::ColumnView; // Import from parent module (model.rs)

/// Represents the state and data for rendering a table view.
pub struct TableView {
    pub(crate) name: String,
    pub(crate) rows: Arc<Vec<usize>>,
    // ... fields (use pub(crate) for module access)
}

impl TableView {
    pub(crate) fn empty() -> Self { /* ... */ }
    pub(crate) fn build_index(&mut self) { /* ... */ }
}
```

### Change 5: Create NEW file model/views/mod.rs

```rust
pub mod table_view;

pub use table_view::TableView;
```

## Import Path Compatibility

The refactoring maintains **backward compatibility** - no changes needed in other files!

### Before Refactoring:
```rust
// In main.rs or other files
use crate::model::TableView; // Works ✓
```

### After Refactoring:
```rust
// In main.rs or other files  
use crate::model::TableView; // Still works ✓ (re-exported)

// Alternative (explicit path):
use crate::model::views::TableView; // Also works ✓
```

## Verification Steps

After making changes, verify with:

```bash
# 1. Check file structure
ls -la src/model/views/

# 2. Count lines removed from model.rs
wc -l src/model.rs  # Should be ~1,564 (was 1,621)

# 3. Compile to check for errors
cargo build

# 4. Run clippy
cargo clippy

# 5. Run tests
cargo test
```

## Benefits Achieved

✅ **Reduced model.rs size**: 1,621 → 1,564 lines (-57 lines)  
✅ **Better organization**: TableView in dedicated file  
✅ **Easier to find**: Clear module structure  
✅ **Easier to test**: Can test TableView independently  
✅ **No breaking changes**: Re-exports maintain API  
✅ **Added documentation**: Doc comments on extracted code  

## Next Extractions

Following the same pattern, extract:

1. **RecordView** (36 lines) → `model/views/record_view.rs`
2. **HistogramView** (36 lines) → `model/views/histogram_view.rs`
3. **Data loading** (200 lines) → `model/loader.rs`
4. **Search operations** (150 lines) → `model/search.rs`
5. **Type definitions** (250 lines) → `model/types.rs`

Each extraction follows these same 5 steps! 🚀
