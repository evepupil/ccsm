(function () {
  "use strict";

  const data = window.CCSM_V3_MOCK_DATA;
  const state = {
    provider: "claude",
    selectedProjectId: "ccsm",
    query: "",
    sort: "recent",
    showArchived: false,
    highestPermissions: false,
    launchingKey: null,
  };

  const elements = {
    providerOptions: Array.from(document.querySelectorAll(".provider-option")),
    search: document.querySelector("#session-search"),
    searchField: document.querySelector(".sidebar-search"),
    clearSearch: document.querySelector("#clear-search"),
    projectList: document.querySelector("#project-list"),
    projectCount: document.querySelector("#project-count"),
    workspaceName: document.querySelector("#workspace-name"),
    workspacePath: document.querySelector("#workspace-path"),
    workspaceCount: document.querySelector("#workspace-count"),
    contentTitle: document.querySelector("#content-title"),
    contentSummary: document.querySelector("#content-summary"),
    activeFilter: document.querySelector("#active-filter"),
    sort: document.querySelector("#sort-select"),
    archived: document.querySelector("#archived-toggle"),
    permission: document.querySelector("#permission-toggle"),
    permissionWarning: document.querySelector("#permission-warning"),
    sessionGroups: document.querySelector("#session-groups"),
    windowProvider: document.querySelector("#window-provider"),
    toast: document.querySelector("#toast"),
  };

  let toastTimer = null;

  function escapeHtml(value) {
    return String(value)
      .replaceAll("&", "&amp;")
      .replaceAll("<", "&lt;")
      .replaceAll(">", "&gt;")
      .replaceAll('"', "&quot;")
      .replaceAll("'", "&#039;");
  }

  function providerName(provider) {
    return provider === "claude" ? "Claude Code" : "Codex";
  }

  function providerIcon(provider) {
    return provider === "claude" ? "sparkles" : "braces";
  }

  function currentProjects() {
    return data[state.provider] ?? [];
  }

  function selectedProject() {
    return currentProjects().find((project) => project.id === state.selectedProjectId) ?? null;
  }

  function normalizedQuery() {
    return state.query.trim().toLocaleLowerCase("zh-CN");
  }

  function matchesQuery(values, query) {
    return values.join(" ").toLocaleLowerCase("zh-CN").includes(query);
  }

  function visibleProjects() {
    const query = normalizedQuery();
    if (!query) return currentProjects();

    return currentProjects().filter(
      (project) =>
        matchesQuery([project.name, project.path], query) ||
        project.sessions.some((session) =>
          matchesQuery([session.title, session.id, session.branch, session.model], query),
        ),
    );
  }

  function visibleSessions() {
    const query = normalizedQuery();
    const source = query
      ? currentProjects().flatMap((project) =>
          project.sessions.map((session) => ({
            ...session,
            projectName: project.name,
            projectPath: project.path,
          })),
        )
      : (selectedProject()?.sessions ?? []);

    const filtered = source.filter((session) => {
      if (!state.showArchived && session.archived) return false;
      if (!query) return true;
      return matchesQuery(
        [
          session.title,
          session.id,
          session.branch,
          session.model,
          session.projectName,
          session.projectPath,
        ],
        query,
      );
    });

    return [...filtered].sort((left, right) => {
      if (state.sort === "title") return left.title.localeCompare(right.title, "zh-CN");
      return right.sortTime.localeCompare(left.sortTime);
    });
  }

  function renderProviders() {
    elements.providerOptions.forEach((option) => {
      const selected = option.dataset.provider === state.provider;
      option.setAttribute("aria-selected", String(selected));
      option.tabIndex = selected ? 0 : -1;
    });
    elements.windowProvider.textContent = providerName(state.provider);
  }

  function renderProjects() {
    const projects = visibleProjects();
    elements.projectCount.textContent = String(projects.length);

    if (projects.length === 0) {
      elements.projectList.innerHTML = '<div class="sidebar-empty">没有匹配的项目</div>';
      return;
    }

    elements.projectList.innerHTML = projects
      .map((project) => {
        const selected = !normalizedQuery() && project.id === state.selectedProjectId;
        const count = project.sessions.filter(
          (session) => state.showArchived || !session.archived,
        ).length;
        return `
          <button
            class="project-item"
            type="button"
            data-project-id="${escapeHtml(project.id)}"
            aria-current="${selected ? "page" : "false"}"
            title="${escapeHtml(project.path)}"
          >
            <span class="project-icon" aria-hidden="true"><i data-lucide="folder-code"></i></span>
            <span class="project-copy">
              <strong>${escapeHtml(project.name)}</strong>
              <small>${escapeHtml(project.lastActivity)}</small>
            </span>
            <span class="project-session-count">${count}</span>
          </button>
        `;
      })
      .join("");
  }

  function renderHeadings(sessions) {
    const project = selectedProject();
    const searching = Boolean(normalizedQuery());

    elements.workspaceName.textContent = searching ? "全局搜索" : (project?.name ?? "选择一个项目");
    elements.workspacePath.textContent = searching
      ? `正在 ${providerName(state.provider)} 的全部项目中搜索`
      : (project?.path ?? "");
    elements.workspaceCount.textContent = String(sessions.length);
    elements.contentTitle.textContent = searching ? "搜索结果" : "会话";
    elements.contentSummary.textContent = `${sessions.length} 个${state.showArchived ? "匹配" : "可续接"}会话`;
    elements.activeFilter.hidden = !searching;
    elements.activeFilter.textContent = searching ? `“${state.query}”` : "";
  }

  function groupSessions(sessions) {
    if (state.sort === "title") return [["按标题排序", sessions]];
    const labels = ["今天", "昨天", "本周", "更早"];
    return labels
      .map((label) => [label, sessions.filter((session) => session.group === label)])
      .filter(([, items]) => items.length > 0);
  }

  function sessionMarkup(session) {
    const key = `${state.provider}:${session.id}`;
    const launching = state.launchingKey === key;
    const projectMeta = session.projectName
      ? `<span><i data-lucide="folder" aria-hidden="true"></i>${escapeHtml(session.projectName)}</span>`
      : "";

    return `
      <article class="session-card" data-session-id="${escapeHtml(session.id)}">
        <span class="session-provider-icon ${state.provider}" aria-hidden="true">
          <i data-lucide="${providerIcon(state.provider)}"></i>
        </span>
        <div class="session-copy">
          <div class="session-heading-line">
            <h3 class="session-title" title="${escapeHtml(session.title)}">${escapeHtml(session.title)}</h3>
            ${
              session.archived
                ? '<span class="archived-badge"><i data-lucide="archive"></i>已归档</span>'
                : ""
            }
          </div>
          <div class="session-meta">
            ${projectMeta}
            <span><i data-lucide="git-branch" aria-hidden="true"></i>${escapeHtml(session.branch)}</span>
            <span class="model-meta"><i data-lucide="cpu" aria-hidden="true"></i>${escapeHtml(session.model)}</span>
            <span><i data-lucide="messages-square" aria-hidden="true"></i>${escapeHtml(session.usage)}</span>
          </div>
          <div class="session-id-line">
            <code class="session-id" title="${escapeHtml(session.id)}">${escapeHtml(session.id)}</code>
            <button class="copy-button" type="button" data-action="copy" title="复制 Session ID">
              <i data-lucide="copy" aria-hidden="true"></i>
              <span class="sr-only">复制 Session ID</span>
            </button>
          </div>
        </div>
        <time class="session-activity" datetime="${escapeHtml(session.sortTime)}">
          ${escapeHtml(session.relativeTime)}
          <small>${escapeHtml(session.absoluteTime)}</small>
        </time>
        <div class="session-actions">
          <button
            class="resume-button ${launching ? "is-launching" : ""}"
            type="button"
            data-action="resume"
            ${!session.canResume || launching ? "disabled" : ""}
            title="${session.canResume ? "在新终端中继续会话" : "原项目目录已不存在"}"
          >
            <i data-lucide="${launching ? "loader-circle" : "play"}" aria-hidden="true"></i>
            <span>${launching ? "启动中" : session.canResume ? "继续" : "不可用"}</span>
          </button>
          <button
            class="icon-button fork-button"
            type="button"
            data-action="fork"
            ${!session.canResume || launching ? "disabled" : ""}
            title="分叉续接"
          >
            <i data-lucide="git-fork" aria-hidden="true"></i>
            <span class="sr-only">分叉续接</span>
          </button>
        </div>
      </article>
    `;
  }

  function renderSessions() {
    const sessions = visibleSessions();
    renderHeadings(sessions);

    if (sessions.length === 0) {
      elements.sessionGroups.innerHTML = `
        <div class="empty-state">
          <span class="empty-state-icon"><i data-lucide="message-square-off"></i></span>
          <h3>没有可显示的会话</h3>
          <p>${normalizedQuery() ? "换个关键词再试一次。" : "当前筛选条件下没有会话。"}</p>
        </div>
      `;
      return;
    }

    elements.sessionGroups.innerHTML = groupSessions(sessions)
      .map(
        ([label, items]) => `
          <section class="session-group" aria-label="${escapeHtml(label)}">
            <div class="session-group-heading">
              <span>${escapeHtml(label)}</span>
              <small>${items.length}</small>
            </div>
            <div class="session-list">${items.map(sessionMarkup).join("")}</div>
          </section>
        `,
      )
      .join("");
  }

  function render() {
    renderProviders();
    renderProjects();
    renderSessions();
    elements.searchField.classList.toggle("has-value", Boolean(state.query));
    elements.permissionWarning.hidden = !state.highestPermissions;
    window.lucide?.createIcons();
  }

  function showToast(message) {
    window.clearTimeout(toastTimer);
    elements.toast.textContent = message;
    elements.toast.classList.add("is-visible");
    toastTimer = window.setTimeout(() => elements.toast.classList.remove("is-visible"), 2200);
  }

  function selectProvider(provider) {
    state.provider = provider;
    const matchingProject = currentProjects().find(
      (project) => project.id === state.selectedProjectId,
    );
    state.selectedProjectId = matchingProject?.id ?? currentProjects()[0]?.id ?? null;
    state.launchingKey = null;
    render();
  }

  function copyText(value) {
    if (navigator.clipboard?.writeText) return navigator.clipboard.writeText(value);
    const textarea = document.createElement("textarea");
    textarea.value = value;
    textarea.style.position = "fixed";
    textarea.style.opacity = "0";
    document.body.append(textarea);
    textarea.select();
    document.execCommand("copy");
    textarea.remove();
    return Promise.resolve();
  }

  function simulateLaunch(session, fork) {
    state.launchingKey = `${state.provider}:${session.id}`;
    render();
    window.setTimeout(() => {
      state.launchingKey = null;
      render();
      showToast(
        `${providerName(state.provider)} ${fork ? "分叉会话" : "原会话"}已模拟启动${
          state.highestPermissions ? "（最高权限）" : ""
        }`,
      );
    }, 850);
  }

  elements.providerOptions.forEach((option) => {
    option.addEventListener("click", () => selectProvider(option.dataset.provider));
  });

  elements.search.addEventListener("input", (event) => {
    state.query = event.target.value;
    render();
  });

  elements.clearSearch.addEventListener("click", () => {
    state.query = "";
    elements.search.value = "";
    elements.search.focus();
    render();
  });

  elements.projectList.addEventListener("click", (event) => {
    const item = event.target.closest("[data-project-id]");
    if (!item) return;
    state.selectedProjectId = item.dataset.projectId;
    state.query = "";
    elements.search.value = "";
    render();
  });

  elements.sort.addEventListener("change", (event) => {
    state.sort = event.target.value;
    render();
  });

  elements.archived.addEventListener("change", (event) => {
    state.showArchived = event.target.checked;
    render();
  });

  elements.permission.addEventListener("change", (event) => {
    state.highestPermissions = event.target.checked;
    render();
  });

  elements.sessionGroups.addEventListener("click", (event) => {
    const button = event.target.closest("[data-action]");
    const card = event.target.closest("[data-session-id]");
    if (!button || !card) return;
    const session = visibleSessions().find((item) => item.id === card.dataset.sessionId);
    if (!session) return;

    if (button.dataset.action === "copy") {
      copyText(session.id)
        .then(() => showToast("Session ID 已复制"))
        .catch(() => showToast("无法访问系统剪贴板"));
      return;
    }

    simulateLaunch(session, button.dataset.action === "fork");
  });

  document.querySelectorAll("[data-window-action]").forEach((button) => {
    button.addEventListener("click", () => {
      const labels = { minimize: "最小化", maximize: "最大化", close: "关闭" };
      showToast(`原型预览：${labels[button.dataset.windowAction]}窗口`);
    });
  });

  render();
})();
