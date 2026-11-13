# guinea-note

`npx guinea-note file.txt` runs **Guinea Note**, a Notepad++-style terminal
text editor written in Rust: tabs, Find/Replace with regular expressions, line
tools, bookmarks, session restore and saves that keep your bytes exactly as
they were.

```sh
npx guinea-note notes.md          # run once
npm install -g guinea-note        # then: guinea notes.md
```

This package is a small launcher. npm installs the native binary for your
platform as an optional dependency (`@guinea-note/win32-x64` or
`@guinea-note/linux-x64`); it is the same file as in the
[GitHub Release](https://github.com/cmm-cmm/GuineaNote/releases) archive
(see `SHA256SUMS`). Node.js is needed only for the launcher. Supported: Windows
x64 and Linux x64 (static, any distribution). macOS is not released.

Documentation, source and issues: https://github.com/cmm-cmm/GuineaNote
— MIT License.
