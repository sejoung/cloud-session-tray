import type { Dashboard, Profile } from "../domain";
import { escape, fact, header, notice } from "./html";

export type InstallerState = "idle" | "downloading" | "opened";

type CliViewOptions = {
  profiles: Profile[] | null;
  cliBusy: boolean;
  installerState: InstallerState;
  wizardBusy: boolean;
};

const profileText: Record<Profile["kind"], string> = {
  iamSource: "IAM 원본",
  iamSession: "IAM 임시 세션",
  sso: "Identity Center",
  unconfigured: "인증 설정 없음",
  unsupported: "지원 범위 외",
};

function cliTitle(state: Dashboard): string {
  if (state.scanning) return "AWS CLI 찾는 중";
  if (!state.cli) return "AWS CLI를 찾을 수 없음";
  const version = escape(state.cli.version);
  return state.cli.v2 ? `AWS CLI v${version}` : `AWS CLI v${version} · 업데이트 필요`;
}

function installerNotices(state: Dashboard, installer: InstallerState): string {
  const messages: string[] = [];
  if (state.preferredPath && state.cli?.path !== state.preferredPath) {
    messages.push(notice("직접 선택한 AWS CLI 경로를 열 수 없어 자동 탐색을 사용했습니다.", true));
  }
  if (!state.scanning && !state.cli?.v2) {
    messages.push(notice("공식 설치 파일을 열거나 기존 AWS CLI v2 실행 파일을 선택하세요.", true));
  } else if (!state.scanning && !state.cli?.mfaSupported) {
    messages.push(notice("이 설치본은 Identity Center를 지원합니다. IAM MFA 갱신을 쓰려면 업데이트하세요.", true));
  }
  if (installer === "opened") {
    messages.push(notice("설치 창에서 설치를 진행하세요. 완료한 뒤 다시 탐색을 누르세요. 취소했거나 설치가 제한된 경우 실행 파일 선택과 공식 문서를 이용할 수 있습니다."));
  }
  return messages.join("");
}

function installerCard(state: Dashboard, { cliBusy, installerState }: CliViewOptions): string {
  const cli = state.cli;
  const downloading = installerState === "downloading";
  return `<section class="card section-card">
    <div class="card-heading"><div><h2>${cliTitle(state)}</h2>
    <p class="mono path">${escape(cli?.path ?? "실행 파일 경로 미확인")}</p></div>
    ${cli?.v2 ? '<span class="badge available"><span class="dot"></span>연결됨</span>' : ""}</div>
    ${installerNotices(state, installerState)}
    <div class="actions">
      <button class="button primary" data-action="install" ${downloading ? "disabled" : ""}>
      ${downloading ? "다운로드 중…" : cli ? "공식 업데이트 열기" : "공식 설치 파일 받기"}</button>
      <button class="button" data-action="rescan" ${cliBusy || state.scanning ? "disabled" : ""}>다시 탐색</button>
      <button class="button" data-action="choose" ${cliBusy ? "disabled" : ""}>실행 파일 선택</button>
      ${state.preferredPath ? '<button class="button subtle" data-action="auto-cli">자동 탐색 사용</button>' : ""}
      <button class="button subtle" data-docs="installation">설치 문서 ↗</button>
    </div>
  </section>`;
}

function profileRow(profile: Profile): string {
  const { name, kind, region, preferred, inConfig, inCredentials } = profile;
  const attributes = `data-profile="${escape(name)}"`;
  const locations = [inConfig && "config", inCredentials && "credentials"].filter(Boolean).join(" · ");
  const usable = kind === "iamSource" || kind === "iamSession" || kind === "sso";
  const attachable = kind === "sso" || profile.targetAllowed;
  return `<li class="profile-row">
    <span class="profile-name"><strong>${escape(name)}</strong>
      <small>${escape(locations || "파일 위치 미확인")}</small>
      ${region ? `<small>${escape(region)}</small>` : ""}
      ${preferred ? "<small>기본 프로필</small>" : ""}
    </span>
    <span class="profile-type">${profileText[kind]}</span>
    <div class="profile-actions">
      ${attachable ? `<button class="button subtle" data-workflow="adopt" ${attributes}>세션에 연결</button>` : ""}
      ${usable ? `<button class="button subtle" data-workflow="connect" ${attributes}>도구 연결</button>
        <button class="button subtle" data-workflow="share" ${attributes}>공유</button>` : ""}
    </div>
  </li>`;
}

function profileList(profiles: Profile[] | null): string {
  if (!profiles?.length) {
    return `<p class="muted">${profiles ? "읽을 수 있는 로컬 프로필이 없습니다." : "로컬 프로필을 불러오세요."}</p>`;
  }
  return `<ul class="catalog">${profiles.map(profileRow).join("")}</ul>`;
}

function profilesCard(state: Dashboard, { profiles, wizardBusy }: CliViewOptions): string {
  const wizardRunning = wizardBusy || state.wizard;
  const hasCli = state.cli?.v2;
  return `<section class="card section-card">
    <div class="card-heading">
      <h2>로컬 프로필</h2>
      <div class="actions">
        <button class="button" data-action="profiles" ${hasCli ? "" : "disabled"}>다시 읽기</button>
        <button class="button" data-workflow="import">공유 파일 가져오기</button>
      </div>
    </div>
    ${state.profilesError ? notice(state.profilesError, true) : ""}
    <dl class="facts paths">
      ${fact("AWS 설정 파일", state.files.config, true, true)}
      ${fact("AWS 자격 증명 파일", state.files.credentials, true, true)}
    </dl>
    ${profileList(profiles)}
    <div class="wizard-section">
      <h3>프로필 등록</h3>
      <p>IAM은 AWS 액세스 키를, Identity Center는 조직의 URL과 SSO 리전을 사용합니다.</p>
      <div class="actions">
        <button class="button" data-action="register-iam" ${hasCli ? "" : "disabled"}>IAM 프로필 등록</button>
        <button class="button" data-action="wizard" ${!hasCli || wizardRunning ? "disabled" : ""}>
        ${wizardRunning ? "마법사 실행 중…" : "SSO 마법사 열기"}</button>
        <button class="button subtle" data-docs="sso">설정 문서 ↗</button>
      </div>
    </div>
  </section>`;
}

export function cliPage(state: Dashboard, options: CliViewOptions): string {
  return header("AWS CLI", "실행 파일과 AWS 프로필 연결을 관리하세요.")
    + installerCard(state, options) + profilesCard(state, options);
}
