# Cloud Session Tray

macOS 메뉴 막대와 Windows 시스템 트레이에서 AWS 프로필을 확인하고 갱신하는 데스크톱 앱입니다. 앱 식별자는 `io.github.sejoung.cst`입니다.

## 시작하기

1. **AWS CLI** 화면에서 실행 파일과 버전을 확인합니다. 새로 설치했다면 **다시 탐색**을 누릅니다. 설치 폴더를 직접 지정했으면 **실행 파일 선택**으로 연결할 수 있습니다.
2. **세션 → 프로필 추가**에서 인증 방식을 고릅니다.
   - **IAM MFA**는 기본 AWS CLI 원본 프로필과 기존 MFA 장치 설정을 채우고 별도의 갱신 대상 이름을 제안합니다. 장치 설정이 없으면 **MFA 장치 찾기**로 조회하거나 ARN 또는 시리얼을 입력할 수 있습니다. 원본 키가 아직 등록되지 않았다면 **IAM 원본 프로필 등록**에서 키와 선택 사항인 리전을 입력합니다.
   - **IAM Identity Center**는 기존 SSO 프로필을 선택합니다. 가져온 프로필의 인증 상태와 신원을 자동 확인합니다. 새 프로필은 조직의 시작 URL 또는 발급자 URL과 SSO 리전을 준비한 뒤 **SSO 마법사 열기**로 생성할 수 있습니다.
3. **갱신**을 눌러 OTP를 입력하거나 기본 브라우저에서 SSO 인증을 완료합니다. **상태 확인**에서 마지막 확인 시각과 계정, AWS 사용자 또는 역할 ARN을 볼 수 있습니다.
4. 필요하면 **설정 → 만료 알림**을 켭니다. 만료 시각을 확인한 IAM MFA 세션에 적용됩니다.

AWS CLI는 기본 AWS 파일이나 앱을 실행할 때 지정한 `AWS_CONFIG_FILE`과 `AWS_SHARED_CREDENTIALS_FILE`을 사용합니다. 실제 경로와 읽어 온 프로필 분류는 **AWS CLI** 화면에 표시됩니다. 앱 설정은 운영체제의 앱 설정 폴더에 저장됩니다. 프로필 등록과 로그인에 사용되는 자격 증명은 AWS CLI가 공유 파일과 캐시에 기록합니다. MFA 장치 조회에는 원본 IAM 사용자의 `iam:ListMFADevices` 권한이 필요합니다.

IAM MFA는 `aws configure mfa-login`을 지원하는 CLI v2가 필요합니다. 앱이 명령 지원 여부를 확인합니다. 양쪽 OS에서 로그인 검증을 마친 뒤 공통 최소 버전을 정할 수 있습니다. [AWS CLI 설치 안내](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html) · [IAM MFA 명령](https://docs.aws.amazon.com/cli/latest/reference/configure/mfa-login.html) · [Identity Center 설정](https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-sso.html)

## 기존 프로필과 도구 연결

- **AWS CLI → 로컬 프로필**에서 각 프로필이 `config`와 `credentials` 중 어디에 있는지 확인합니다. 이미 도구에서 쓰는 임시 프로필은 **세션에 연결**을 눌러 갱신 대상으로 지정합니다. 장기 키가 `default`에 있다면 `default`를 원본으로 선택하고, 갱신 대상에는 도구에서 사용하는 이름을 입력합니다.
- IAM MFA로 갱신하면 AWS CLI가 대상 프로필의 임시 키와 토큰을 `credentials`에 기록합니다. 대상에 리전이 비어 있을 때 CST가 초기 대상 리전이나 원본의 리전을 `config`에 기록합니다.
- **도구 연결**에서 실제 프로필명과 파일 경로를 확인하고 bash/zsh 또는 PowerShell 설정을 복사할 수 있습니다. EKS 클러스터를 입력하면 동일한 프로필을 사용하는 `update-kubeconfig` 명령이 표시됩니다. 기존 kubeconfig가 이미 그 프로필을 사용한다면 세션 갱신 후 다시 조회하면 됩니다. [AWS 공유 파일](https://docs.aws.amazon.com/sdkref/latest/guide/file-format.html) · [EKS kubeconfig](https://docs.aws.amazon.com/eks/latest/userguide/create-kubeconfig.html)

## 팀 공유

**공유**에서 갱신 대상과 작업 메모를 확인한 뒤 **안내 복사** 또는 **공유 파일 내보내기**를 선택합니다. IAM 공유에는 원본 프로필 힌트, 갱신 대상, 리전과 기간을 담을 수 있습니다. Identity Center 공유에는 로그인 URL, SSO 리전, 계정과 역할을 담습니다. 자격 증명과 OTP는 공유 파일에 포함되지 않습니다.

팀원은 **공유 파일 가져오기**에서 설정을 먼저 확인합니다. IAM의 경우 본인의 장기 키가 들어 있는 로컬 원본과 MFA 장치를 선택합니다. Identity Center는 가져온 프로필로 각자 로그인합니다.

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

GitHub Actions의 **Desktop builds**가 macOS Universal DMG와 Windows x64 설치 파일을 실행 결과에 첨부합니다. 인증서와 공증 설정을 추가하면 배포용 서명 빌드를 만들 수 있습니다.

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
