# 06 — Phân phối đa nền tảng

Ngày: **10/11/2026**. Toàn bộ quy trình dưới đây là **đề xuất cho giai đoạn release**. Không tạo workflow, build artifact, commit, đổi visibility hay publish package trong đợt nghiên cứu. Quy tắc phát triển hiện tại vẫn là không có GitHub CI.

## Identity và khoảng cách hiện trạng

| Mục | Hiện tại đã xác minh | Đề xuất |
| --- | --- | --- |
| Sản phẩm | GuineaNote, MIT | Giữ |
| CLI | Cargo package mặc định và Clap là `guineanote` | Executable chính thức `guinote` / `guinote.exe` |
| Cargo package | `guineanote`, workspace version 0.1.0 | Có thể giữ package name; khai báo bin khác package khi triển khai |
| npm package | Chưa publish; tên dự kiến `guineanote` | Chỉ quyết định sau kiểm tra tên và launcher contract |
| Agent/MCP binaries | `guineanote-agent`, `guineanote-mcp` | Giữ tên hiện tại trước; rename chỉ có migration host config rõ |
| Repository | `cmm-cmm/GuineaNote`, **private**, đã kiểm tra bằng GitHub CLI | Cần chốt public source hoặc public distribution repo trước kênh công khai |
| Rust | Manifest MSRV 1.98; host `rustc 1.98.1` | Stable + MSRV tests; lock exact build toolchain cho release |

Rename binary ảnh hưởng README, scripts/fixtures, usage text, installer, shell completion và tên executable Agent có thể tham chiếu. Đề xuất một migration riêng; optional alias `guineanote` có thời hạn nếu cần tương thích người dùng hiện có. Không thay đổi code ở đây. Binary `guinote` trực tiếp không cần Rust hoặc Node lúc chạy, nhưng vẫn có OS/ABI/system-library requirements cần kiểm tra.

## Kiểm tra tên package, không xem là đặt chỗ

Các GET read-only thực hiện **10/11/2026 khoảng 00:44 UTC**; WinGet probe khoảng cuối phiên nghiên cứu. `404` hoặc zero-result chỉ chứng minh endpoint không có entry tại thời điểm đó, **không chứng minh có quyền đăng ký**, không loại trừ reserved/similar names, private registry, policy hoặc thay đổi sau đó.

| Namespace / tên truy vấn | Endpoint và kết quả | Kết luận được phép |
| --- | --- | --- |
| crates.io `guineanote` | [API crate](https://crates.io/api/v1/crates/guineanote), HTTP 404 “does not exist” | Chưa thấy crate công khai ở endpoint |
| crates.io `guinote` | [API crate](https://crates.io/api/v1/crates/guinote), HTTP 404 | Như trên; chưa chọn làm package name |
| npm `guineanote` | [registry](https://registry.npmjs.org/guineanote), HTTP 404 “Not found” | Chưa thấy package công khai; không giữ chỗ |
| Homebrew `guineanote`, `guinote` | [formula API](https://formulae.brew.sh/api/formula/guineanote.json), [API guinote](https://formulae.brew.sh/api/formula/guinote.json), cả hai 404 | Không có formula ở API đó; không bao phủ mọi tap |
| AUR `guineanote`, `guineanote-bin`, `guinote` | [RPC info](https://aur.archlinux.org/rpc/v5/info?arg%5B%5D=guineanote&arg%5B%5D=guineanote-bin&arg%5B%5D=guinote), HTTP 200, `resultcount: 0` | Không có ba entry ở AUR lúc kiểm tra |
| Debian exact name `guineanote` | [package search](https://packages.debian.org/search?keywords=guineanote&searchon=names&suite=all&section=all), HTTP 200, “your search gave no results” | Không thấy ở search scope all suites; không bao phủ repo bên thứ ba |
| WinGet `cmm-cmm.GuineaNote` candidate | [manifest path](https://api.github.com/repos/microsoft/winget-pkgs/contents/manifests/c/cmm-cmm/GuineaNote), 404 | Chỉ path đó chưa có; chưa tra toàn catalog/similar identifier |
| Nix attr và Fedora/RPM name | Chưa kiểm tra catalog đủ tin cậy | **Chưa xác minh**, không giả định tên trống |
| Cargo internal crates / npm platform packages | Chưa kiểm tra từng tên | Gate bắt buộc trước publish dependency chain |

Không đăng ký hoặc chiếm tên trong nghiên cứu. Trước release chạy lại GET, kiểm tra naming policy/ownership và tìm tương đồng thương hiệu. `guinote` trên PATH còn có thể xung đột một chương trình ngoài package catalogs.

## Ma trận build và mức hỗ trợ thực tế

Rust target support không chứng minh dependency GuineaNote hoặc native input trên target đó đã chạy đúng. Nguồn: [Rust target tiers](https://doc.rust-lang.org/rustc/platform-support.html).

| Target đề xuất | Artifact | Evidence trong repository hiện tại | Cổng phát hành |
| --- | --- | --- | --- |
| `x86_64-pc-windows-msvc` | zip, `guinote.exe` | Native release/tests/ConPTY đã có evidence | Clean install + Unicode save + terminal restore trên máy sạch |
| `aarch64-pc-windows-msvc` | zip ARM64 | Chưa native build/run đã chứng minh | Build dependencies và test ARM64 thật; không chỉ emulation |
| `x86_64-unknown-linux-gnu` | tar.gz, deb/rpm sau | Có cross-Clippy; chưa native runtime | Link/test/PTTY native; chốt glibc baseline và distro tối thiểu |
| `aarch64-unknown-linux-gnu` | tar.gz ARM64, deb/rpm sau | Chưa native evidence | ARM64 runner/máy thật, ABI và TTY tests |
| `x86_64-apple-darwin` | tar.gz Intel | Chưa native evidence | Build/test macOS Intel, deployment target và terminal tests |
| `aarch64-apple-darwin` | tar.gz Apple Silicon | Có cross-Clippy; chưa native runtime | Link/test native APFS, Unicode/clipboard/capability |

Linux musl có thể là biến thể sau; không gộp GNU/musl dưới nhãn “linux” rồi tải nhầm. Không hứa static binary khi chưa kiểm tra dynamic dependencies. macOS universal binary chỉ thêm khi hai slice cùng version được test; không bắt buộc first release. Không phát hành target failed với nhãn supported để đủ sáu ô.

## GitHub Releases: nguồn artifact chuẩn

Đề xuất release tag SemVer `vX.Y.Z` trỏ đúng commit đã kiểm tra; assets dạng `guineanote-X.Y.Z-TARGET.zip` hoặc `.tar.gz`, bên trong có `guinote`, LICENSE, third-party notices và hướng dẫn support. Một release manifest ghi version, commit, target triple, Rust toolchain, minimum OS/ABI, artifact SHA-256 và binary hash. Tên archive là sản phẩm, executable là `guinote`.

Public package managers không thể dùng URL asset private đòi token của maintainer. Cần public release/source policy; tuyệt đối không nhúng PAT vào npm, Homebrew hay installer. Chưa đổi visibility trong nghiên cứu.

Đề xuất `SHA256SUMS` + chữ ký/provenance xác thực identity của release. Checksum cùng một kênh bị xâm nhập chỉ phát hiện hỏng dữ liệu, chưa xác thực tác giả. Signature hoặc attestation phải xác minh repo/workflow identity và immutable source commit. GitHub artifact attestation phụ thuộc plan/visibility; private repo có điều kiện Enterprise Cloud theo tài liệu hiện tại, nên không mặc định dùng được. Nguồn: [artifact attestations](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations).

## Cargo / crates.io

Package `guineanote` có thể xuất bin `guinote`; lệnh dự kiến sau publish là `cargo install guineanote --locked --version X.Y.Z --bin guinote`. Đây là ví dụ tương lai, chưa chạy được từ registry hiện tại. Cargo install cần Rust ở bước build/install, không ở bước chạy binary. Nguồn: [cargo install](https://doc.rust-lang.org/cargo/commands/cargo-install.html).

Workspace đang có internal path dependencies; crates.io không lấy source tùy ý từ đường dẫn local. Trước publish cần version requirements hợp lệ, kiểm tra tên mọi internal crate, publish theo thứ tự dependency, rồi `cargo package`/verify từ registry. Có thể tách những crate không public API nhưng vẫn cần phục vụ binary distribution; không flatten source chỉ để né vấn đề. Root `guinea-service` workspace path hiện chưa có version như một số entry khác, là điểm cần kiểm tra trong packaging milestone.

Version đã publish thường không thể ghi đè; rollback là install bản cũ hoặc release bản sửa mới, không republish cùng số. Lockfile cần được đưa vào package phù hợp; `--locked` giúp dùng graph đã kiểm chứng, không có nghĩa binary reproducibility hoàn chỉnh. Nguồn: [Cargo publishing](https://doc.rust-lang.org/cargo/reference/publishing.html).

## npm / npx

**Ranh giới yêu cầu:** `guinote` native chạy độc lập Node. Bản thân npm/npx là công cụ trong hệ sinh thái Node; nếu quy tắc “không runtime ngoài” áp dụng cả launcher, không thể quảng cáo lệnh npx là Node-free. npm là kênh tiện dụng tùy chọn, không thay đường cài native.

Thiết kế cần prototype trước khi chốt:

1. Main package `guineanote`, bin public `guinote`, exact-version optional dependencies theo OS/CPU/libc; mỗi platform package chứa native binary đã kiểm chứng.
2. Không compiler/runtime editor TypeScript. Launcher JS tối thiểu, nếu dùng, chỉ chọn payload và spawn native với args/stdin/stdout/stderr nguyên vẹn, không shell; **nó vẫn cần Node khi gọi launcher**, phải ghi rõ.
3. Nếu cần không Node ngay cả khi chạy lệnh cài xong, khảo sát native bin/shim sau install; phải chứng minh npm bin linking hoạt động Windows/Unix, npx và `--ignore-scripts`. Chưa có prototype nên không khẳng định giải pháp này đáp ứng sẵn.
4. Ưu tiên payload trong package thay download tùy ý lúc launch. Dùng npm integrity cho tarball và release hash/provenance cho payload; không `curl | sh`, không fallback build ngầm, không tải bản khác version khi target thiếu.
5. Test Node launcher overhead riêng; terminal raw mode, Ctrl+C, resize, exit code và cleanup phải qua native smoke.

Lệnh npx tránh lệch package/bin dự kiến: `npx --package=guineanote@X.Y.Z guinote file.txt`. Chưa publish, không coi là hướng dẫn hiện hành. Nguồn: [npm package.json: bin, os/cpu/libc, optionalDependencies](https://docs.npmjs.com/cli/v11/configuring-npm/package-json/), [npm exec/npx](https://docs.npmjs.com/cli/v11/commands/npm-exec/).

Nếu yêu cầu tuyệt đối không Node cho mọi hình thức chạy, giữ npm ở trạng thái thử nghiệm/chưa hỗ trợ đến khi chứng minh native shim; không âm thầm nới yêu cầu.

## WinGet và Homebrew

**WinGet:** candidate identifier `cmm-cmm.GuineaNote`, command `guinote`; zip portable package có executable alias phù hợp, URL immutable/versioned, architecture đúng và `InstallerSha256`. MSI chỉ thêm nếu cần lifecycle cài đặt phức tạp hơn portable. Manifest cần validation và sandbox install/upgrade/uninstall; không tạo PR submission trong nghiên cứu. Nguồn: [manifest](https://learn.microsoft.com/en-us/windows/package-manager/package/manifest).

**Homebrew:** bắt đầu tap riêng để có vòng release kiểm soát; formula `guineanote` candidate cài `guinote`, source build hoặc bottles do tap quản lý cho target đã test. Cần SHA, license, install path và test `guinote --version`; thêm PTY/save test ngoài formula smoke. Xét điều kiện brew/core sau, không giả định tự được nhận. `brew pin` chỉ ghim bản đang cài, không tự khôi phục bản đã bị thay; rollback cần versioned formula/bottle đã giữ. Nguồn: [Formula Cookbook](https://docs.brew.sh/Formula-Cookbook).

## Debian APT, RPM, AUR và Nix

| Kênh | Đề xuất và test | Chi phí / giới hạn |
| --- | --- | --- |
| `.deb` | Package candidate `guineanote`, `/usr/bin/guinote`, copyright/man page, `amd64`/`arm64`, dependencies theo linkage thực | Cài `.deb` từ Releases khác với có APT repository |
| APT repo riêng | Metadata Packages/Release/InRelease ký, scoped keyring + Signed-By, giữ bản cũ | Vận hành signing/key rotation và repo; chưa đồng nghĩa vào Debian official |
| RPM | `.spec`, Version/Release, `x86_64`/`aarch64`, signed RPM + repository metadata, upgrade/uninstall tests | Distro policy và baseline riêng; không assume mọi RPM distro tương thích binary |
| AUR | Source package candidate `guineanote`, binary variant `guineanote-bin`; PKGBUILD/.SRCINFO, SHA-256, LICENSE, no install-time network bất ngờ | AUR là build recipe của cộng đồng; trang guideline bị chặn trong nghiên cứu, cần đọc lại trước submission |
| Nix | Overlay/flake ban đầu; `rustPlatform.buildRustPackage`, pinned source, cargoHash/vendor graph và checks | Không hứa đã có nixpkgs attr; cần build sandbox/offline và native support từng system |

Nguồn: [Guide for Debian Maintainers](https://www.debian.org/doc/manuals/debmake-doc/), [Debian package/repository security](https://www.debian.org/doc/manuals/debian-reference/ch02.en.html), [RPM Packaging Guide](https://rpm-packaging-guide.github.io/), [RPM signatures](https://rpm.org/docs/latest/manual/signatures_digests.html), [AUR RPC đã đọc](https://aur.archlinux.org/rpc/v5/info?arg%5B%5D=guineanote&arg%5B%5D=guineanote-bin&arg%5B%5D=guinote), [Nixpkgs Rust manual](https://nixos.org/manual/nixpkgs/stable/#rust).

Nix build dependency closure không phải runtime phụ thuộc Rust/Node. Cần kiểm tra ELF/Mach-O/PE linkage thực thay vì chỉ nhìn manifest. Không tạo deb/rpm/PKGBUILD/Nix code trong lần này.

## GitHub Actions ở giai đoạn release sau

Yêu cầu trước đây không tạo CI đang được giữ. Phần này thiết kế pipeline tương lai theo prompt nghiên cứu, không bật Actions:

1. `check`: fmt, strict Clippy, tests/property/fault tests, lock/license/advisory review; stable và MSRV; permissions tối thiểu.
2. `native-build-test`: sáu target theo support matrix; PTY/Unicode roundtrip/save failure/terminal cleanup trên OS thật; ARM chỉ phát hành khi đủ evidence.
3. `package-verify`: unpack artifacts trong môi trường sạch, version/command/path/architecture, smoke non-TTY/TTY, checksums và notices.
4. `stage-release`: tag/manifest/toolchain/commit đồng nhất, draft release, SBOM/provenance nếu hạ tầng cho phép; ký bằng identity được quản lý.
5. `publish`: promote verified assets; package channels lấy cùng manifest/version/hash; rollback metadata có kế hoạch. Không rebuild tùy tiện mỗi channel rồi coi binary là cùng artifact.

Pin action tới immutable commit, không chạy untrusted PR với publish secrets, tách build và publish permissions, dùng trusted publishing/OIDC nếu registry/account hỗ trợ. Tên runner/khả năng ARM thay đổi: tài liệu ngày nghiên cứu có các runner ARM, nhưng chưa xác minh quota/account của repo private; không hard-code giả định mọi nhãn dùng được. Nguồn: [chọn runner](https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/choose-the-runner-for-a-job), [runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).

## Version, update và rollback

Một upstream SemVer trong workspace và release manifest là nguồn chuẩn; Cargo, npm và binaries cùng `X.Y.Z`. Debian/RPM có packaging revision riêng, ví dụ upstream `0.2.0` → deb `0.2.0-1`, RPM Version `0.2.0`, Release `1`; ví dụ không phải version dự định đã chốt. Prerelease có mapping theo từng manager và channel riêng, không đưa beta vào stable tag ngầm.

Không thể publish atomically qua mọi registry. Dùng trạng thái `Staged / NativeReleased / ChannelPending / Complete`, kiểm tra exact version thay vì “latest”, cho retry idempotent ở channel chưa hoàn tất. Không overwrite release assets cùng tên/hash đã công bố. Nếu binary lỗi, phát hành bản fix mới và đánh dấu withdrawn; không xóa evidence.

| Kênh | Update | Rollback dự kiến |
| --- | --- | --- |
| Native Releases | Tải version mới, verify, thay executable khi app đã thoát | Cài archive version cũ đã verify |
| Cargo | Install exact version với `--locked` | Reinstall exact bản cũ, xử lý conflict binary có chủ ý |
| npm | Install version hoặc chuyển dist-tag sau verify | Install exact bản cũ; dist-tag chỉ là chỉ dẫn, không sửa code bản cũ |
| WinGet | Upgrade theo manifest | Cài version cũ nếu manifest/assets còn được cung cấp; không hứa auto-downgrade |
| Homebrew tap | Update formula/bottle | Versioned formula/bottle đã giữ; pin không thay restore |
| APT/RPM | Signed repo metadata và package version mới | Install retained version có policy downgrade; không assume mọi dependency cho phép |
| AUR | PKGBUILD version mới | Recipe cũ + source/hash hợp lệ, người dùng build có chủ ý |
| Nix | Update pinned revision/lock | Revert pin hoặc generation nếu dùng profile, không tự phục hồi nội dung file đã edit |

Mọi rollback phải giữ preferences, sessions và recovery drafts; schema version có migration/backups, không mặc định downgrade đọc schema mới được. Package manager rollback chỉ đổi chương trình, không hoàn tác file người dùng đã lưu.

## Thứ tự kênh đề xuất

1. Native Windows release có checksum/signature và support scope đúng; song song hoàn thiện runtime Linux/macOS.
2. GitHub Releases cho các target đã qua native gates; Cargo khi public source/dependency chain sẵn sàng.
3. Homebrew tap và WinGet, vì phù hợp native binary và số kênh bảo trì còn nhỏ.
4. deb/rpm trước; APT/RPM repository khi có nguồn lực quản lý keys/metadata. AUR/Nix sau khi recipes đã test.
5. npm/npx sau khi chốt Node launcher contract và prototype portable shim. Không ưu tiên hơn bảo toàn file hoặc native evidence.
