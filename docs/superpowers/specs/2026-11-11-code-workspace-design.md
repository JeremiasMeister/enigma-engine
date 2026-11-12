# Project code workspace

The user requested easy external opening of the Rust project, plus file preview and editing inside Enigma. Extend the existing native editor, using Rust/egui and no engine changes or new dependencies.

## Behavior

- Add a Code workspace beside 3D scene and UI, with a searchable project file tree, document tabs, Rust syntax coloring, line numbers, text editing, image previews, Save/Save All/Reload and external-editor actions.
- Open Rust Project opens the loaded Cargo project folder in an auto-detected or selected VS Code, Cursor, Zed or RustRover installation. Reveal Folder is a separate file-manager action. Launch arguments never pass through a shell.
- File tabs identify their project and path. Unsaved buffers survive workspace and project switches. Closing/reloading a dirty document asks Save, Discard or Cancel. Code workspace Cmd/Ctrl+S saves the active file; Build/Play saves changed code for the current project before starting Cargo and stops on a failed/conflicting save.
- Preserve UTF-8 BOM and CRLF when saving. Compare the original disk bytes before replacing a file; external changes require an explicit overwrite or reload. Use a sibling temporary file and preserve existing permissions. Refuse symlink/path escapes and special files.
- Persist dirty text drafts under the owning project's `.enigma-editor/drafts` so closing the application through the engine cannot silently lose edits. Recover a draft when reopening its source file; conflicting external disk content stays protected. Source changes are written on Save or the build/play actions above. Draft write failures remain visible.
- Browse within the loaded project; skip build/version-control/log/recovery directories. Text editing is bounded to 1 MiB per file. Images use the existing bounded decoder. Other files show metadata and an external-open action. Project/scene metadata managed by Enigma is preview-only; generated Rust is editable with a regeneration notice.
- This is a lightweight editor, not a language server or debugger. Completion, diagnostics, refactoring and advanced navigation remain available through the external IDE.

These implementation details are assistant choices within the requested workflow. Existing UI-editor work and user changes remain intact; no commit or push is requested.
