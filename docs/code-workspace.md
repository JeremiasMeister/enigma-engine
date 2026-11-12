# Project code workspace

Open your game project, then select **Code** in the top toolbar. Search or expand the file tree and click a file to open a tab. Rust files have basic syntax coloring and line numbers; other UTF-8 text files are editable too. Images have a visual preview. Other formats and large files show a preview explanation and can be opened externally.

## Open the Rust project in your IDE

Use **Open IDE** in the main toolbar, **File → Open Rust Project**, or **Open Rust project** in the Code workspace. This opens the loaded game's Cargo project folder. The Code workspace's editor menu offers Auto, VS Code, Cursor, Zed and RustRover. Auto tries these installed editors in that order. **Open file externally** opens the active tab's file; **Reveal folder** opens the project in the file manager.

On macOS, Enigma launches the installed application by name. On Linux and Windows the editor executable must be available on PATH; Windows also checks the usual per-user VS Code and Cursor installation paths. The editor choice lasts for the current Enigma session. Launch failures appear in the editor. Completion, refactoring, Rust diagnostics and debugging are provided by the external IDE.

## Edit and save

| Action | Shortcut in Code |
| --- | --- |
| Save active file | Cmd/Ctrl+S |
| Save all open changed files, including other projects | Cmd/Ctrl+Shift+S |
| Find in the current file | Cmd/Ctrl+F |

The text widget supports normal selection, clipboard operations and undo/redo. Use Previous/Next beside Find to select a match. Tabs show an asterisk when changed, and their header identifies the owning project and relative path. Tabs remain available when switching workspaces or projects.

**Build**, **Play** and **Update dependencies** save changed code belonging to the current project before starting. A failed save stops the operation. Saving a scene or project does not save open source files.

**Reload** reads the source from disk. Closing or reloading a changed tab offers Save, Discard or Cancel. If another tool changed the source, saving offers Overwrite disk, Reload disk or Cancel. The original bytes are checked before replacement, and the editor preserves file permissions, UTF-8 BOM and CRLF encoding. Mixed newline files are normalized to CRLF on an actual edit when they originally contained CRLF.

## Recover unsaved work

Editing writes a separate local recovery draft under `.enigma-editor/drafts`; source changes are written on Save or the build/play actions above. Reopen the same project and source file after restarting Enigma to recover that draft. A banner identifies recovered text. Save keeps it; Reload lets you explicitly discard it in favor of the disk version. If a file becomes read-only, a failed Save keeps the draft; restore write permission and save again.

Draft failures display a warning above the text. Save the file before closing if that warning appears. Recovery files are ignored by Git through a local `.enigma-editor/.gitignore`. They are tied to the original project location and source path; keep those paths intact until drafts are saved. Deleted source files must be restored before their drafts can be reopened. Enigma does not automatically reopen the previous tab list.

## File limits and managed content

- Editable text is limited to 1 MiB of serialized UTF-8 text, including newline encoding. Input exceeding the limit is truncated with a visible warning. Syntax coloring is disabled above 200,000 bytes.
- Up to 24 tabs can be open. The file tree is limited to 10,000 inspected entries and 16 directory levels; it skips build output, version-control data, logs and recovery files. Click Refresh after external file creation or removal.
- Images use the existing bounded decoder and GPU texture limits. Sprite sheets preview as whole images; use the UI workspace to edit sprite regions and styles.
- Linked files, special files and paths outside the project are not editable here.
- `enigma_project.json` and scene JSON under `src/resources/scenes` are read-only previews because Enigma owns their contents.
- Generated UI Rust is editable, with a warning that **Generate Rust** replaces it. Keep custom game behavior in your own source modules. Reopen or Reload after regenerating.
