import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import FileTreeNode, { type TreeNode } from "./FileTreeNode.vue";

function leaf(path: string): TreeNode {
  const segments = path.split("/");
  return {
    name: segments[segments.length - 1],
    fullPath: path,
    isFile: true,
    children: [],
  };
}

function folder(name: string, children: TreeNode[]): TreeNode {
  return { name, fullPath: "", isFile: false, children };
}

describe("FileTreeNode (file)", () => {
  it("shows a muted dot and no live badge when not edited", () => {
    const wrapper = mount(FileTreeNode, {
      props: {
        node: leaf("src/main.rs"),
        selected: null,
        editedFiles: new Set<string>(),
      },
    });
    const dot = wrapper.find("span.rounded-full");
    expect(dot.classes()).toContain("bg-lg-border-strong");
    expect(dot.classes()).not.toContain("animate-pulse");
    expect(wrapper.text()).not.toContain("live");
  });

  it("shows a pulsing accent dot and live badge when edited", () => {
    const wrapper = mount(FileTreeNode, {
      props: {
        node: leaf("src/main.rs"),
        selected: null,
        editedFiles: new Set(["src/main.rs"]),
      },
    });
    const dot = wrapper.find("span.rounded-full");
    expect(dot.classes()).toContain("bg-lg-ink");
    expect(dot.classes()).toContain("animate-pulse");
    expect(wrapper.text()).toContain("live");
  });

  it("emits open event with the full path on click", async () => {
    const wrapper = mount(FileTreeNode, {
      props: {
        node: leaf("src/main.rs"),
        selected: null,
        editedFiles: new Set<string>(),
      },
    });
    await wrapper.find("button").trigger("click");
    expect(wrapper.emitted("open")).toBeTruthy();
    expect(wrapper.emitted("open")![0]).toEqual(["src/main.rs"]);
  });

  it("marks the selected file with a distinct background class", () => {
    const wrapper = mount(FileTreeNode, {
      props: {
        node: leaf("src/main.rs"),
        selected: "src/main.rs",
        editedFiles: new Set<string>(),
      },
    });
    const btn = wrapper.find("button");
    expect(btn.classes().some((c) => c.includes("bg-"))).toBe(true);
  });
});

describe("FileTreeNode (folder)", () => {
  it("renders children when expanded by default", () => {
    const node = folder("src", [leaf("src/main.rs"), leaf("src/lib.rs")]);
    const wrapper = mount(FileTreeNode, {
      props: { node, selected: null, editedFiles: new Set<string>() },
    });
    expect(wrapper.text()).toContain("main.rs");
    expect(wrapper.text()).toContain("lib.rs");
  });

  it("collapses children when the folder header is clicked", async () => {
    const node = folder("src", [leaf("src/main.rs")]);
    const wrapper = mount(FileTreeNode, {
      props: { node, selected: null, editedFiles: new Set<string>() },
    });
    expect(wrapper.text()).toContain("main.rs");
    await wrapper.find("button").trigger("click");
    expect(wrapper.text()).not.toContain("main.rs");
  });

  it("propagates open events from children", async () => {
    const node = folder("src", [leaf("src/main.rs")]);
    const wrapper = mount(FileTreeNode, {
      props: { node, selected: null, editedFiles: new Set<string>() },
    });
    const leafBtn = wrapper.findAll("button").find((b) => b.text().includes("main.rs"));
    expect(leafBtn).toBeDefined();
    await leafBtn!.trigger("click");
    expect(wrapper.emitted("open")).toBeTruthy();
    expect(wrapper.emitted("open")![0]).toEqual(["src/main.rs"]);
  });
});
