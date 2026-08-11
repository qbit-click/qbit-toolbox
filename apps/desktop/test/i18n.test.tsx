import { render, screen } from "@testing-library/react";
import { I18nProvider, useI18n } from "@qbit/i18n";
import { expect, test } from "vitest";

function Probe() {
  const { t } = useI18n();

  return (
    <>
      <output data-testid="interpolated">
        {t("test.greeting", { name: "Ada", count: 3 })}
      </output>
      <output data-testid="missing-key">{t("test.missing")}</output>
      <output data-testid="missing-parameter">
        {t("test.incomplete", { name: "Ada" })}
      </output>
    </>
  );
}

test("translates English catalogs and configures the document locale", () => {
  render(
    <I18nProvider
      catalogExtensions={{
        "test.greeting": "Hello, {name}. You have {count} messages.",
        "test.incomplete": "Hello, {name}. Your code is {code}.",
      }}
    >
      <Probe />
    </I18nProvider>,
  );

  expect(screen.getByTestId("interpolated")).toHaveTextContent(
    "Hello, Ada. You have 3 messages.",
  );
  expect(screen.getByTestId("missing-key")).toHaveTextContent("test.missing");
  expect(screen.getByTestId("missing-parameter")).toHaveTextContent(
    "Hello, Ada. Your code is {code}.",
  );
  expect(document.documentElement.lang).toBe("en");
  expect(document.documentElement.dir).toBe("ltr");
});
