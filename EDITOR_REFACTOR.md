# Editor Refactor Plan

## Decision

Do not split `EditorState` in one large rewrite.

The editor has real maintenance risk because one user action can update text, undo history, cursor position, parsing state, and layout caches together. The safe response is to protect those behaviors first, then improve one proven pain point at a time.

File size or field count alone is not a reason to refactor. Start this work only when an editor feature is being changed, a recurring defect identifies a weak boundary, or measurements show a performance problem.

## User Behavior To Protect

The editor must continue to preserve these observable behaviors:

- typing, deletion, selection, and cursor movement;
- undo and redo ordering;
- search, match navigation, and replacement;
- folding and displayed-line mapping;
- file loading, saving, and modified-state tracking;
- syntax parsing and rejection of stale asynchronous results;
- scrolling and rendering after edits.

Public APIs and legacy import paths must remain compatible throughout the work.

## Current Risk

`src/capabilities/editor/editor.rs` keeps most editor state and behavior in `EditorState`. Text mutation paths must coordinate several related updates. A missed update can produce user-visible failures such as incorrect undo results, stale highlighting, an outdated layout, or a misplaced cursor.

The in-file tests now cover parse revisions, stale parse results, UTF-8 cursor movement and selection (including word movement), stale search replacement ranges, stale asynchronous search results, stale fold-state reset after content replacement, undo/redo grouping for selection and multi-match replacement, and load/save modified-state transitions. They still do not provide a broad safety net for selection restoration or folding after line edits.

## Staged Approach

### 1. Add tests for the behavior being changed

Before restructuring a subsystem, add focused tests around its current behavior. Prioritize:

1. insert/delete followed by undo and redo;
2. cursor and selection results after edits;
3. search navigation and replacement;
4. folding behavior after line-changing edits;
5. load/save round trips and modified-state transitions.

Do not build a large test framework in advance. Add the smallest tests needed to protect the next change.

### 2. Centralize text-mutation invariants

The first useful code boundary should ensure that a committed text edit consistently updates the state that must move together:

- rope content;
- undo/redo history;
- cursor and selection;
- modified/content version state;
- parse revision or syntax-tree updates;
- layout and highlight invalidation.

Keep existing public methods as the user-facing operations. Consolidate only the internal steps that must always happen together. This reduces missed updates without forcing a new public API or a full document-model rewrite.

### 3. Extract only a subsystem with demonstrated value

After tests and mutation rules are stable, extract the subsystem that is actively causing defects or blocking planned work. Possible seams include search tasks, parsing coordination, interaction state, layout caches, or cursor blinking.

An extracted type should own a rule or lifecycle, not merely group fields. If moving fields does not make a failure harder to introduce or a change easier to test, leave them in `EditorState`.

### 4. Keep performance changes separate

Do not combine structural cleanup with changes such as partial layout-cache invalidation. Cache behavior affects rendering correctness and needs dedicated measurements and tests, especially for multi-line edits and folding.

## Explicit Non-Goals

- No big-bang `EditorDocument` rewrite.
- No field extraction solely to reduce the size of `EditorState`.
- No public API or compatibility-path changes.
- No rendering snapshot rewrite without a reproduced rendering problem.
- No cache-policy change hidden inside a structural refactor.
- No speculative abstraction for future editor engines.

## Completion Criteria For Each Step

A refactor step is complete when:

- the affected user behavior has focused regression coverage;
- public behavior and import paths remain unchanged;
- the new boundary owns a clear rule or lifecycle;
- `just fmt`, `just clippy`, and `just test` pass;
- the change can be reviewed and reverted independently.
