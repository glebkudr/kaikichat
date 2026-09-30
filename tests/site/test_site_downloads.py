"""kaikichat.com offers the desktop app for the visitor's own system.

The hero's download buttons and the tiles under "Get the app" link the same
published builds of one release; a script in the page's head marks the
visitor's system (macOS or Linux) on <html data-os>, and style.css then shows
only that system's button. Any other system, or no script, gets both.
"""
import json
import os
import re
import shutil
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SITE = ROOT / "deploy" / "site"
RELEASE = re.compile(
    r'href="(https://github\.com/glebkudr/kaikichat-releases/releases/download/v([0-9.]+)/([^"]+))"'
)
SYSTEM_SCRIPT = re.compile(r'<script id="system">(.*?)</script>', re.S)

# What real browsers report, and the button each should get.
NAVIGATORS = [
    ("Safari on a Mac", "mac", {
        "platform": "MacIntel", "maxTouchPoints": 0,
        "userAgent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15",
    }),
    ("Chrome on a Mac", "mac", {
        "userAgentData": {"platform": "macOS"}, "platform": "MacIntel", "maxTouchPoints": 0,
        "userAgent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36",
    }),
    ("Firefox on Linux x86_64", "linux", {
        "platform": "Linux x86_64", "maxTouchPoints": 0,
        "userAgent": "Mozilla/5.0 (X11; Linux x86_64; rv:143.0) Gecko/20100101 Firefox/143.0",
    }),
    ("Chrome on Linux x86_64", "linux", {
        "userAgentData": {"platform": "Linux"}, "platform": "Linux x86_64", "maxTouchPoints": 0,
        "userAgent": "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36",
    }),
    ("Chrome on Linux arm64, whose user agent still says x86_64", "", {
        "userAgentData": {"platform": "Linux"}, "platform": "Linux aarch64", "maxTouchPoints": 0,
        "userAgent": "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36",
    }),
    ("an iPad asking for the desktop site", "", {
        "platform": "MacIntel", "maxTouchPoints": 5,
        "userAgent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15",
    }),
    ("an iPhone", "", {
        "platform": "iPhone", "maxTouchPoints": 5,
        "userAgent": "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1",
    }),
    ("Chrome on Android", "", {
        "userAgentData": {"platform": "Android"}, "platform": "Linux armv8l", "maxTouchPoints": 5,
        "userAgent": "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Mobile Safari/537.36",
    }),
    ("Firefox on Android", "", {
        "platform": "Linux aarch64", "maxTouchPoints": 5,
        "userAgent": "Mozilla/5.0 (Android 15; Mobile; rv:143.0) Gecko/143.0 Firefox/143.0",
    }),
    ("a Chromebook", "", {
        "userAgentData": {"platform": "Chrome OS"}, "platform": "Linux x86_64", "maxTouchPoints": 0,
        "userAgent": "Mozilla/5.0 (X11; CrOS x86_64 14541.0.0) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36",
    }),
    ("Windows", "", {
        "userAgentData": {"platform": "Windows"}, "platform": "Win32", "maxTouchPoints": 0,
        "userAgent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36",
    }),
]

# Runs the head script once per navigator and prints the data-os each got.
RUNNER = """
const { script, navigators } = JSON.parse(require('fs').readFileSync(0, 'utf8'));
const seen = navigators.map((navigator) => {
  const attributes = {};
  const documentElement = { setAttribute: (name, value) => { attributes[name] = String(value); } };
  new Function('document', 'navigator', script)({ documentElement }, navigator);
  return attributes['data-os'] || '';
});
process.stdout.write(JSON.stringify(seen));
"""


def element(html, css_class):
    found = re.findall(rf'<a class="{re.escape(css_class)}"[^>]*>', html)
    assert len(found) == 1, f"{css_class}: {len(found)} elements"
    return found[0]


def href(tag):
    return re.search(r'href="([^"]+)"', tag).group(1)


class SiteDownloads(unittest.TestCase):
    def setUp(self):
        self.html = (SITE / "index.html").read_text()

    def test_every_download_names_one_release(self):
        links = RELEASE.findall(self.html)
        self.assertEqual(len(links), 4, "two hero buttons and two tiles")
        self.assertEqual(len({version for _, version, _ in links}), 1, links)
        version = links[0][1]
        self.assertEqual(
            sorted({name for _, _, name in links}),
            sorted([f"kaiki-chat_{version}_amd64.deb", "kaiki-chat-macos-arm64.zip"]),
        )

    def test_the_hero_buttons_download_what_the_tiles_do(self):
        tiles = re.search(r'<section class="part" id="get">(.*?)</section>', self.html, re.S).group(1)
        tile = {name: url for url, _, name in RELEASE.findall(tiles)}
        mac = href(element(self.html, "button download mac"))
        linux = href(element(self.html, "button download linux"))
        self.assertEqual(mac, tile["kaiki-chat-macos-arm64.zip"])
        self.assertTrue(linux.endswith("_amd64.deb"), linux)
        self.assertEqual(linux, tile[linux.rsplit("/", 1)[1]])

    def test_each_system_gets_its_own_button_and_others_get_both(self):
        head = self.html.split("</head>", 1)[0]
        script = SYSTEM_SCRIPT.search(head)
        self.assertIsNotNone(script, "the head marks the visitor's system before the page is drawn")
        node = os.environ.get("AIN_NODE") or shutil.which("node")
        self.assertIsNotNone(node, "node runs the head script (AIN_NODE or PATH)")
        run = subprocess.run(
            [node, "-e", RUNNER],
            input=json.dumps({"script": script.group(1), "navigators": [nav for _, _, nav in NAVIGATORS]}),
            capture_output=True, text=True, check=True, timeout=30,
        )
        seen = dict(zip([name for name, _, _ in NAVIGATORS], json.loads(run.stdout)))
        self.assertEqual(seen, {name: expected for name, expected, _ in NAVIGATORS})

    def test_the_stylesheet_hides_the_other_systems_button(self):
        css = " ".join((SITE / "style.css").read_text().split())
        self.assertRegex(css, r'html\[data-os="mac"\] \.download\.linux[^{]*\{ display: none; \}')
        self.assertRegex(css, r'html\[data-os="linux"\] \.download\.mac[^{]*\{ display: none; \}')


if __name__ == "__main__":
    unittest.main()
