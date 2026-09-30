// kaikichat.com in the visitor's language: the page is written in English,
// and every other language of the app comes from its table in /i18n/.
// The language is ?lang=, else the browser's first language the site has.
// tests/site/test_site_i18n.py keeps LANGUAGES equal to the app's and every
// table complete and translated.
(function () {
  'use strict';
  var LANGUAGES = [
    ['en', 'English'], ['es', 'Español'], ['de', 'Deutsch'], ['ja', '日本語'],
    ['fr', 'Français'], ['pt', 'Português'], ['ru', 'Русский'], ['it', 'Italiano'],
    ['nl', 'Nederlands'], ['pl', 'Polski'], ['tr', 'Türkçe'], ['zh', '简体中文'],
    ['fa', 'فارسی'], ['vi', 'Tiếng Việt'], ['cs', 'Čeština'], ['id', 'Bahasa Indonesia'],
    ['ko', '한국어'], ['uk', 'Українська'], ['hu', 'Magyar'], ['ar', 'العربية']
  ];
  var RTL = { fa: true, ar: true };
  var codes = LANGUAGES.map(function (entry) { return entry[0]; });
  var english = {
    copied: 'Copied. Paste it into Claude Code or Codex.',
    copyManually: 'The text is selected: press ⌘C or Ctrl+C.'
  };

  function chosen() {
    var asked = new URLSearchParams(location.search).get('lang');
    if (asked && codes.indexOf(asked) >= 0) return { code: asked, explicit: true };
    var wanted = navigator.languages && navigator.languages.length ? navigator.languages : [navigator.language || 'en'];
    for (var i = 0; i < wanted.length; i++) {
      var primary = String(wanted[i]).toLowerCase().split('-')[0];
      if (codes.indexOf(primary) >= 0) return { code: primary, explicit: false };
    }
    return { code: 'en', explicit: false };
  }

  function withLang(href, code) {
    var url = new URL(href, location.href);
    if (url.origin !== location.origin) return href;
    url.searchParams.set('lang', code);
    return url.pathname + url.search + url.hash;
  }

  function apply(strings, code) {
    var root = document.documentElement;
    root.lang = code;
    root.dir = RTL[code] ? 'rtl' : 'ltr';
    document.querySelectorAll('[data-i18n]').forEach(function (element) {
      var text = strings[element.getAttribute('data-i18n')];
      if (text) element.innerHTML = text;
    });
    var title = strings[root.getAttribute('data-title')];
    if (title) document.title = title;
    var description = strings[root.getAttribute('data-description')];
    var meta = document.querySelector('meta[name="description"]');
    if (description && meta) meta.setAttribute('content', description);
  }

  function languageList(current) {
    var select = document.getElementById('lang');
    if (!select) return;
    LANGUAGES.forEach(function (entry) {
      var option = document.createElement('option');
      option.value = entry[0];
      option.textContent = entry[1];
      option.lang = entry[0];
      option.selected = entry[0] === current;
      select.appendChild(option);
    });
    select.hidden = false;
    select.addEventListener('change', function () {
      location.href = withLang(location.pathname + location.hash, select.value);
    });
  }

  function copyButton(words) {
    var button = document.getElementById('copy-agent-text');
    if (!button) return;
    button.addEventListener('click', function () {
      var box = document.getElementById('agent-text');
      var status = document.getElementById('copy-status');
      var done = function () { status.textContent = words.copied; };
      var manual = function () {
        var range = document.createRange();
        range.selectNodeContents(box);
        var selection = getSelection();
        selection.removeAllRanges();
        selection.addRange(range);
        var copied = false;
        try { copied = document.execCommand('copy'); } catch (e) { copied = false; }
        status.textContent = copied ? words.copied : words.copyManually;
      };
      if (navigator.clipboard && navigator.clipboard.writeText) navigator.clipboard.writeText(box.textContent).then(done, manual);
      else manual();
    });
  }

  var language = chosen();
  languageList(language.code);
  if (language.explicit) {
    // Links within the site keep the chosen language.
    document.querySelectorAll('a[href^="/"]').forEach(function (link) {
      link.setAttribute('href', withLang(link.getAttribute('href'), language.code));
    });
  }
  if (language.code === 'en') {
    copyButton(english);
    return;
  }
  fetch('/i18n/' + language.code + '.json')
    .then(function (response) { if (!response.ok) throw new Error(response.status); return response.json(); })
    .then(function (table) { apply(table.strings, language.code); copyButton(table.strings); })
    .catch(function () { copyButton(english); });
})();
