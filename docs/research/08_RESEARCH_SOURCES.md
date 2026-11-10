# 08 — Danh mục nguồn và độ tin cậy

Tất cả nguồn trong danh mục được truy cập hoặc thử truy cập ngày **10/11/2026, múi giờ Asia/Saigon (UTC+7)**. Các commit bên ngoài là snapshot đã đọc, không phải tuyên bố chúng là release stable mới nhất. Tài liệu không có version/commit được ghi **rolling**; không gán commit giả cho website.

## Quy ước chứng cứ

| Mức | Loại nguồn | Có thể kết luận | Không thể tự suy ra |
| --- | --- | --- | --- |
| A | Source commit cố định, manifest/lock và dữ liệu local | Cấu trúc/API/path và phép đo được ghi trong scope | Code đã chạy native trên mọi OS, không còn bug |
| B | API/manual/spec chính thức đọc được | Hợp đồng hoặc mô tả tính năng theo version đó | Latency thực tế của GuineaNote hoặc implementation bên ngoài |
| C | Bài kỹ thuật tác giả/README giới thiệu | Ý đồ thiết kế, lý do và claim được tác giả đưa ra | Benchmark độc lập, guarantee cho sản phẩm khác |
| D | Truy cập bị chặn/lỗi/không có nội dung đọc được | Chỉ trạng thái truy cập | Không dùng làm bằng chứng kiến trúc |
| Đề xuất | Phân tích của báo cáo | Phương án cần kiểm thử/đánh giá | Không coi đã triển khai hoặc đã đo |

GitHub sources được đọc qua connector GitHub bằng GET; website/API tài liệu qua browsing hoặc HTTP GET read-only. Không clone/cài/build editor tham khảo. Không cài dependency, sửa source hoặc chạy benchmark ứng dụng trong lần nghiên cứu này.

## Nguồn GuineaNote

Baseline Git: **`3186ddf49e181d14f465f29af1e8a6e7e4392383`**, branch `dev`. Source này có version workspace **0.1.0**, license MIT, MSRV **1.98**. Host chỉ đọc `rustc -V` và nhận `rustc 1.98.1 (48a229cea 2026-09-01)`. GitHub CLI xác nhận repository private; chưa có package publish theo tài liệu hiện có và registry probes nêu dưới.

| Nguồn local, mức A | Phạm vi đã dùng |
| --- | --- |
| [Cargo.toml](../../Cargo.toml), [Cargo.lock](../../Cargo.lock), [LICENSE](../../LICENSE) | Crates, dependency versions, package/bin, MSRV/license |
| [README](../../README.md), [spec](../../GUINEANOTE_TECHNICAL_SPEC.md) | Mục tiêu và usage; đối chiếu code để nhận ra thông tin lịch sử |
| [Architecture](../ARCHITECTURE.md), [Implementation status](../IMPLEMENTATION_STATUS.md) | Lịch sử phát triển, scope test/cross/native |
| [Core buffer](../../crates/guinea-core/src/buffer.rs) | Trait, owned slice, chunk APIs, Rope private |
| [Core document](../../crates/guinea-core/src/document.rs) | Concrete storage, Clone/history, state token, prepared/save acceptance, newline policy |
| [Core view](../../crates/guinea-core/src/view.rs), [graphemes](../../crates/guinea-core/src/graphemes.rs), [coordinates](../../crates/guinea-core/src/coordinates.rs) | Sparse layout, Unicode và tọa độ |
| [I/O](../../crates/guinea-io/src/lib.rs), [File safety](../FILE_SAFETY.md) | Open/save, fingerprints, BOM, permissions, failures và giới hạn |
| [TUI loop](../../crates/guinea-tui/src/lib.rs), [input](../../crates/guinea-tui/src/input.rs), [App](../../crates/guinea-tui/src/app.rs) | Event batching, input adapters, worker ownership và UI clones |
| [Renderer](../../crates/guinea-tui/src/workspace_render.rs), [render helpers](../../crates/guinea-tui/src/render.rs) | Viewport, clipped-window syntax, escaping, hit geometry |
| [Search](../../crates/guinea-tui/src/search.rs), [search guide](../SEARCH_REPLACE.md), [review](../../crates/guinea-tui/src/review.rs) | Worker caps/debounce, replace/review allocation và limits |
| [Syntax](../../crates/guinea-syntax/src/lib.rs), [TUI config](../../crates/guinea-tui/src/config.rs), [theme](../../crates/guinea-tui/src/theme.rs) | Language detection, basic lexer, color behavior |
| [Service live](../../crates/guinea-service/src/live.rs), [policy](../../crates/guinea-service/src/policy.rs), [service manifest](../../crates/guinea-service/Cargo.toml), [Agent manifest](../../crates/guinea-agent/Cargo.toml) | Protocol state boundary, permissions/leases, dependency graph và binary names |
| [CLI](../../src/main.rs), [microbenchmark harness](../../benches/editor.rs) | CLI hiện tại; harness 21 samples p50/p95 chưa đủ thay protocol p99 mới |
| [Performance](../PERFORMANCE.md), [validation manifest](../dialog-search-validation.json) | Evidence 09/11/2026; không remeasure trong nghiên cứu |
| [Outlier](../responsiveness-navigation-dialog-outlier.json), [final search](../responsiveness-dialog-search-final-copy.json) | Giữ 916.326 ms failure, completion tách khỏi feedback |
| [Test plan](../TEST_PLAN.md), [dependency audit](../DEPENDENCY_AUDIT.md), [behavior](../EDITOR_BEHAVIOR.md), [formats](../FORMATS.md) | Cổng kiểm thử, audit lịch sử và scope format |

Version lock liên quan được đọc: Ropey 1.6.1; Ratatui 0.30.2; Crossterm 0.29.0; unicode-segmentation 1.13.3; unicode-width 0.2.3; tempfile 3.27.0; Tokio 1.54.1; rmcp 3.9.0. Audit dependency ngày 09/11 là evidence cũ của repo, không phải audit security mới ngày 10/11. Báo cáo không chứng nhận trạng thái advisory hiện tại.

## Editor tham khảo: snapshot và file đã đọc

### Geany — mức A/C

- Snapshot **`0e02cc52a7634d1fed8511bd44730ea0ab791d9f`**, commit date **01/11/2026 UTC**; [metadata snapshot](https://api.github.com/repos/geany/geany/commits/0e02cc52a7634d1fed8511bd44730ea0ab791d9f).
- [README](https://github.com/geany/geany/blob/0e02cc52a7634d1fed8511bd44730ea0ab791d9f/README): GTK/Scintilla và định hướng nhẹ. Mức C cho các claim hiệu suất.
- [src/search.c](https://github.com/geany/geany/blob/0e02cc52a7634d1fed8511bd44730ea0ab791d9f/src/search.c), [src/document.c](https://github.com/geany/geany/blob/0e02cc52a7634d1fed8511bd44730ea0ab791d9f/src/document.c): phân chia dialog/document và scopes, mức A.
- Header hai file ghi GPL-2.0-or-later; chỉ đọc, không copy code.
- [Manual URL](https://www.geany.org/manual/current/index.html) không đọc được do robots; không dùng làm bằng chứng chi tiết.

### Kate và KTextEditor — mức A/B

- Kate snapshot **`d6977a183c3badc8c4ed8ef45c29a6db4770d006`**, commit date **09/11/2026 UTC**: [main window](https://github.com/KDE/kate/blob/d6977a183c3badc8c4ed8ef45c29a6db4770d006/apps/lib/katemainwindow.cpp).
- KTextEditor snapshot **`82325fbff097abea15c54c7ffcf2b3882fec79e5`**, commit date **08/11/2026 UTC**: [buffer](https://github.com/KDE/ktexteditor/blob/82325fbff097abea15c54c7ffcf2b3882fec79e5/src/buffer/katetextbuffer.cpp), [block](https://github.com/KDE/ktexteditor/blob/82325fbff097abea15c54c7ffcf2b3882fec79e5/src/buffer/katetextblock.h).
- [Handbook stable_kf6](https://docs.kde.org/stable_kf6/en/kate/kate/index.html), [Sessions](https://docs.kde.org/stable_kf6/en/kate/kate/sessions-menu.html), [LSP Client](https://docs.kde.org/stable_kf6/en/kate/kate/kate-application-plugin-lspclient.html), [plugins](https://docs.kde.org/stable_kf6/en/kate/kate/plugins.html): rolling stable channel, không pin cùng source commit.
- [Trang sản phẩm](https://apps.kde.org/kate/): capability ở mức giới thiệu, không benchmark.
- [LICENSES directory](https://github.com/KDE/kate/tree/d6977a183c3badc8c4ed8ef45c29a6db4770d006/LICENSES) có nhiều giấy phép. File main window đã xem LGPL-2.0-only; hai file KTextEditor đã xem LGPL-2.0-or-later. Không gán một license duy nhất cho mọi file KDE.
- URL handbook `stable5` LSP cũ trả 404; thay bằng stable_kf6 đọc được, không dựa vào bản cũ.

### gedit — mức B, source commit chưa xác minh

- [Website chính thức](https://gedit-text-editor.org/), [tính năng](https://gedit-text-editor.org/features.html), [Technology](https://gedit-text-editor.org/technology.html), [developer directory](https://gedit-text-editor.org/developer-docs/): rolling.
- [Reference Manual](https://gedit-text-editor.org/developer-docs/gedit/) hiển thị **gedit 51.0**, generated GTK-Doc 1.37.0.
- [GeditDocument](https://gedit-text-editor.org/developer-docs/gedit/GeditDocument.html), [GeditView](https://gedit-text-editor.org/developer-docs/gedit/GeditView.html), [GeditTab](https://gedit-text-editor.org/developer-docs/gedit/GeditTab.html): quan hệ object, public state/load/save APIs.
- Website dẫn source tới [World/gedit/gedit README](https://gitlab.gnome.org/World/gedit/gedit/-/blob/main/README.md). URL cũ [GNOME/gedit](https://gitlab.gnome.org/GNOME/gedit) lỗi 406; README mới chỉ có một dòng đọc được trong tool. Không lấy được source commit đáng tin trong phiên này.
- [GitHub organization](https://github.com/gedit-technology) truy cập được, nhưng probe repo tên gedit không cung cấp snapshot dùng được. Không suy ra editor engine hoặc license source cụ thể từ trang organization. Không nhầm GTK/Tepl/GtkSourceView public types với bằng chứng về cấu trúc lưu text nội bộ.

### Fresh — mức A/C

- Snapshot **`a1ce79df50b0642bdb6badd9dc9eb010c1e133cf`**, commit date **09/11/2026 UTC**.
- [Workspace Cargo.toml](https://github.com/sinelaw/fresh/blob/a1ce79df50b0642bdb6badd9dc9eb010c1e133cf/Cargo.toml): version **0.5.8**, license **GPL-3.0-or-later**.
- [Terminal editor manifest](https://github.com/sinelaw/fresh/blob/a1ce79df50b0642bdb6badd9dc9eb010c1e133cf/crates/fresh-editor/Cargo.toml): Ratatui/Crossterm runtime features.
- [Piece Tree](https://github.com/sinelaw/fresh/blob/a1ce79df50b0642bdb6badd9dc9eb010c1e133cf/crates/fresh-editor-core/src/model/piece_tree.rs), [buffer model](https://github.com/sinelaw/fresh/blob/a1ce79df50b0642bdb6badd9dc9eb010c1e133cf/crates/fresh-editor-core/src/model/buffer/mod.rs): loaded/unloaded data, shared bytes, optional index, mức A.
- [README](https://github.com/sinelaw/fresh/blob/a1ce79df50b0642bdb6badd9dc9eb010c1e133cf/README.md): UX và hiệu suất quảng bá, mức C.
- [How Fresh Loads Huge Files Fast](https://noamlewis.com/blog/2025/12/09/how-fresh-loads-huge-files-fast), Noam Lewis, **09/12/2025**: ý đồ lazy Piece Tree, approximate large-file navigation, mức C. Không dùng ngưỡng và claim tốc độ lịch sử làm kết quả đo commit hiện tại.

### GNU Nano — mức B

- [Trang chủ](https://www.nano-editor.org/) hiển thị **9.3**, ngày cập nhật **21/10/2026** tại thời điểm đọc.
- [Manual dòng v9](https://www.nano-editor.org/dist/v9/nano.html), [manual latest](https://www.nano-editor.org/dist/latest/nano.html): cả hai nội dung đọc được ghi **9.3**. Nghiên cứu UX modeless, help, mouse, search/replace, browser và options backup.
- Không pin source commit Nano, không thẩm định backend/complexity hoặc license từng file Nano. Không có benchmark Nano trong nghiên cứu. Probe README không cung cấp nội dung hữu dụng.

### Micro — mức A/C

- [URL cũ](https://github.com/zyedidia/micro) redirect sang [micro-editor/micro](https://github.com/micro-editor/micro); connector REST cũ trả moved response, sau đó dùng repo mới.
- Snapshot **`ab37da303ae719a469236697239a00e3e1ca8b16`**, commit date **08/11/2026 UTC**; không gán release tag chưa kiểm tra.
- [LineArray](https://github.com/micro-editor/micro/blob/ab37da303ae719a469236697239a00e3e1ca8b16/internal/buffer/line_array.go), [TextEvent/history](https://github.com/micro-editor/micro/blob/ab37da303ae719a469236697239a00e3e1ca8b16/internal/buffer/eventhandler.go), [terminal clipboard](https://github.com/micro-editor/micro/blob/ab37da303ae719a469236697239a00e3e1ca8b16/internal/clipboard/terminal.go): mức A.
- [Lua plugin docs](https://github.com/micro-editor/micro/blob/ab37da303ae719a469236697239a00e3e1ca8b16/runtime/help/plugins.md), [README](https://github.com/micro-editor/micro/blob/ab37da303ae719a469236697239a00e3e1ca8b16/README.md): use/config/features; claim dependency/static binary không thay kiểm tra linkage cụ thể.
- [LICENSE](https://github.com/micro-editor/micro/blob/ab37da303ae719a469236697239a00e3e1ca8b16/LICENSE) là MIT. Không port code trong nghiên cứu.

### Helix — mức A

- Snapshot **`ba40e547426b0f9896c8bdc699a4ab11f2b37dbc`**, commit date **29/09/2026 UTC**; không gọi là latest stable.
- [Core lib](https://github.com/helix-editor/helix/blob/ba40e547426b0f9896c8bdc699a4ab11f2b37dbc/helix-core/src/lib.rs): Ropey re-export.
- [Transaction](https://github.com/helix-editor/helix/blob/ba40e547426b0f9896c8bdc699a4ab11f2b37dbc/helix-core/src/transaction.rs), [document](https://github.com/helix-editor/helix/blob/ba40e547426b0f9896c8bdc699a4ab11f2b37dbc/helix-view/src/document.rs), [renderer](https://github.com/helix-editor/helix/blob/ba40e547426b0f9896c8bdc699a4ab11f2b37dbc/helix-term/src/ui/editor.rs): transaction mapping, view/render boundaries.
- [LICENSE](https://github.com/helix-editor/helix/blob/ba40e547426b0f9896c8bdc699a4ab11f2b37dbc/LICENSE) tại snapshot là **Mozilla Public License 2.0**. Không dùng thông tin license cũ để mô tả snapshot này; không relicense code thành MIT.
- Probe trang design master không đọc được; không viện dẫn nó làm evidence.

### Neovim — mức A

- Snapshot **`d175a05e0d24b2796f9624f8b6965d690394116e`**, commit date **09/11/2026 UTC**.
- [memline.c](https://github.com/neovim/neovim/blob/d175a05e0d24b2796f9624f8b6965d690394116e/src/nvim/memline.c): mô tả tree blocks, memfile backing và recovery.
- [event/loop.c](https://github.com/neovim/neovim/blob/d175a05e0d24b2796f9624f8b6965d690394116e/src/nvim/event/loop.c): libuv/queue orchestration.
- [api/ui_events.in.h](https://github.com/neovim/neovim/blob/d175a05e0d24b2796f9624f8b6965d690394116e/src/nvim/api/ui_events.in.h): grid/flush event contract. Probe `runtime/doc/ui.txt` không có content source dùng được; dùng file API thật.
- [LICENSE.txt](https://github.com/neovim/neovim/blob/d175a05e0d24b2796f9624f8b6965d690394116e/LICENSE.txt): Apache-2.0 trừ các phần đã đóng góp theo Vim license. Không gọi toàn bộ là MIT.
- Online API docs redirect sang trang không có readable lines trong tool; không dựa vào đó cho nội dung kỹ thuật.

## Data structures, rendering và runtime

| Nguồn chính thức | Version / ngày | Mức và phạm vi |
| --- | --- | --- |
| [Ropey crate](https://docs.rs/ropey/1.6.1/ropey/) | 1.6.1, khớp lock | B: UTF-8/scalar/chunks/line-break feature semantics |
| [Rope API](https://docs.rs/ropey/1.6.1/ropey/struct.Rope.html) | 1.6.1 | B: published complexity, clone, read/write contract; không là app benchmark |
| [RopeBuilder](https://docs.rs/ropey/1.6.1/ropey/struct.RopeBuilder.html) | 1.6.1 | B: incremental builder phục vụ streaming |
| [VS Code Text Buffer Reimplementation](https://code.visualstudio.com/blogs/2018/03/23/text-buffer-reimplementation) | Peng Lyu, 23/03/2018; VS Code 1.21 | C: tác giả mô tả Piece Tree/metadata và native boundary; không dùng benchmark/string limit cũ cho Node hiện tại |
| [Emacs Buffer Gap, source manual](https://github.com/emacs-mirror/emacs/blob/636f166cfc86aa90d63f592fd99f3fdd9ef95ebd/doc/lispref/buffers.texi) | Tag emacs-30.2 resolve commit `636f166cfc86aa90d63f592fd99f3fdd9ef95ebd`, 14/08/2025 UTC | A/B: gap di chuyển lúc edit; nguồn GNU qua mirror, đọc section Buffer Gap; không copy implementation |
| [Ratatui rendering](https://ratatui.rs/concepts/rendering/) | Rolling docs, dự án dùng 0.30.2 | B: immediate-mode model |
| [Ratatui under the hood](https://ratatui.rs/concepts/rendering/under-the-hood/) | Rolling docs | B: frame buffer/diff/flush; không tự suy dirty-widget skipping |
| [Crossterm event](https://docs.rs/crossterm/0.29.0/crossterm/event/) | 0.29.0 | B: input/poll/resize/mouse contract |
| [Crossterm clipboard](https://docs.rs/crossterm/0.29.0/crossterm/clipboard/) | 0.29.0 | B: CopyToClipboard API; không có nghĩa GuineaNote đã tích hợp |
| [Kitty keyboard protocol](https://sw.kovidgoyal.net/kitty/keyboard-protocol/) | Rolling specification | B: detection, progressive enhancement, ambiguity legacy keys |
| [tmux manual](https://man.archlinux.org/man/tmux.1.en) | Rolling upstream manual qua Arch man service | B: clipboard controls và terminal-feature policy; không chứng nhận mọi SSH setup |
| [NO_COLOR convention](https://no-color.org/) | Website last updated 03/11/2026 | B: disable color mặc định và explicit override; không đồng nhất với 256-color |
| [Rust ownership](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html) | Rolling Rust Book | B: ownership model; không chứng minh không có latency spikes |
| [Node memory](https://nodejs.org/learn/diagnostics/memory/understanding-and-tuning-memory) | Rolling Node Learn | B: heap/GC/external memory; không có engine benchmark A/B |
| [Node event-loop guidance](https://nodejs.org/learn/asynchronous-work/dont-block-the-event-loop) | Rolling Node Learn | B: event loop/worker considerations; không quy CPU work chỉ Node mới bị block |
| [Rust platform support](https://doc.rust-lang.org/rustc/platform-support.html) | Rolling rustc book | B: tiers/target triples, không là evidence GuineaNote native runtime |

## Unicode và bảo toàn dữ liệu

| Nguồn | Version / độ tin cậy | Cách dùng và giới hạn |
| --- | --- | --- |
| [UAX #29 bản cố định](https://www.unicode.org/reports/tr29/tr29-49.html), [trang đã đọc](https://www.unicode.org/reports/tr29/) | Unicode 18.0.0, revision 49, 01/09/2026; B normative | Grapheme segmentation; không tự xác nhận dependency hỗ trợ toàn bộ version này |
| [UAX #11 bản cố định](https://www.unicode.org/reports/tr11/tr11-46.html), [trang đã đọc](https://www.unicode.org/reports/tr11/) | Unicode 18.0.0, revision 46, 31/07/2026; B normative | East Asian Width; không đủ mô tả width mọi terminal grapheme |
| [tempfile NamedTempFile](https://docs.rs/tempfile/3.27.0/tempfile/struct.NamedTempFile.html) | 3.27.0, khớp lock; B | persist/persist_noclobber/security/durability caveats |
| [memmap2 MmapOptions](https://docs.rs/memmap2/latest/memmap2/struct.MmapOptions.html) | Trang latest đọc được hiển thị **0.9.12**; B | File-backed mapping safety. URL pin 0.9.12 bị tool từ chối, nên dẫn trang thực sự đọc; không đề xuất thêm dependency |
| [encoding_rs](https://docs.rs/encoding_rs/latest/encoding_rs/) | Trang latest đọc được hiển thị **0.8.44**; B | Encoding/error/streaming policy. URL pin 0.8.44 không đọc được trong tool; chưa chọn dependency |
| [Linux rename(2)](https://man7.org/linux/man-pages/man2/rename.2.html) | Rolling Linux man-pages; B | Same-filesystem rename, hardlink/handle và NFS caveats; không áp nguyên cho Windows |
| [Linux fsync(2)](https://man7.org/linux/man-pages/man2/fsync.2.html) | Rolling Linux man-pages; B | File và directory durability khác nhau |
| [Microsoft ReplaceFileW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilew) | Rolling Win32 reference; B | Replacement/share/metadata contract; không gán tự động cho tempfile wrapper |
| [Apple fsync manual archive](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/fsync.2.html) | Tài liệu archive, không version macOS hiện hành; B lịch sử | Phân biệt fsync/F_FULLFSYNC; cần xác minh APFS/API hiện đại trước guarantee |

Không dùng bài viết cộng đồng hoặc câu trả lời forum làm cơ sở cho kết luận kỹ thuật chính. Nguồn license là file LICENSE/header/manifest đã xem, không phải suy đoán từ ngôn ngữ hoặc tên sản phẩm. Báo cáo chỉ tham khảo thiết kế; không chuyển mã GPL/LGPL/MPL/Vim/Apache thành MIT.

## Phát hành và package managers

| Nguồn | Version / mức | Phạm vi |
| --- | --- | --- |
| [Cargo publishing](https://doc.rust-lang.org/cargo/reference/publishing.html) | Rolling Cargo Book; B | Publish permanence, dependency/package requirements |
| [cargo install](https://doc.rust-lang.org/cargo/commands/cargo-install.html) | Rolling Cargo Book; B | Package/bin distinction, exact version, `--locked` |
| [npm package.json](https://docs.npmjs.com/cli/v11/configuring-npm/package-json/) | npm CLI **v11** docs; B | bin, optionalDependencies, OS/CPU/libc; không chứng minh native shim prototype |
| [npm exec](https://docs.npmjs.com/cli/v11/commands/npm-exec/) | npm CLI **v11** docs; B | Explicit package/bin invocation; npm/npx không Node-free |
| [WinGet manifest](https://learn.microsoft.com/en-us/windows/package-manager/package/manifest) | Rolling Microsoft docs; B | Manifest submission/fields; chưa submit |
| [Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook) | Rolling; B | Formula/install/test/hash conventions, chưa có tap được tạo |
| [Guide for Debian Maintainers](https://www.debian.org/doc/manuals/debmake-doc/) | Rolling official guide; B | Packaging workflow, ưu tiên thay guide cũ |
| [Debian Reference ch02](https://www.debian.org/doc/manuals/debian-reference/ch02.en.html) | Rolling; B | Repo/package trust và quản lý package |
| [RPM Packaging Guide](https://rpm-packaging-guide.github.io/) | Rolling project guide; B | Spec, package/repository/signing workflow |
| [RPM Signatures and Digests](https://rpm.org/docs/latest/manual/signatures_digests.html) | Rolling rpm.org; B | Integrity/signature fields và version differences |
| [Nixpkgs Rust manual](https://nixos.org/manual/nixpkgs/stable/#rust) | Trang đọc hiển thị **26.05pre-git**, không tự đổi nhãn thành stable 26.05 | B cho helper/pinning model; chưa có build Nix/native verification |
| [GitHub runner selection](https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/choose-the-runner-for-a-job) | Rolling; B | Strategy chọn runner, chưa tạo workflow |
| [GitHub hosted runners](https://docs.github.com/en/actions/reference/runners/github-hosted-runners) | Rolling tại 10/11/2026; B | Available architecture/labels theo tài liệu; chưa kiểm tra entitlement của tài khoản |
| [GitHub artifact attestations](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations) | Rolling; B | Provenance và điều kiện plan/public/private, chưa bật feature |

### Registry probes read-only

Chi tiết kết quả và phạm vi suy luận tại [distribution](06_CROSS_PLATFORM_DISTRIBUTION.md). Các endpoint chính thức đã GET:

- [crates.io guineanote](https://crates.io/api/v1/crates/guineanote): 404; [guinote](https://crates.io/api/v1/crates/guinote): 404.
- [npm guineanote](https://registry.npmjs.org/guineanote): 404.
- [Homebrew guineanote](https://formulae.brew.sh/api/formula/guineanote.json), [guinote](https://formulae.brew.sh/api/formula/guinote.json): 404.
- [AUR RPC ba tên](https://aur.archlinux.org/rpc/v5/info?arg%5B%5D=guineanote&arg%5B%5D=guineanote-bin&arg%5B%5D=guinote): 200, zero results.
- [Debian package search](https://packages.debian.org/search?keywords=guineanote&searchon=names&suite=all&section=all): 200, không có kết quả.
- [WinGet candidate path](https://api.github.com/repos/microsoft/winget-pkgs/contents/manifests/c/cmm-cmm/GuineaNote): 404, không phải full-catalog search.

Mức A cho HTTP response thời điểm quan sát, không phải chứng nhận tên available/reserved/owned. Chưa kiểm tra Nix/Fedora catalogs và các tên internal Cargo/platform npm; phải làm lại trước publish. Không có thao tác đặt tên hoặc publish nào.

## Các lỗi truy cập và cách xử lý

| Nguồn / endpoint | Quan sát | Cách xử lý |
| --- | --- | --- |
| GNOME/gedit cũ, README GitLab mới | 406 hoặc không readable body | Dùng official developer docs 51.0; để trống source commit |
| Geany manual current | Robots deny | Dùng source pinned; không bịa nội dung manual |
| Kate stable5 LSP | 404 | Đọc stable_kf6 chính thức |
| Micro old REST repo | Moved Permanently | Resolve public repo mới rồi pin commit |
| Helix design page | Không đọc được | Dùng source core/transaction/renderer |
| Neovim online API/runtime doc path | Redirect rỗng hoặc file không có nội dung dùng được | Dùng source API event header |
| GNU Emacs Buffer Gap HTML | Timeout nhiều lần | Đọc source manual buffers.texi ở tag emacs-30.2, commit cố định |
| Xterm ctlseqs | 403 | Không dùng làm bằng chứng OSC 52; đọc Crossterm/tmux docs |
| AUR submission guidelines | Anubis access denied | Chỉ dùng RPC availability; submission policy còn phải đọc lại |
| GtkSourceView5, unicode-width pin, keyboard flags docs URL thử | Không đọc được | Không khẳng định implementation từ các endpoint đó |
| docs.rs memmap2/encoding_rs pin, tempfile 3.23 probe | Một số URL không truy cập được | Dùng latest đọc được với observed version; tempfile đúng lock 3.27 có pin đọc được |
| Debian New Maintainers' Guide | Trang ghi đã có guide thay thế | Dùng Guide for Debian Maintainers cho đề xuất mới |
| Debian signing manual path cũ | 404 | Dùng Debian Reference chính thức |

Không coi HTTP 200 của trang chống bot hoặc một body “Action completed” của wrapper là đã đọc source; khi connector trả structured content thì đọc content thực. Không dùng search snippets của Reddit/Wikipedia/forum làm bằng chứng kỹ thuật trong báo cáo.

## Những gì chưa có số liệu

Không có benchmark mới hoặc A/B Rust–TypeScript; không có native editor comparison; không có RSS guarantee cho 100 MB–1 GB, physical terminal paint hay power-loss guarantee. Các thresholds và acceptance protocol đề xuất nằm ở tài liệu 02–05, không trộn với measured evidence 09/11/2026. Chưa có dự toán thời gian hoặc quyết định publish/CI được thực thi.

Danh mục này là audit trail cho phạm vi đã nghiên cứu; các website rolling, runner availability, package namespaces và registry policies cần kiểm tra lại ở thời điểm triển khai/phát hành.
