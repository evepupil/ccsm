(function () {
  "use strict";

  const data = window.CCSM_MOCK_DATA;
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
    tabs: Array.from(document.querySelectorAll(".provider-tab")),
    search: document.querySelector("#session-search"),
    searchField: document.querySelector(".search-field"),
    clearSearch: document.querySelector("#clear-search"),
    refresh: document.querySelector("#refresh-button"),
    projectList: document.querySelector("#project-list"),
    projectCount: document.querySelector("#project-count"),
    totalSessionCount: document.querySelector("#total-session-count"),
    workspaceTitle: document.querySelector("#workspace-title"),
    workspacePath: document.querySelector("#workspace-path"),
    workspaceCount: document.querySelector("#workspace-count"),
    sort: document.querySelector("#sort-select"),
    archived: document.querySelector("#archived-toggle"),
    permission: document.querySelector("#permission-toggle"),
    permissionWarning: document.querySelector("#permission-warning"),
    sessionList: document.querySelector("#session-list"),
    sourceSummary: document.querySelector("#source-summary"),
    scanStatus: document.querySelector("#scan-status"),
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

  function providerLabel(provider) {
    return provider === "claude" ? "Claude" : "Codex";
  }

  function allProjects() {
    return data[state.provider];
  }

  function matchesQuery(project, query) {
    if (!query) return true;
    const projectText = `${project.name} ${project.path}`.toLocaleLowerCase("zh-CN");
    if (projectText.includes(query)) return true;
    return project.sessions.some((session) => {
      const sessionText = [session.title, session.id, session.branch, session.model]
        .join(" ")
        .toLocaleLowerCase("zh-CN");
      return sessionText.includes(query);
    });
  }

  function visibleProjects() {
    const query = state.query.trim().toLocaleLowerCase("zh-CN");
    return allProjects().filter((project) => matchesQuery(project, query));
  }

  function selectedProject() {
    return allProjects().find((project) => project.id === state.selectedProjectId) ?? null;
  }

  function visibleSessions() {
    const query = state.query.trim().toLocaleLowerCase("zh-CN");
    const source = query
      ? allProjects().flatMap((project) =>
          project.sessions.map((session) => ({ ...session, projectName: project.name })),
        )
      : (selectedProject()?.sessions ?? []);

    const filtered = source.filter((session) => {
      if (!state.showArchived && session.archived) return false;
      if (!query) return true;
      return [session.title, session.id, session.branch, session.model, session.projectName]
        .join(" ")
        .toLocaleLowerCase("zh-CN")
        .includes(query);
    });

    return [...filtered].sort((left, right) => {
      if (state.sort === "title") return left.title.localeCompare(right.title, "zh-CN");
      return left.absoluteTime < right.absoluteTime ? 1 : -1;
    });
  }

  function renderTabs() {
    elements.tabs.forEach((tab) => {
      const selected = tab.dataset.provider === state.provider;
      tab.setAttribute("aria-selected", String(selected));
      tab.tabIndex = selected ? 0 : -1;
    });
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
        const selected = !state.query && project.id === state.selectedProjectId;
        const visibleCount = project.sessions.filter(
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
            <i data-lucide="folder" aria-hidden="true"></i>
            <span class="project-item-copy">
              <strong>${escapeHtml(project.name)}</strong>
              <small>${escapeHtml(project.lastActivity)}</small>
            </span>
            <span class="project-session-count">${visibleCount}</span>
          </button>
        `;
      })
      .join("");
  }

  function renderHeading(sessions) {
    const project = selectedProject();
    if (state.query) {
      elements.workspaceTitle.textContent = "搜索结果";
      elements.workspacePath.textContent = `在 ${providerLabel(state.provider)} 会话中搜索“${state.query}”`;
    } else {
      elements.workspaceTitle.textContent = project?.name ?? "选择一个项目";
      elements.workspacePath.textContent = project?.path ?? "";
    }
    elements.workspaceCount.textContent = `${sessions.length} 条`;
  }

  function renderSessions() {
    const sessions = visibleSessions();
    renderHeading(sessions);

    if (sessions.length === 0) {
      elements.sessionList.innerHTML = `
        <div class="empty-state">
          <i data-lucide="message-square-off" aria-hidden="true"></i>
          <h2>没有可显示的会话</h2>
          <p>${state.query ? "没有找到匹配结果。" : "当前筛选条件下没有会话。"}</p>
        </div>
      `;
      return;
    }

    elements.sessionList.innerHTML = sessions
      .map((session) => {
        const sessionKey = `${state.provider}:${session.id}`;
        const launching = state.launchingKey === sessionKey;
        const provider = providerLabel(state.provider);
        const projectMeta =
          state.query && session.projectName
            ? `<span>${escapeHtml(session.projectName)}</span>`
            : "";
        return `
          <article class="session-row" data-session-id="${escapeHtml(session.id)}">
            <div class="session-main">
              <div class="session-title-line">
                <span class="session-title" title="${escapeHtml(session.title)}">${escapeHtml(session.title)}</span>
                <span class="provider-badge ${state.provider}">${provider}</span>
                ${
                  session.archived
                    ? '<span class="archived-badge"><i data-lucide="archive"></i>已归档</span>'
                    : ""
                }
              </div>
              <div class="session-meta">
                ${projectMeta}
                <span>${escapeHtml(session.branch)}</span>
                <span>${escapeHtml(session.model)}</span>
                <span>${escapeHtml(session.usage)}</span>
                <span>${escapeHtml(session.size)}</span>
                <span class="session-mobile-time"><i data-lucide="clock-3"></i>${escapeHtml(session.relativeTime)}</span>
              </div>
              <div class="session-id-line">
                <code class="session-id" title="${escapeHtml(session.id)}">${escapeHtml(session.id)}</code>
                <button class="icon-button copy-button" type="button" data-action="copy" title="复制 Session ID">
                  <i data-lucide="copy" aria-hidden="true"></i>
                  <span class="sr-only">复制 Session ID</span>
                </button>
              </div>
            </div>
            <time class="session-activity" datetime="${escapeHtml(session.absoluteTime)}">
              ${escapeHtml(session.relativeTime)}
              <small>${escapeHtml(session.absoluteTime)}</small>
            </time>
            <div class="session-actions">
              <button
                class="primary-button ${launching ? "is-launching" : ""}"
                type="button"
                data-action="resume"
                ${!session.canResume || launching ? "disabled" : ""}
                title="${session.canResume ? "在新终端中继续会话" : "原项目目录已不存在"}"
              >
                <i data-lucide="${launching ? "loader-circle" : "play"}" aria-hidden="true"></i>
                <span>${launching ? "启动中" : "继续"}</span>
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
      })
      .join("");
  }

  function renderStatus() {
    const claudeCount = data.claude.reduce((sum, project) => sum + project.sessions.length, 0);
    const codexCount = data.codex.reduce((sum, project) => sum + project.sessions.length, 0);
    const total = claudeCount + codexCount;
    elements.totalSessionCount.textContent = `${total} 条本机会话`;
    elements.sourceSummary.textContent = `Claude Code ${claudeCount} 条 · Codex ${codexCount} 条`;
  }

  function render() {
    renderTabs();
    renderProjects();
    renderSessions();
    renderStatus();
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
    const matchingProject = data[provider].find(
      (project) => project.id === state.selectedProjectId,
    );
    state.selectedProjectId = matchingProject?.id ?? data[provider][0]?.id ?? null;
    state.launchingKey = null;
    render();
  }

  function copyText(value) {
    if (navigator.clipboard?.writeText) {
      return navigator.clipboard.writeText(value);
    }
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
        `${providerLabel(state.provider)} ${fork ? "分叉会话" : "原会话"}已模拟启动${
          state.highestPermissions ? "（最高权限）" : ""
        }`,
      );
    }, 900);
  }

  elements.tabs.forEach((tab) => {
    tab.addEventListener("click", () => selectProvider(tab.dataset.provider));
  });

  elements.search.addEventListener("input", (event) => {
    state.query = event.target.value.trim();
    render();
  });

  elements.clearSearch.addEventListener("click", () => {
    state.query = "";
    elements.search.value = "";
    elements.search.focus();
    render();
  });

  elements.refresh.addEventListener("click", () => {
    elements.refresh.classList.add("is-refreshing");
    elements.refresh.disabled = true;
    elements.scanStatus.textContent = "正在扫描本机会话...";
    window.setTimeout(() => {
      elements.refresh.classList.remove("is-refreshing");
      elements.refresh.disabled = false;
      elements.scanStatus.textContent = "上次扫描：刚刚";
      showToast("扫描完成，会话列表已更新");
    }, 700);
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

  elements.sessionList.addEventListener("click", (event) => {
    const button = event.target.closest("[data-action]");
    const row = event.target.closest("[data-session-id]");
    if (!button || !row) return;
    const session = visibleSessions().find((item) => item.id === row.dataset.sessionId);
    if (!session) return;

    if (button.dataset.action === "copy") {
      copyText(session.id)
        .then(() => showToast("Session ID 已复制"))
        .catch(() => showToast("无法访问系统剪贴板"));
      return;
    }

    simulateLaunch(session, button.dataset.action === "fork");
  });

  render();
})();
