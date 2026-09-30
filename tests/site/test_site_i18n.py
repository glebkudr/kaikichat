"""kaikichat.com speaks every language the app speaks.

The pages are written in English; deploy/site/i18n/<code>.json holds each
language's texts, deploy/site/site.js picks one. These tests keep the site's
languages equal to the app's, every table complete and really translated
(the same rules as apps/desktop/tests/i18n.test.tsx: a text equal to English,
or made only of English words, is a stand-in unless the language keeps it on
purpose), and the markup and the agent's commands intact.
"""
import json
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SITE = ROOT / "deploy" / "site"
APP_I18N = ROOT / "apps" / "desktop" / "src" / "i18n"

# Elements with data-i18n and their inner HTML, as the pages are written.
ELEMENT = re.compile(
    r'<(?P<tag>[a-z0-9]+)\b[^>]*?\bdata-i18n="(?P<key>[^"]+)"[^>]*>(?P<inner>.*?)</(?P=tag)>', re.S
)
TAG = re.compile(r"<[^>]+>")
SCRIPTS = {
    "ja": r"[぀-ヿ一-鿿]",
    "zh": r"[一-鿿]",
    "ko": r"[가-힯]",
    "fa": r"[؀-ۿ]",
    "ar": r"[؀-ۿ]",
    "ru": r"[Ѐ-ӿ]",
    "uk": r"[Ѐ-ӿ]",
}
CYRILLIC = re.compile(r"[А-Яа-яЁёІіЇїЄєҐґ]")
# Texts a language keeps as in English on purpose: the brand, and words it
# borrows as they are. Anything else that reads as English is a stand-in.
SAME_AS_ENGLISH = {
    "*": set(),
    "de": {"agent"},
    "fr": {"agent", "privacyMessagesTitle"},
    "nl": {"agent"},
    "pl": {"agent"},
    "cs": {"agent"},
    "hu": set(),
    "id": set(),
    "it": set(),
    "es": set(),
    "pt": set(),
    "tr": set(),
    "vi": set(),
}
# The commands the agent runs stay as they are in every language.
COMMANDS = [
    "https://kaikichat.com",
    "curl -fsSL https://kaikichat.com/install.sh | sh",
    "kaiki skill show",
    "kaiki skill install",
    "kaiki skill install --dir ~/.codex/skills",
    'kaiki init --name "',
    "kaiki coins claim",
    "kaiki coins balance",
    "kaiki network",
    "AGENTIC_PASSWORD_FILE",
    "Claude Code",
    "Codex",
]


def page_strings(name):
    text = (SITE / name).read_text()
    found = {}
    for match in ELEMENT.finditer(text):
        key, inner = match["key"], match["inner"]
        if key != "agentText":
            inner = " ".join(inner.split())
        assert found.get(key, inner) == inner, f"{name}: {key} differs between its elements"
        found[key] = inner
    return found, text


def table(code):
    return json.loads((SITE / "i18n" / f"{code}.json").read_text())


def app_languages():
    index = (APP_I18N / "index.tsx").read_text()
    codes = re.search(r"export const locales=\{([^}]*)\}", index).group(1).split(",")
    codes = [code.strip() for code in codes]
    names = {}
    for code in codes:
        source = (APP_I18N / f"{code}.ts").read_text()
        names[code] = re.search(r"language: \{label: '[^']*', name: '([^']*)'", source).group(1)
    return codes, names



def plain(text):
    text = TAG.sub(" ", text).lower()
    return " ".join(re.findall(r"[^\W\d_]+", text))


def words(text):
    found = re.findall(r"[^\W\d_]+", TAG.sub(" ", text))
    kept = []
    for word in found:
        if word.isupper():
            continue  # an acronym such as CLI or MCP
        if len(word) < 3 and re.fullmatch(r"[A-Za-z]+", word):
            continue  # "de" or "in" are not English only
        kept.append(word.lower())
    return kept


def markup(text):
    return sorted(TAG.findall(text))


class SiteLanguages(unittest.TestCase):
    def setUp(self):
        self.index, self.index_html = page_strings("index.html")
        self.privacy, self.privacy_html = page_strings("privacy.html")
        self.codes, self.names = app_languages()
        self.english = table("en")["strings"]

    def test_the_site_offers_the_languages_of_the_app_each_named_in_itself(self):
        files = sorted(path.stem for path in (SITE / "i18n").glob("*.json"))
        self.assertEqual(files, sorted(self.codes))
        site_js = (SITE / "site.js").read_text()
        listed = re.findall(r"\['([a-z]{2})', '([^']+)'\]", site_js)
        self.assertEqual([code for code, _ in listed], self.codes)
        self.assertEqual(dict(listed), self.names)
        for code in self.codes:
            language = table(code)["language"]
            self.assertEqual(language["code"], code)
            self.assertEqual(language["name"], self.names[code])
            self.assertEqual(language["dir"], "rtl" if code in ("fa", "ar") else "ltr", code)

    def test_the_english_table_is_the_text_the_pages_are_written_in(self):
        for key, text in {**self.index, **self.privacy}.items():
            self.assertEqual(self.english.get(key), text, key)
        titles = {
            "pageTitle": re.search(r"<title>(.*?)</title>", self.index_html).group(1),
            "privacyPageTitle": re.search(r"<title>(.*?)</title>", self.privacy_html).group(1),
            "pageDescription": re.search(r'<meta name="description" content="([^"]*)"', self.index_html).group(1),
            "privacyPageDescription": re.search(
                r'<meta name="description" content="([^"]*)"', self.privacy_html
            ).group(1),
        }
        for key, text in titles.items():
            self.assertEqual(self.english.get(key), text, key)
        used = set(self.index) | set(self.privacy) | set(titles) | {"copied", "copyManually"}
        self.assertEqual(sorted(set(self.english) - used), [], "texts no page shows")

    def test_every_language_has_every_text_and_none_empty(self):
        for code in self.codes:
            strings = table(code)["strings"]
            self.assertEqual(
                {"missing": sorted(set(self.english) - set(strings)), "extra": sorted(set(strings) - set(self.english))},
                {"missing": [], "extra": []},
                code,
            )
            self.assertEqual([key for key, text in strings.items() if not text.strip()], [], code)
            if code not in ("ru", "uk"):
                self.assertEqual([key for key, text in strings.items() if CYRILLIC.search(text)], [], code)

    def test_every_language_translates_for_real(self):
        vocabulary = {word for text in self.english.values() for word in words(text)}
        stand_ins, translated_after_all = {}, {}
        for code in self.codes:
            if code == "en":
                continue
            kept = SAME_AS_ENGLISH["*"] | SAME_AS_ENGLISH.get(code, set())
            script = SCRIPTS.get(code)
            copied = []
            for key, text in table(code)["strings"].items():
                same = plain(text) == plain(self.english[key])
                english_only = len(words(text)) >= 2 and all(word in vocabulary for word in words(text))
                foreign_script = script is not None and not re.search(script, TAG.sub(" ", text))
                if same or english_only or foreign_script:
                    copied.append(key)
            unexpected = [f"{key}: {table(code)['strings'][key]}" for key in copied if key not in kept]
            stale = sorted(kept - set(copied))
            if unexpected:
                stand_ins[code] = unexpected
            if stale:
                translated_after_all[code] = stale
        self.assertEqual({"standIns": stand_ins, "translatedAfterAll": translated_after_all},
                         {"standIns": {}, "translatedAfterAll": {}})

    def test_translations_keep_the_markup_and_the_agents_commands(self):
        for code in self.codes:
            strings = table(code)["strings"]
            for key, text in self.english.items():
                self.assertEqual(markup(strings[key]), markup(text), f"{code} {key}")
            agent_text = strings["agentText"]
            self.assertEqual([command for command in COMMANDS if command not in agent_text], [], code)
            steps = re.findall(r"^\s*([1-5])\. ", agent_text, re.M)
            self.assertEqual(steps, ["1", "2", "3", "4", "5"], code)



if __name__ == "__main__":
    unittest.main()
