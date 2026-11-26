import { element, escape } from "./html";

// https://docs.aws.amazon.com/global-infrastructure/latest/regions/aws-regions.html
const regions = {
  "아시아 태평양": {
    "ap-northeast-2": "서울",
    "ap-northeast-1": "도쿄",
    "ap-northeast-3": "오사카",
    "ap-southeast-1": "싱가포르",
    "ap-southeast-2": "시드니",
    "ap-east-1": "홍콩",
    "ap-east-2": "타이베이",
    "ap-south-1": "뭄바이",
    "ap-south-2": "하이데라바드",
    "ap-southeast-3": "자카르타",
    "ap-southeast-4": "멜버른",
    "ap-southeast-5": "말레이시아",
    "ap-southeast-6": "뉴질랜드",
    "ap-southeast-7": "태국",
  },
  "북미": {
    "us-east-1": "버지니아 북부",
    "us-east-2": "오하이오",
    "us-west-1": "캘리포니아 북부",
    "us-west-2": "오리건",
    "ca-central-1": "캐나다 중부",
    "ca-west-1": "캘거리",
    "mx-central-1": "멕시코 중부",
  },
  "유럽": {
    "eu-central-1": "프랑크푸르트",
    "eu-west-1": "아일랜드",
    "eu-west-2": "런던",
    "eu-west-3": "파리",
    "eu-north-1": "스톡홀름",
    "eu-south-1": "밀라노",
    "eu-south-2": "스페인",
    "eu-central-2": "취리히",
  },
  "기타 지역": {
    "sa-east-1": "상파울루",
    "af-south-1": "케이프타운",
    "il-central-1": "텔아비브",
    "me-south-1": "바레인",
    "me-central-1": "아랍에미리트",
  },
};
const customOption = "__custom__";

export function regionField(label: string, unset: string, region = ""): string {
  const custom = Boolean(region) && !Object.values(regions).some((group) => Object.hasOwn(group, region));
  const options = Object.entries(regions).map(([area, group]) =>
    `<optgroup label="${area}">${Object.entries(group).map(([code, city]) =>
      `<option value="${code}" ${code === region ? "selected" : ""}>${city} (${code})</option>`,
    ).join("")}</optgroup>`,
  ).join("");
  return `<label class="field">${label}
    <select name="region">
      <option value="">${unset}</option>
      ${options}
      <option value="${customOption}" ${custom ? "selected" : ""}>리전 코드 직접 입력</option>
    </select>
  </label>
  <label class="field" data-custom-region>리전 코드
    <input name="customRegion" maxlength="64" pattern="[A-Za-z0-9\\-]+" spellcheck="false"
      autocomplete="off" placeholder="예시 ap-northeast-2" value="${custom ? escape(region) : ""}" />
  </label>`;
}

export function bindRegionField(form: HTMLFormElement): () => string {
  const select = element<HTMLSelectElement>(form, '[name="region"]');
  const customField = element<HTMLLabelElement>(form, "[data-custom-region]");
  const customInput = element<HTMLInputElement>(customField, "input");
  const update = () => {
    const custom = select.value === customOption;
    customInput.required = custom;
    customInput.disabled = customField.hidden = !custom;
  };
  select.addEventListener("change", update);
  update();
  return () => select.value === customOption ? customInput.value.trim() : select.value;
}
