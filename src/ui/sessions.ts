import appIcon from "../../src-tauri/icons/icon.png";
import { method, profileName, type Dashboard, type Phase, type Profile, type Session, type Status } from "../domain";
import { date, escape, fact, header, notice } from "./html";

const statusText: Record<Status, string> = {
  unknown: "미확인",
  available: "사용 가능",
  needsRenewal: "갱신 필요",
};
const phaseText: Record<Phase, string> = {
  queued: "대기 중",
  signingIn: "로그인 중",
  checking: "확인 중",
};

function badge(session: Session): string {
  const text = session.phase ? phaseText[session.phase] : statusText[session.status];
  const appearance = session.phase ? "working" : session.status;
  return `<span class="badge ${appearance}"><span class="dot"></span>${text}</span>`;
}

function onboarding(hasCli: boolean, profiles: Profile[] | null): string {
  const hasIam = profiles?.some((profile) => profile.kind === "iamSource");
  const hasSso = profiles?.some((profile) => profile.kind === "sso");
  const firstRegistration = hasIam === false && !hasSso;
  const heading = hasCli ? "첫 AWS 프로필 연결" : "AWS CLI 연결";
  const description = !hasCli ? "AWS CLI v2를 연결하면 앱에서 AWS 프로필을 등록하고 연결할 수 있습니다."
    : firstRegistration ? "IAM 액세스 키를 등록하고 MFA 갱신에 사용할 프로필을 설정하세요."
    : "로컬 IAM 원본 프로필이나 조직의 Identity Center 프로필을 연결하세요.";
  const actions = hasCli ? `
    ${hasIam === false ? `<button class="button ${firstRegistration ? "primary" : ""}" data-action="start-iam">IAM 액세스 키 등록</button>` : ""}
    <button class="button ${firstRegistration ? "" : "primary"}" data-action="add">
      ${firstRegistration ? "인증 방식 선택" : "프로필 연결"}
    </button>` : '<button class="button primary" data-page="cli">AWS CLI 확인</button>';
  return `<section class="onboarding">
    <img src="${appIcon}" alt="" width="58" height="58" />
    <h2>${heading}</h2>
    <p>${description}</p>
    <div class="actions">${actions}</div>
  </section>`;
}

function sessionList(sessions: Session[], selected: Session): string {
  const rows = sessions.map((item) => `<li><button class="session-row" data-session="${escape(item.id)}"
    aria-pressed="${item.id === selected.id}">
    <span class="row-label"><strong>${escape(item.label)}</strong><span>${escape(profileName(item.login))}</span></span>
    ${badge(item)}
  </button></li>`).join("");
  return `<section class="card session-list" aria-label="저장한 프로필">
    <h2>프로필 <span>${sessions.length}</span></h2><ul>${rows}</ul>
  </section>`;
}

function sessionFacts(session: Session): string {
  const { login, identity } = session;
  const expiry = login.kind === "sso" ? "로그인 시점에 따라 다름" : date(session.expiresAt);
  return `<dl class="facts">
    ${fact("AWS 프로필", profileName(login), true)}
    ${login.kind === "iamMfa" ? fact("원본 프로필", login.source, true) : ""}
    ${fact("계정", identity?.Account ?? "미확인", true)}
    ${fact("마지막 확인", session.checkedAt ? date(session.checkedAt) : "미확인")}
    ${fact("만료 시각", expiry)}
    ${fact("IAM 사용자 / 역할 ARN", identity?.Arn ?? "미확인", true, true)}
  </dl>`;
}

function sessionActions(session: Session, hasCli: boolean, requiresMfa: boolean): string {
  const canStart = hasCli && !session.phase;
  return `<div class="actions detail-actions">
    <button class="button primary" data-action="renew" ${canStart && !requiresMfa ? "" : "disabled"}>갱신</button>
    <button class="button" data-action="check" ${canStart ? "" : "disabled"}>상태 확인</button>
    <button class="button subtle" data-workflow="connect" ${hasCli ? "" : "disabled"}>도구 연결</button>
    <button class="button subtle" data-workflow="share">공유</button>
    ${session.phase ? `<button class="button subtle" data-action="cancel" ${!session.canCancel ? "disabled" : ""}>
      ${session.canCancel ? "취소" : "취소 중"}</button>`
      : `<button class="button subtle" data-action="edit" ${hasCli ? "" : "disabled"}>편집</button>`}
  </div>`;
}

function sessionDetail(session: Session, cli: Dashboard["cli"]): string {
  const hasCli = Boolean(cli?.v2);
  const requiresMfa = session.login.kind === "iamMfa" && !cli?.mfaSupported;
  return `<section class="card detail" aria-label="선택한 세션">
    <div class="detail-top">
      <div><p class="method">${method(session.login)}</p><h2>${escape(session.label)}</h2></div>
      ${badge(session)}
    </div>
    ${sessionFacts(session)}
    ${session.task ? `<section class="task-note"><h3>작업 메모</h3><p>${escape(session.task)}</p></section>` : ""}
    ${session.checkMessage ? notice(session.checkMessage) : ""}
    ${session.activity ? notice(session.activity) : ""}
    ${requiresMfa && hasCli ? notice("IAM MFA 갱신을 사용하려면 AWS CLI를 업데이트해 주세요.", true) : ""}
    ${sessionActions(session, hasCli, requiresMfa)}
  </section>`;
}

export function sessionsPage(state: Dashboard, profiles: Profile[] | null, session?: Session): string {
  const hasCli = Boolean(state.cli?.v2);
  const add = session
    ? `<button class="button primary" data-action="add" ${hasCli ? "" : "disabled"}>프로필 추가</button>` : "";
  const banners = [state.settingsError, state.profilesError]
    .filter((message): message is string => Boolean(message))
    .map((message) => notice(message, true)).join("");
  const body = session
    ? `<div class="session-grid">${sessionList(state.sessions, session)}${sessionDetail(session, state.cli)}</div>`
    : onboarding(hasCli, profiles);
  const actions = `<div class="actions">${add}<button class="button" data-workflow="import">공유 파일 가져오기</button></div>`;
  return header("AWS 세션", "AWS 프로필의 로그인 상태를 확인하고 갱신하세요.", actions) + banners + body;
}
