import { expect, test, type Locator, type Page } from "@playwright/test";
import { fileURLToPath } from "node:url";

const fixture = "/tests/e2e/fixture.html";

async function capture(page: Page, name: string, next?: Locator): Promise<void> {
  if (test.info().project.name !== "manual") return;
  await next?.evaluate((element) => element.setAttribute("data-manual-focus", ""));
  try {
    await page.screenshot({
      path: fileURLToPath(new URL(`../../docs/assets/${name}.png`, import.meta.url)),
      animations: "disabled",
      style: `:focus-visible { outline: none !important; }
        [data-manual-focus] { outline: 2px solid #4873ef !important; outline-offset: 3px !important; }`,
    });
  } finally {
    await next?.evaluate((element) => element.removeAttribute("data-manual-focus"));
  }
}

test("installs AWS CLI, registers an IAM profile, and renews the first session", async ({ page }) => {
  await page.goto(fixture);
  await page.getByRole("button", { name: "AWS CLI 확인" }).click();
  const installer = page.getByRole("button", { name: "공식 설치 파일 받기" });
  await expect(installer).toBeVisible();
  await capture(page, "cli", installer);
  await installer.click();
  await expect(page.getByText(/설치 창에서 설치를 진행하세요/)).toBeVisible();
  await page.getByRole("button", { name: "다시 탐색" }).click();
  await expect(page.getByText("최신 버전", { exact: true })).toBeVisible();
  await expect(installer).toHaveCount(0);

  await page.getByRole("button", { name: "세션", exact: true }).click();
  await page.getByRole("button", { name: "IAM 액세스 키 등록" }).click();
  const registration = page.getByRole("dialog", { name: "IAM 원본 프로필 등록" });
  const register = registration.getByRole("button", { name: "등록", exact: true });
  await registration.getByLabel("AWS 프로필 이름").fill("work-source");
  await registration.getByLabel("기본 리전 (선택)").selectOption("ap-northeast-2");
  await capture(page, "iam-register", register);
  await registration.getByLabel("액세스 키 ID").fill("AKIAIOSFODNN7EXAMPLE");
  await registration.getByLabel("비밀 액세스 키").fill("documentation-example-key");
  await register.click();

  const editor = page.getByRole("dialog", { name: "AWS 프로필 추가" });
  const findDevice = editor.getByRole("button", { name: "MFA 장치 찾기" });
  await expect(editor.getByRole("combobox", { name: "원본 프로필", exact: true })).toHaveValue("work-source");
  await expect(editor.getByLabel("갱신 대상 프로필")).toHaveValue("work-source-session");
  if (test.info().project.name === "manual") {
    const viewport = page.viewportSize()!;
    await page.setViewportSize({ width: viewport.width, height: 880 });
    await capture(page, "iam-mfa", findDevice);
    await page.setViewportSize(viewport);
  }
  await findDevice.click();
  await expect(editor.getByLabel("MFA 장치 ARN 또는 시리얼")).toHaveValue("arn:aws:iam::123456789012:mfa/example");
  await editor.getByLabel("표시 이름").fill("개발 환경");
  await editor.getByLabel("작업 메모 (선택)").fill("개발 계정의 AWS 리소스 확인");
  await editor.getByRole("button", { name: "저장" }).click();

  await page.getByRole("button", { name: "갱신", exact: true }).click();
  const renewal = page.getByRole("dialog", { name: "IAM MFA 갱신" });
  const otp = renewal.getByLabel("MFA 인증 코드");
  await expect(otp).toBeFocused();
  await capture(page, "renew", otp);
  await otp.fill("123456");
  await renewal.getByRole("button", { name: "갱신" }).click();
  const session = page.getByRole("region", { name: "선택한 세션" });
  await expect(session.getByText("사용 가능")).toBeVisible();
  await expect(session.getByText("123456789012", { exact: true })).toBeVisible();
  await capture(page, "sessions");
});

test("connects an IAM Identity Center profile", async ({ page }) => {
  await page.goto(`${fixture}?scenario=sso`);
  await page.getByRole("button", { name: "프로필 연결" }).click();
  const editor = page.getByRole("dialog", { name: "AWS 프로필 추가" });
  await expect(editor.getByLabel("인증 방식")).toHaveValue("sso");
  await expect(editor.getByLabel("Identity Center 프로필")).toHaveValue("team-dev");
  await capture(page, "sso", editor.getByRole("button", { name: "SSO 마법사 열기" }));
  await editor.getByRole("button", { name: "저장" }).click();
  await expect(page.getByRole("region", { name: "선택한 세션" }).getByText("사용 가능")).toBeVisible();
});
