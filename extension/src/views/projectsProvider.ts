import * as vscode from "vscode";

const PROJECTS_KEY = "lightningGit.projects";
const WORKSPACE_PROJECT_ID_KEY = "lightning-git.projectId";

export type SavedProject = Readonly<{
  id: string;
  label: string;
}>;

class ProjectItem extends vscode.TreeItem {
  readonly project: SavedProject;

  constructor(project: SavedProject) {
    super(project.label, vscode.TreeItemCollapsibleState.None);
    this.project = project;
    this.description = project.id;
    this.contextValue = "lightningGit.project";
    this.command = {
      command: "lightningGit.openProject",
      title: "Open Project",
      arguments: [project]
    };
  }
}

export class ProjectsProvider implements vscode.TreeDataProvider<ProjectItem> {
  readonly #context: vscode.ExtensionContext;
  readonly #onDidChangeTreeData = new vscode.EventEmitter<ProjectItem | undefined | void>();

  constructor(context: vscode.ExtensionContext) {
    this.#context = context;
  }

  get onDidChangeTreeData(): vscode.Event<ProjectItem | undefined | void> {
    return this.#onDidChangeTreeData.event;
  }

  refresh(): void {
    this.#onDidChangeTreeData.fire();
  }

  getTreeItem(element: ProjectItem): vscode.TreeItem {
    return element;
  }

  async getChildren(element?: ProjectItem): Promise<ProjectItem[]> {
    if (element) {
      return [];
    }

    const projects = await this.getProjects();
    return projects.map((project) => new ProjectItem(project));
  }

  async getProjects(): Promise<SavedProject[]> {
    const raw = this.#context.globalState.get<SavedProject[]>(PROJECTS_KEY, []);
    return raw.slice().sort((a, b) => a.label.localeCompare(b.label));
  }

  async addProject(): Promise<void> {
    const projectId = await vscode.window.showInputBox({
      title: "Lightning Git: Add Project",
      prompt: "Paste the project UUID from the lightning-git backend",
      ignoreFocusOut: true,
      validateInput: (value) => {
        if (!value.trim()) {
          return "Project UUID is required.";
        }
        return undefined;
      }
    });

    if (!projectId) {
      return;
    }

    const trimmedId = projectId.trim();

    const label = await vscode.window.showInputBox({
      title: "Lightning Git: Project Label",
      prompt: "Optional friendly label for this project",
      value: trimmedId,
      ignoreFocusOut: true
    });

    const projects = await this.getProjects();
    const nextProject: SavedProject = {
      id: trimmedId,
      label: (label?.trim() || trimmedId)
    };

    const deduped = projects.filter((project) => project.id !== trimmedId);
    deduped.push(nextProject);

    await this.#context.globalState.update(PROJECTS_KEY, deduped);
    this.refresh();
  }

  async openProject(project: SavedProject): Promise<void> {
    await this.#context.workspaceState.update(WORKSPACE_PROJECT_ID_KEY, project.id);

    void vscode.window.showInformationMessage(`Lightning Git project active: ${project.label}`);
  }
}

export function registerProjectsView(
  context: vscode.ExtensionContext
): { provider: ProjectsProvider; disposable: vscode.Disposable } {
  const provider = new ProjectsProvider(context);
  const tree = vscode.window.createTreeView("lightningGit.projectsView", {
    treeDataProvider: provider,
    showCollapseAll: false
  });

  return {
    provider,
    disposable: tree
  };
}