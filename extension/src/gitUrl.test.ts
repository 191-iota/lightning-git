import { describe, expect, it } from "vitest";
import { normalizeGitUrl } from "./gitUrl";

describe("normalizeGitUrl", () => {
  it("extracts owner/repo from an https url with .git", () => {
    expect(normalizeGitUrl("https://github.com/owner/repo.git")).toBe("owner/repo");
  });

  it("extracts owner/repo from an https url without .git", () => {
    expect(normalizeGitUrl("https://github.com/owner/repo")).toBe("owner/repo");
  });

  it("handles http (not only https)", () => {
    expect(normalizeGitUrl("http://github.com/owner/repo.git")).toBe("owner/repo");
  });

  it("extracts owner/repo from an ssh:// url", () => {
    expect(normalizeGitUrl("ssh://git@github.com/owner/repo.git")).toBe("owner/repo");
  });

  it("extracts owner/repo from scp-like git@host:owner/repo.git", () => {
    expect(normalizeGitUrl("git@github.com:owner/repo.git")).toBe("owner/repo");
  });

  it("extracts owner/repo from an ssh-config host alias form", () => {
    expect(normalizeGitUrl("github-iota:owner/repo.git")).toBe("owner/repo");
  });

  it("collapses every form to the same owner/repo so they match", () => {
    const forms = [
      "https://github.com/owner/repo.git",
      "https://github.com/owner/repo",
      "ssh://git@github.com/owner/repo.git",
      "git@github.com:owner/repo.git",
      "github-iota:owner/repo.git",
    ];
    const normalized = forms.map(normalizeGitUrl);
    expect(new Set(normalized).size).toBe(1);
    expect(normalized[0]).toBe("owner/repo");
  });

  it("strips a trailing .git", () => {
    expect(normalizeGitUrl("git@host:o/r.git")).toBe("o/r");
    expect(normalizeGitUrl("git@host:o/r")).toBe("o/r");
  });

  it("lowercases so case-differing remotes still match", () => {
    expect(normalizeGitUrl("https://github.com/Owner/Repo.git")).toBe("owner/repo");
  });

  it("preserves nested owner/repo paths (e.g. self-hosted subgroups)", () => {
    expect(normalizeGitUrl("https://gitlab.example.com/group/subgroup/repo.git")).toBe(
      "group/subgroup/repo",
    );
  });

  it("trims leading whitespace", () => {
    expect(normalizeGitUrl("  https://github.com/owner/repo.git")).toBe("owner/repo");
  });
});
