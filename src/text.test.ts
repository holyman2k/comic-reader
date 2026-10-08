import { describe, expect, it } from "vitest";
import { displayName, formatCounter, openingLabel, resumeToastText } from "./text";

describe("text", () => {
  it("takes the last path segment on both platforms", () => {
    expect(displayName("/Users/me/Comics/Vol 1.cbz")).toBe("Vol 1.cbz");
    expect(displayName("C:\\Comics\\Vol 2.cbr")).toBe("Vol 2.cbr");
    expect(displayName("/Users/me/Comics/Folder/")).toBe("Folder");
  });

  it("says Extracting for rar and tar.gz, Opening otherwise", () => {
    expect(openingLabel("/a/b.CBR")).toBe("Extracting b.CBR…");
    expect(openingLabel("/a/b.rar")).toBe("Extracting b.rar…");
    expect(openingLabel("/a/b.tar.gz")).toBe("Extracting b.tar.gz…");
    expect(openingLabel("/a/b.tgz")).toBe("Extracting b.tgz…");
    expect(openingLabel("/a/b.cbz")).toBe("Opening b.cbz…");
    expect(openingLabel("/a/Folder")).toBe("Opening Folder…");
  });

  it("formats the page counter", () => {
    expect(formatCounter(12, 48)).toBe("12 / 48");
    expect(formatCounter(0, 0)).toBe("");
  });
});

describe("resumeToastText", () => {
  it("names the page", () => {
    expect(resumeToastText(12)).toBe("Resumed at page 12");
  });
});
