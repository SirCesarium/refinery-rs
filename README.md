# 🦀🏭 Refinery-RS

> Refining Rust into universal artifacts: The **industrial-grade** production plant for exporting **Rust** to almost every machine on Earth.

## Features

- **Refinery CLI**: Interactive tool to initialize workflows and check for updates.
- **Refinery-RS Release**: Multi-target builds for Windows, Linux (GNU/MUSL), and macOS (Intel/Silicon).
- **Refinery-RS CI**: Integrated code quality gate with [`sweet`](https://github.com/SirCesarium/sweet) (swt), `clippy`, and `rustfmt`.
- **Docker Ready**: Automatic binary-to-container pipeline for GitHub Packages (GHCR).

---

## 🦀 Refinery CLI

Refinery-RS includes a powerful CLI to manage your workflows surgicaly.

### Installation
```bash
cargo install refinery-rs
```

### Commands
- **`refinery init`**: Interactive workflow generator. Supports multiple binaries, custom features, and surgical target selection.
- **`refinery check`**: Verify if you are using the latest version of Refinery on crates.io.

---

## 🛠️ Manual Integration

If you prefer not to use the CLI, you can manually create your workflow files in `.github/workflows/`.

### 1. Manual CI (`ci.yml`)
Create `.github/workflows/ci.yml` and add:

```yaml
name: CI

on: [push, pull_request]

jobs:
  quality-gate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6.0.2
      - uses: sircesarium/refinery-rs/ci@v2.0
        with:
          enable-sweet: true
          enable-clippy: true
          enable-fmt: true
```

### 2. Manual Release (`release.yml`)
Create `.github/workflows/release.yml` and add:

```yaml
name: Release

on:
  push:
    tags: ['v*']

jobs:
  build:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
          - target: x86_64-pc-windows-msvc
            os: windows-latest
    steps:
      - uses: actions/checkout@v6.0.2
      - uses: sircesarium/refinery-rs@v2.0
        with:
          target: ${{ matrix.target }}
          binary-name: {your_bin_name}
          github-token: ${{ secrets.GITHUB_TOKEN }}
```

---

## 🚀 CI Workflow (Quality Gate)

```yaml
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6.0.2
      - uses: sircesarium/refinery-rs/ci@v2.0
        with:
          enable-sweet: true
          enable-fmt: true
          enable-clippy: true
```

### CI Inputs

| Input             | Description                                      | Default                      |
| ----------------- | ------------------------------------------------ | ---------------------------- |
| `enable-sweet`    | Run `swt` maintainability analysis               | `true`                       |
| `enable-clippy`   | Run Clippy lints                                 | `true`                       |
| `enable-fmt`      | Check code formatting                            | `true`                       |
| `sweet-threshold` | Custom directory/file for `swt`                  | `.`                          |
| `clippy-args`     | Additional arguments for `cargo clippy`          | `--workspace -- -D warnings` |

---

## 📦 Release Workflow (Matrix Build)

```yaml
jobs:
  release:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
          - target: x86_64-pc-windows-msvc
            os: windows-latest
    steps:
      - uses: actions/checkout@v6.0.2
      - uses: sircesarium/refinery-rs@v2.0
        with:
          target: ${{ matrix.target }}
          publish-docker: true
          github-token: ${{ secrets.GITHUB_TOKEN }}
```

### Release & Docker Inputs

| Input               | Description                                      | Default                        |
| ------------------- | ------------------------------------------------ | ------------------------------ |
| `target`            | **Required**. The Rust target triple to build    | -                              |
| `package-name`      | Name of the package                              | `github.event.repository.name` |
| `binary-name`       | Name of the binary to export                     | `github.event.repository.name` |
| `features`          | Rust features to enable (or `all-features`)      | ""                             |
| `export-bins`       | Export binary executables                        | `true`                         |
| `export-libs`       | Export `.so`, `.dll`, `.dylib`, `.a`             | `true`                         |
| `use-cross`         | Use `cross-rs` for building                      | `false`                        |
| `package`           | Enable packaging (`.deb`, `.rpm`, `.msi`)        | `false`                        |
| `publish-docker`    | Build and push Docker image to GHCR              | `false`                        |
| `docker-image-name` | Custom name for the docker image                 | `github.repository`            |
| `github-token`      | Token for GHCR auth / GitHub Release             | -                   |

### 🔐 Secrets & Environment Variables

| Variable            | Description                                      | Usage                          |
| ------------------- | ------------------------------------------------ | ------------------------------ |
| `GITHUB_TOKEN`      | Automatically provided by GitHub                 | Auth for GHCR & Releases       |
| `CRATES_IO_TOKEN`   | Required if using the `publish-crates` job       | Auth for Crates.io             |

### 📂 Artifact Naming Convention

All exported artifacts follow a standardized surgical naming pattern: `{name}_{system}-{arch}{ext}`.

#### Binaries
- `{bin_name}_linux-x86_64`
- `{bin_name}_linux-arm64`
- `{bin_name}_linux-x86`
- `{bin_name}_windows-x86_64.exe`
- `{bin_name}_windows-arm64.exe`
- `{bin_name}_windows-x86.exe`
- `{bin_name}_macos-x86_64`
- `{bin_name}_macos-arm64`

#### Libraries
- `{lib_name}_linux-x86_64.so`
- `{lib_name}_linux-x86_64.a`
- `{lib_name}_linux-arm64.so`
- `{lib_name}_linux-arm64.a`
- `{lib_name}_linux-x86.so`
- `{lib_name}_linux-x86.a`
- `{lib_name}_windows-x86_64.dll`
- `{lib_name}_windows-arm64.dll`
- `{lib_name}_windows-x86.dll`
- `{lib_name}_macos-x86_64.dylib`
- `{lib_name}_macos-x86_64.a`
- `{lib_name}_macos-arm64.dylib`
- `{lib_name}_macos-arm64.a`

---

## 🍬 [Sweet](https://github.com/SirCesarium/sweet) Integration

The CI action automatically installs and runs `swt`. If `swt` finds maintenance risks, it will emit a GitHub Action warning:
`🍬 Sweet identified potential maintainability issues in your code!`

---

## 🐳 Docker Deployment

If `publish-docker` is enabled and no `Dockerfile` is found in the repository, Refinery-RS generates a surgical **Debian-slim** image:

- **Base**: `debian:bookworm-slim`
- **Path**: `/usr/local/bin/app`
- **Tags**: `latest` and `github.sha`

---

## License

MIT
