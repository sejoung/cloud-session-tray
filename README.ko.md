# Cloud Session Tray

[English](README.md) · 한국어

[![CI](https://github.com/sejoung/cloud-session-tray/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/sejoung/cloud-session-tray/actions/workflows/ci.yml)
[![최신 릴리즈](https://img.shields.io/github/v/release/sejoung/cloud-session-tray)](https://github.com/sejoung/cloud-session-tray/releases/latest)
[![누적 다운로드](https://img.shields.io/github/downloads/sejoung/cloud-session-tray/total?label=%EB%8B%A4%EC%9A%B4%EB%A1%9C%EB%93%9C)](https://github.com/sejoung/cloud-session-tray/releases/latest)
[![Apache 2.0 라이선스](https://img.shields.io/badge/license-Apache%202.0-blue)](LICENSE)

macOS 메뉴 막대와 Windows 시스템 트레이에서 AWS 프로필을 확인하고 갱신하는 데스크톱 앱입니다. 앱 식별자는 `io.github.sejoung.cst`입니다.

## 설치

- **macOS** — Apple Silicon(`arm64`) 전용입니다. DMG를 내려받으세요. 앱은 애드혹 서명을 사용하며 공증되지 않았습니다.
- **Windows** — x64를 지원합니다. NSIS 설치 파일을 내려받으세요. 설치 파일에는 코드 서명이 없습니다.

[다운로드 및 화면 매뉴얼](https://sejoung.github.io/cloud-session-tray/ko/) · [최신 릴리즈](https://github.com/sejoung/cloud-session-tray/releases/latest)

## 시작하기

앱은 시스템 언어를 따르며 영어와 한국어를 지원합니다. **설정**에서 표시 언어를 바꿀 수 있습니다.

1. **AWS CLI** 화면에서 실행 파일과 버전을 확인합니다. 설치 가능한 새 버전이 확인되면 **공식 업데이트 열기**가 표시됩니다. 자동 확인은 24시간마다 최대 한 번 진행되며 **업데이트 확인**으로 즉시 다시 확인할 수 있습니다. 설치 후에는 **다시 탐색**을 누르거나 **실행 파일 선택**으로 직접 연결합니다.
2. 세션 목록이 비어 있고 등록된 IAM 원본이 없다면 **세션 → IAM 액세스 키 등록**을 누릅니다. 기본 리전은 선택 사항이며 새 원본 프로필의 CLI 출력 형식은 JSON으로 저장됩니다. 등록 후 새 원본이 선택된 MFA 설정 화면이 열립니다. 기존 프로필은 **프로필 연결** 또는 **프로필 추가**에서 연결합니다.
   - **IAM MFA**는 기본 AWS CLI 원본 프로필과 기존 MFA 장치 설정을 채우고 별도의 갱신 대상 이름을 제안합니다. 장치 설정이 없으면 **MFA 장치 찾기**로 조회하거나 ARN 또는 시리얼을 입력할 수 있습니다. 여기서도 **IAM 원본 프로필 등록**을 열 수 있습니다.
   - **IAM Identity Center**는 기존 SSO 프로필을 선택합니다. 가져온 프로필의 인증 상태와 신원을 자동 확인합니다. 새 프로필은 조직의 시작 URL 또는 발급자 URL과 SSO 리전을 준비한 뒤 **SSO 마법사 열기**로 생성할 수 있습니다.
3. **갱신**을 눌러 OTP를 입력하거나 기본 브라우저에서 SSO 인증을 완료합니다. **상태 확인**에서 마지막 확인 시각과 계정, AWS 사용자 또는 역할 ARN을 볼 수 있습니다.
4. 필요하면 **설정 → 만료 알림**을 켭니다. 만료 시각을 확인한 IAM MFA 세션에 적용됩니다.

AWS CLI는 기본 AWS 파일이나 앱을 실행할 때 지정한 `AWS_CONFIG_FILE`과 `AWS_SHARED_CREDENTIALS_FILE`을 사용합니다. 실제 경로와 읽어 온 프로필 분류는 **AWS CLI** 화면에 표시됩니다. 앱 설정은 운영체제의 앱 설정 폴더에 저장됩니다. 프로필 등록과 로그인에 사용되는 자격 증명은 AWS CLI가 공유 파일과 캐시에 기록합니다. MFA 장치 조회에는 원본 IAM 사용자의 `iam:ListMFADevices` 권한이 필요합니다.

IAM MFA는 `aws configure mfa-login`을 지원하는 CLI v2가 필요합니다. 앱이 명령 지원 여부를 확인합니다. [AWS CLI 설치 안내](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html) · [IAM MFA 명령](https://docs.aws.amazon.com/cli/latest/reference/configure/mfa-login.html) · [Identity Center 설정](https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-sso.html)

## 기존 AWS 프로필 사용

- **AWS CLI → 로컬 프로필**에서 각 프로필이 `config`와 `credentials` 중 어디에 있는지 확인합니다. 이미 쓰는 임시 프로필은 **세션에 연결**을 눌러 갱신 대상으로 지정합니다. 장기 키가 `default`에 있고 `tech`를 임시 프로필로 쓴다면 `default`를 원본으로, `tech`를 갱신 대상으로 선택합니다.
- IAM MFA로 갱신하면 AWS CLI가 대상 프로필의 임시 키와 토큰을 `credentials`에 기록합니다. 대상에 리전이 비어 있을 때 CST가 초기 대상 리전이나 원본의 리전을 `config`에 기록합니다. [AWS 공유 파일](https://docs.aws.amazon.com/sdkref/latest/guide/file-format.html)

## 팀 공유

**세션**에서 프로필을 선택하고 **공유 파일 내보내기**를 누릅니다. 갱신 대상과 작업 메모를 확인한 뒤 안내를 복사하거나 JSON 파일을 저장합니다. IAM 공유에는 원본 프로필 힌트, 갱신 대상, 리전과 기간을 담을 수 있습니다. Identity Center 공유에는 로그인 URL, SSO 리전, 계정과 역할을 담습니다. AWS 액세스 키 ID, 시크릿 액세스 키, 세션 토큰, MFA 코드는 기기에 남습니다. 작업 메모도 공유되므로 민감한 정보는 입력하지 마세요.

팀원은 **공유 파일 가져오기**에서 설정을 먼저 확인합니다. IAM의 경우 본인의 장기 키가 들어 있는 로컬 원본과 MFA 장치를 선택합니다. Identity Center는 가져온 프로필로 각자 로그인합니다.

[공유 파일 내보내기 매뉴얼](https://sejoung.github.io/cloud-session-tray/ko/#share-export) · [공유 파일 가져오기 매뉴얼](https://sejoung.github.io/cloud-session-tray/ko/#share-import)

## 진단

**설정 → 진단 → 로그 폴더 열기**에서 영어 로그를 확인할 수 있습니다. 앱 시작, CLI 탐색, 프로필 설정, 세션 작업의 결과와 오류 코드를 기록합니다. 로그 파일은 1 MiB 단위로 교체됩니다.

## 개발 및 패키징

Node.js 24, Rust stable과 운영체제별 Tauri 개발 도구가 필요합니다. [Tauri 개발 환경](https://v2.tauri.app/start/prerequisites/)

```sh
npm ci
npm run tauri dev
```

```sh
npm run build
npm run lint
npm test
npm run tauri build
```

Playwright용 Chromium을 설치하면 브라우저 E2E를 실행하고 매뉴얼 화면을 다시 캡처할 수 있습니다. 캡처 명령은 기본 창 크기에 예시 데이터를 표시한 한영 화면을 `docs/assets`에 저장합니다. WebKit에서 대화상자 레이아웃을 확인하려면 `npx playwright install webkit`으로 설치하고 `npx playwright test --project=webkit-layout`을 실행합니다. 앱 화면과 시스템 트레이는 `locales/en.json` 및 `locales/ko.json`의 번역 키를 공유합니다.

```sh
npx playwright install chromium
npm run test:e2e
npm run docs:capture
```

## 릴리즈

- **CI**는 `main`에 푸시하거나 PR을 열 때 Ubuntu에서 버전 일치, 린트, 테스트, 프런트엔드 빌드를 확인합니다.
- **Release desktop installers**는 `vMAJOR.MINOR.PATCH` 태그를 푸시할 때 실행됩니다. 태그와 앱 버전을 대조하고 Apple Silicon용 macOS DMG와 Windows x64 설치 파일을 빌드합니다. `main`의 같은 커밋에서 CI가 통과하면 GitHub Release에 게시합니다. 릴리즈용 Rust 캐시를 준비하려면 `main`에서 이 워크플로를 수동으로 실행할 수 있습니다.

변경 사항을 모두 커밋한 `main`에서 다음 명령 중 하나를 실행합니다. 앱 버전은 `package.json`에서 관리하며 Tauri가 같은 값을 읽습니다.

```sh
npm run release:patch               # 호환되는 버그 수정
npm run release:minor               # 호환되는 새 기능
npm run release:major               # 호환성이 깨지는 변경
npm run release:current             # 현재 버전 배포
npm run release:resume -- v0.1.1    # 준비해 둔 태그의 푸시 재시도
```

`npm run release`로 명령 목록을 볼 수 있습니다. 릴리즈 명령은 npm과 Cargo의 버전을 맞춰 확인한 뒤, 버전 변경을 커밋하고 로컬 태그를 만듭니다. 표시된 태그와 커밋을 확인하고 `y`를 입력하면 `main`과 태그를 함께 푸시합니다. GitHub CI는 설치 파일을 만드는 동안 린트·테스트·프런트엔드 빌드를 실행합니다. 푸시를 미뤘거나 실패했다면 `release:resume`에 준비한 태그를 전달하세요.

GitHub Actions의 중간 설치 파일은 3일간 보관합니다. 게시한 설치 파일은 GitHub Releases에서 계속 받을 수 있습니다. 일시적인 오류는 Actions에서 해당 작업을 다시 실행하고, 아티팩트가 만료되면 빌드부터 다시 실행합니다. 소스를 수정하거나 게시한 설치 파일을 교체할 때는 새 버전을 발행합니다.

## 코드 검사

- 소스, 테스트와 설정 파일은 줄바꿈과 주석을 포함해 파일당 400줄까지 허용합니다. 잠금 파일과 생성물은 검사 대상에서 제외합니다.
- TypeScript는 ESLint와 dependency-cruiser로 검사합니다. 화면과 도메인 모델은 Tauri에 직접 접근하지 않고 애플리케이션은 백엔드 모듈을 통해 호출합니다.
- Rust는 rustfmt, Clippy, 소스 의존성 테스트로 검사합니다. 데스크톱 어댑터에서 앱 상태를 거쳐 AWS CLI와 프로필 계층으로 의존성이 흐릅니다. Rust와 TypeScript 모두 역방향 의존성과 순환 참조를 검사합니다.
- jscpd는 프로덕션 코드에서 10줄과 80토큰 이상의 중복 블록을 검사합니다. 기능 테스트는 AWS 대신 임시 파일과 테스트용 CLI 또는 백엔드를 사용합니다.

## 운영체제별 확인 항목

- AWS CLI가 없는 PC에서 공식 설치 창 열기, 설치 취소 후 재시도, 설치 직후 경로 탐색
- IAM MFA 갱신과 잘못된 OTP 처리, 대상 프로필만 갱신되는지 확인, 앱 재시작 후 만료 시각 확인
- 브라우저 SSO 로그인과 취소 후 재시도, 같은 SSO 세션을 쓰는 두 프로필의 계정 및 역할 확인
- 메뉴 막대 또는 트레이에서 갱신과 상태 확인, 만료 전 운영체제 알림

## 라이선스

[Apache License 2.0](LICENSE)
