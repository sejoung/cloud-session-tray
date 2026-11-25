# Cloud Session Tray

English · [한국어](README.ko.md)

[![CI](https://github.com/sejoung/cloud-session-tray/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/sejoung/cloud-session-tray/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/sejoung/cloud-session-tray)](https://github.com/sejoung/cloud-session-tray/releases/latest)
[![Apache 2.0 license](https://img.shields.io/badge/license-Apache%202.0-blue)](LICENSE)

Cloud Session Tray lets you check and renew AWS CLI sessions from the macOS menu bar or Windows system tray. It supports IAM users with MFA and IAM Identity Center profiles. The application identifier is `io.github.sejoung.cst`.

## Installation

Download the macOS Universal DMG for Apple Silicon or Intel, or the Windows x64 NSIS installer. The macOS app is ad hoc signed and has no Apple notarization. The Windows installer is unsigned. [Latest release](https://github.com/sejoung/cloud-session-tray/releases/latest)

## Getting started

The app interface is in Korean. The labels below match what you see in the app.

1. Open **AWS CLI** to check the executable and version. Use **다시 탐색** (scan again) after installing the CLI or **실행 파일 선택** (select executable) for a custom install location.
2. Open **세션 → 프로필 추가** (Sessions → Add profile) and choose an authentication method.
   - **IAM MFA** starts with your preferred local source profile and suggests a separate destination for temporary credentials. Use **MFA 장치 찾기** (Find MFA devices), enter the device ARN or serial, or register an IAM source if you still need to add your long-term keys locally. Device lookup requires `iam:ListMFADevices`.
   - **IAM Identity Center** uses a local SSO profile and checks its existing login. To create one, open **SSO 마법사 열기** (Open SSO wizard) with your organization's start or issuer URL and SSO region.
3. Click **갱신** (Renew) and enter an MFA code or finish the SSO login in your browser. **상태 확인** (Check status) shows the account, caller ARN and last check time. Enable expiration notifications in **설정** (Settings) for IAM MFA sessions whose expiration is known.

IAM MFA requires AWS CLI v2 with `aws configure mfa-login`. The app detects whether that command is available. [Install AWS CLI](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html) · [IAM MFA login](https://docs.aws.amazon.com/cli/latest/reference/configure/mfa-login.html) · [Identity Center setup](https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-sso.html)

## Connect existing profiles and tools

- Under **AWS CLI → 로컬 프로필** (Local profiles), see which profiles are defined in `config` and `credentials`. Use **세션에 연결** (Link to a session) to renew the temporary profile your tools already use. For example, if your long-term key lives in `default` and your kubeconfig uses `tech`, choose `default` as the source and `tech` as the destination.
- The AWS CLI writes temporary keys and the session token to the destination profile in the shared credentials file. If that profile has no region, the app uses the initial destination region or the source region. The **AWS CLI** screen shows the effective file paths, including overrides from `AWS_CONFIG_FILE` and `AWS_SHARED_CREDENTIALS_FILE`.
- In **도구 연결** (Connect tools), copy the shell settings for Bash, Zsh, or PowerShell. Enter an EKS cluster name to generate an `aws eks update-kubeconfig` command that uses the same profile. An existing kubeconfig using that profile can reuse it after renewal. [AWS shared file format](https://docs.aws.amazon.com/sdkref/latest/guide/file-format.html) · [EKS kubeconfig](https://docs.aws.amazon.com/eks/latest/userguide/create-kubeconfig.html)

## Share a profile setup

Open **공유** (Share) to copy instructions or export a setup file with the destination profile and a task note. An IAM setup can include a source profile hint, region, and session duration. An Identity Center setup can include the sign-in URL, SSO region, account, and role. Shared files omit AWS keys, session tokens, and MFA codes.

Teammates can preview a setup with **공유 파일 가져오기** (Import setup). Each teammate selects their own local IAM source and MFA device or signs in through Identity Center.

## Development

Use Node.js 24, stable Rust, and the Tauri prerequisites for your operating system. [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

```sh
npm ci
npm run tauri dev
```

```sh
npm run lint
npm test
npm run build
npm run tauri build
```

The lint suite limits source, tests, and config files to 400 lines each. ESLint, dependency-cruiser, and Rust architecture tests check lint rules, dependency boundaries, and cycles. The suite also runs jscpd, rustfmt, and Clippy. Tests use temporary files and test backends in place of live AWS credentials.

Before distributing a build, smoke-test CLI discovery and installation, IAM renewal and invalid MFA codes, SSO login and cancellation, and tray actions and expiration notifications on each platform. For IAM renewal, verify that the configured destination profile is the one your tools use.

## Releasing

- **CI** runs on pull requests and pushes to `main`. It checks versions, lints, tests, and builds the frontend on macOS and Windows.
- **Release desktop installers** runs on `vMAJOR.MINOR.PATCH` tags. It validates the tag and publishes a GitHub Release after the macOS Universal DMG and Windows x64 installer both build successfully.

Commit your changes on `main`, then run one of the following in a terminal. Tauri reads the app version from `package.json`.

```sh
npm run release -- patch   # Backward-compatible fixes
npm run release -- minor   # Backward-compatible features
npm run release -- major   # Breaking changes
npm run release -- current # First release or a version already set
```

The command synchronizes the npm and Cargo versions, runs lint, tests, and the frontend build, commits a version bump, and creates a local tag. It shows the tag and commit, then waits for `y` before pushing `main` and the tag together. If you defer the push or the push fails, use `npm run release -- resume v0.1.1` with the prepared tag.

GitHub Actions keeps intermediate installer artifacts for three days. Published installers remain available on GitHub Releases. Retry transient failures in Actions and rerun the build jobs if their artifacts have expired. Use a new version for code changes or replacements for published installers.

## License

[Apache License 2.0](LICENSE)
