import { element, escape, text } from "../../shared/html";

// https://docs.aws.amazon.com/global-infrastructure/latest/regions/aws-regions.html
const regions = {
  asiaPacific: [
    "ap-northeast-2", "ap-northeast-1", "ap-northeast-3", "ap-southeast-1", "ap-southeast-2", "ap-east-1", "ap-east-2",
    "ap-south-1", "ap-south-2", "ap-southeast-3", "ap-southeast-4", "ap-southeast-5", "ap-southeast-6", "ap-southeast-7",
  ],
  northAmerica: [
    "us-east-1", "us-east-2", "us-west-1", "us-west-2", "ca-central-1", "ca-west-1", "mx-central-1",
  ],
  europe: [
    "eu-central-1", "eu-west-1", "eu-west-2", "eu-west-3", "eu-north-1", "eu-south-1", "eu-south-2", "eu-central-2",
  ],
  other: [
    "sa-east-1", "af-south-1", "il-central-1", "me-south-1", "me-central-1",
  ],
} as const;
const customOption = "__custom__";

export function regionField(label: string, unset: string, region = ""): string {
  const custom = Boolean(region) && !Object.values(regions).some((group) => group.some((code) => code === region));
  const options = Object.entries(regions).map(([area, group]) =>
    `<optgroup label="${text(`region.${area as keyof typeof regions}`)}">${group.map((code) =>
      `<option value="${code}" ${code === region ? "selected" : ""}>${text(`region.${code}`)} (${code})</option>`,
    ).join("")}</optgroup>`,
  ).join("");
  return `<label class="field">${escape(label)}
    <select name="region">
      <option value="">${escape(unset)}</option>
      ${options}
      <option value="${customOption}" ${custom ? "selected" : ""}>${text("region.enterCode")}</option>
    </select>
  </label>
  <label class="field" data-custom-region>${text("region.code")}
    <input name="customRegion" maxlength="64" pattern="[A-Za-z0-9\\-]+" spellcheck="false"
      autocomplete="off" placeholder="${text("region.example")}" value="${custom ? escape(region) : ""}" />
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
