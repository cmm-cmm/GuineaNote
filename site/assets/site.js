// Guinea Note website: install tabs, copy buttons, default tab for the
// visitor's OS. Without JavaScript every install panel is shown in turn.
(function () {
  "use strict";
  var root = document.documentElement;
  root.classList.remove("no-js");
  root.classList.add("js");

  // Pick a default install tab from the platform: Linux gets the Linux
  // archive; macOS gets a notice (no macOS build); everything else npm.
  var platform = ((navigator.userAgentData && navigator.userAgentData.platform) || navigator.platform || navigator.userAgent || "").toLowerCase();
  var isMac = /mac|iphone|ipad/.test(platform);
  var isLinux = !isMac && /linux|x11/.test(platform) && !/android/.test(platform);
  if (isMac) root.classList.add("is-mac");

  var tablist = document.querySelector("[data-tabs]");
  if (tablist) {
    var tabs = Array.prototype.slice.call(tablist.querySelectorAll("[role=tab]"));
    var select = function (tab, focus) {
      tabs.forEach(function (other) {
        var on = other === tab;
        other.setAttribute("aria-selected", on ? "true" : "false");
        other.tabIndex = on ? 0 : -1;
        var panel = document.getElementById(other.getAttribute("aria-controls"));
        if (panel) panel.hidden = !on;
      });
      if (focus) tab.focus();
    };
    tabs.forEach(function (tab, index) {
      tab.addEventListener("click", function () {
        select(tab, false);
        if (history.replaceState) history.replaceState(null, "", "#" + tab.dataset.hash);
      });
      tab.addEventListener("keydown", function (event) {
        var next = null;
        if (event.key === "ArrowRight") next = tabs[(index + 1) % tabs.length];
        else if (event.key === "ArrowLeft") next = tabs[(index - 1 + tabs.length) % tabs.length];
        else if (event.key === "Home") next = tabs[0];
        else if (event.key === "End") next = tabs[tabs.length - 1];
        if (next) { event.preventDefault(); select(next, true); }
      });
    });
    var byHash = function (hash) {
      for (var i = 0; i < tabs.length; i++) if (tabs[i].dataset.hash === hash) return tabs[i];
      return null;
    };
    var wanted = byHash(location.hash.slice(1)) || byHash(isLinux ? "install-linux" : "install-npm") || tabs[0];
    select(wanted, false);
    // Links elsewhere on the page that point at a tab (#install-linux, ...).
    document.addEventListener("click", function (event) {
      var link = event.target.closest && event.target.closest("a[href^='#install-']");
      var tab = link && byHash(link.getAttribute("href").slice(1));
      if (tab) { select(tab, false); }
    });
  }

  // Copy buttons copy the text of the <pre> next to them, without prompts
  // (spans marked .p) and without comment lines (spans marked .c).
  var label = document.documentElement.lang === "vi" ? ["Chép", "Đã chép"] : ["Copy", "Copied"];
  Array.prototype.forEach.call(document.querySelectorAll(".cmd"), function (box) {
    var pre = box.querySelector("pre");
    if (!pre || !navigator.clipboard) return;
    var button = document.createElement("button");
    button.type = "button";
    button.className = "copy";
    button.textContent = label[0];
    button.setAttribute("aria-label", label[0] + ": " + pre.textContent.trim().split("\n")[0]);
    button.addEventListener("click", function () {
      var copy = pre.cloneNode(true);
      Array.prototype.forEach.call(copy.querySelectorAll(".p, .c"), function (node) { node.remove(); });
      var text = copy.textContent.split("\n").map(function (line) { return line.replace(/\s+$/, ""); })
        .filter(function (line) { return line.length; }).join("\n");
      navigator.clipboard.writeText(text).then(function () {
        button.textContent = label[1];
        button.setAttribute("data-done", "");
        setTimeout(function () { button.textContent = label[0]; button.removeAttribute("data-done"); }, 1800);
      });
    });
    box.appendChild(button);
  });
})();
