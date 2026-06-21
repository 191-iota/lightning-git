// collapses every git URL form to just "owner/repo" so https on the web side
// matches ssh + ssh-config-alias on the local side. handles:
//   https://github.com/owner/repo.git
//   ssh://git@github.com/owner/repo.git
//   git@github.com:owner/repo.git
//   github-iota:owner/repo.git   (user's ~/.ssh/config Host alias)
export function normalizeGitUrl(url: string): string {
  const trimmed = url.replace(/\.git$/, "").trim().toLowerCase();
  const urlMatch = trimmed.match(/^(?:https?|ssh):\/\/[^/]+\/(.+)$/);
  if (urlMatch) return urlMatch[1];
  const sshMatch = trimmed.match(/^(?:git@)?[^:/]+:(.+)$/);
  if (sshMatch) return sshMatch[1];
  return trimmed;
}
